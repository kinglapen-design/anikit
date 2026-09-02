//! CEP 导出接收入库（8.1，PRD 07-03 实施定案）：
//! 插件导出产物（fla + PNG 序列 / 静图 PNG）→ 合成 WebP（动态 = ffmpeg 降级链压 2M；静态 =
//! image crate 转换）→ 正常导入管线（Source::Move 同盘剪切）→ hidden 绑定（PRD 02-15）→ 清场。
//! 产物落点 = 库根 temp/cep-out/（temp ≠ cache：中转产物用完即删，2026-09-01 用户定）。
//!
//! 三段式拆分（prepare / compose / commit）：ffmpeg 合成是秒级慢 IO，**锁外执行**——
//! server 任务先短锁 prepare 取路径，锁外 compose，再锁 commit 导入（合成期间不冻结命令层）。

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::core::library::{Library, TEMP_DIR};
use crate::core::models::new_asset_id;
use crate::core::paths;
use crate::services::anim;
use crate::services::asset_menu;
use crate::services::import::{self as ops, PendingFile, Source};
use crate::services::operations;
use crate::services::thumbnail;

/// 动态降级链（质量, 长边系数）：长边档 = maxEdge × 系数（1024 基准则 768/512）；
/// 全过仍超 → 抽帧末段，再超保留产出（软上限口径）
const FALLBACK: &[(u32, f64)] = &[(80, 1.0), (60, 1.0), (40, 1.0), (60, 0.75), (60, 0.5)];

/// 插件 exported 通知载荷（WS 协议，server 层 serde 解析）
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPayload {
  /// 命名（面板输入框内容，默认 = 元件名）：fla / webp 同名词干
  pub name: String,
  /// 目标文件夹（None / 失效 = root 未分类；树选择器随 2b 接入）
  #[serde(default)]
  pub folder_id: Option<String>,
  pub dynamic: bool,
  pub fla_path: String,
  /// 动态 = PNG 序列目录（f_0001.png 起）；静态 = 单帧 PNG 路径
  #[serde(default)]
  pub seq_dir: Option<String>,
  #[serde(default)]
  pub png_path: Option<String>,
  /// 文档帧率（动态合成节奏；缺省 24）
  #[serde(default = "default_fps")]
  pub fps: f64,
}

fn default_fps() -> f64 {
  24.0
}

/// 读 PNG 序列目录（f_0001.png 起按文件名排序）→ 逐帧解码 + 长边缩放（0 = 不缩放）→ 帧列表
fn load_seq_frames(seq: &Path, fps: f64, max_edge: u32, step: u32) -> Option<Vec<anim::AnimFrame>> {
  let mut files: Vec<PathBuf> = fs::read_dir(seq)
    .ok()?
    .filter_map(|e| e.ok())
    .map(|e| e.path())
    .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("png"))
    .collect();
  files.sort();
  if files.is_empty() {
    return None;
  }
  let step = step.max(1) as usize;
  let delay_ms = (1000.0 / fps * step as f64).round().max(1.0) as u32; // 抽帧补时长：delay × step
  let mut frames = Vec::new();
  for path in files.into_iter().step_by(step) {
    let img = thumbnail::decode_by_signature(&path)?;
    let img = if max_edge > 0 { thumbnail::shrink_long_edge(img, max_edge) } else { img };
    frames.push(anim::AnimFrame { rgba: img.to_rgba8(), delay_ms });
  }
  Some(frames)
}

/// 入库结果（WS importDone 回执载荷）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOutcome {
  pub fla_asset_id: String,
  pub webp_asset_id: String,
  /// true = 降级链压不进 2M 的保留产出（PRD 软上限：压不进也留产出并提示）
  pub over_limit: bool,
  pub webp_bytes: u64,
}

/// prepare 产物（锁外 compose 所需全部输入；不持库引用）
#[derive(Debug)]
pub struct Prepared {
  payload: ExportPayload,
  cep_out: PathBuf,
  fla_src: PathBuf,
  webp_tmp: PathBuf,
  folder: String,
  /// 导出设置快照（settings.cepExport 子树；prepare 时读取，compose 锁外沿用）
  settings: crate::core::config::CepExportSettings,
}

/// 阶段①（短锁）：产物校验 + 路径推导 + 目标文件夹解析（失效 id 回退 root + warn）
pub fn prepare(lib: &Library, payload: &ExportPayload) -> Result<Prepared, String> {
  let fla_src = PathBuf::from(&payload.fla_path);
  if !fla_src.is_file() {
    return Err(format!("fla 产物不存在：{}", payload.fla_path));
  }
  if payload.name.trim().is_empty() {
    return Err("名称为空".to_string());
  }
  if payload.dynamic {
    let seq = payload.seq_dir.as_deref().unwrap_or("");
    if seq.is_empty() || !Path::new(seq).is_dir() {
      return Err(format!("序列帧目录不存在：{seq}"));
    }
  } else {
    let png = payload.png_path.as_deref().unwrap_or("");
    if png.is_empty() || !Path::new(png).is_file() {
      return Err(format!("静图 PNG 不存在：{png}"));
    }
  }
  let cep_out = lib.root.join(TEMP_DIR).join("cep-out");
  fs::create_dir_all(&cep_out).map_err(|e| format!("cep-out 目录创建失败：{e}"))?;
  let folder = match &payload.folder_id {
    Some(id) if lib.index.folders.contains_key(id.as_str()) => id.clone(),
    Some(id) => {
      log::warn!("CEP 导出目标文件夹失效（{id}），回退未分类");
      "root".to_string()
    }
    None => "root".to_string(),
  };
  let webp_tmp = cep_out.join(format!("{}.webp", payload.name.trim()));
  // 设置快照（读盘失败回退默认档——设置损坏不阻断导出）
  let settings = crate::core::config::cep_export_settings().unwrap_or_default();
  Ok(Prepared { payload: payload.clone(), cep_out, fla_src, webp_tmp, folder, settings })
}

/// 阶段②（锁外慢 IO）：合成 WebP——动态 = 降级链压上限，静态 = image crate 转换；
/// 返回 (over_limit, webp_bytes)
pub fn compose(p: &Prepared) -> Result<(bool, u64), String> {
  compose_webp(p.payload.dynamic, p.payload.seq_dir.as_deref(), p.payload.png_path.as_deref(), p.payload.fps, &p.webp_tmp, &p.settings)
}

/// 合成共用核心（批次 2 仅缩略图同用）：动态读序列目录降级链、静态读单帧 PNG 转 WebP 落 out
fn compose_webp(
  dynamic: bool,
  seq_dir: Option<&str>,
  png_path: Option<&str>,
  fps: f64,
  out: &Path,
  settings: &crate::core::config::CepExportSettings,
) -> Result<(bool, u64), String> {
  let over_limit = if dynamic {
    let seq = Path::new(seq_dir.unwrap_or(""));
    compose_with_fallback(seq, fps, out, settings)?
  } else {
    let png = Path::new(png_path.unwrap_or(""));
    let img = thumbnail::decode_by_signature(png).ok_or_else(|| "PNG 解码失败".to_string())?;
    let img = thumbnail::shrink_long_edge(img, settings.max_edge);
    thumbnail::save_webp(&img, out).map_err(|e| format!("WebP 编码失败：{e}"))?;
    false
  };
  let bytes = fs::metadata(out).map(|m| m.len()).unwrap_or(0);
  Ok((over_limit, bytes))
}

/// 动态合成降级链：FALLBACK 逐级（长边档 = maxEdge × 系数；系数 < 1 档从基底内存再缩，不重读 PNG）→
/// 末段抽帧降帧率（隔帧取 + delay×2，时长不变）→ 压不进保留最后一版（over_limit = true）。
/// 2026-09-02 提速（用户实机报「8M 导出卡 10 秒」）：大小判断改内存（data.len()）过了才写盘
///（旧版每档写盘再 stat——temp 里文件早现≠完成）；首档超上限 >5 倍直接跳中间档
///（8M/2M=4x 级别时同尺寸低质量档压进上限概率极低，直奔 0.5 尺寸档 + 抽帧档）。
fn compose_with_fallback(seq: &Path, fps: f64, out: &Path, settings: &crate::core::config::CepExportSettings) -> Result<bool, String> {
  let max_edge = settings.max_edge.max(16);
  let Some(base) = load_seq_frames(seq, fps, max_edge, 1) else {
    return Err("序列帧解码失败（PNG 损坏或目录为空）".to_string());
  };
  let mut first_over_ratio: Option<u64> = None; // 首档超出倍数（跳档依据）
  for &(q, ratio) in FALLBACK {
    // 跳档：首档超 >5 倍时，其余 1.0 系数档与 0.75 档直接跳过
    if let Some(r) = first_over_ratio {
      if r > 5 && ratio >= 0.75 {
        continue;
      }
    }
    let edge = ((max_edge as f64 * ratio).round() as u32).max(16);
    let owned;
    let frames: &[anim::AnimFrame] = if ratio >= 1.0 {
      &base
    } else {
      owned = rescale_frames(&base, edge);
      &owned
    };
    if let Some(data) = anim::encode_webp_anim(frames, q as f32) {
      let len = data.len() as u64;
      if first_over_ratio.is_none() && len > settings.max_bytes {
        first_over_ratio = Some(len / settings.max_bytes.max(1));
      }
      if len <= settings.max_bytes {
        fs::write(out, &data).map_err(|e| format!("WebP 写入失败：{e}"))?;
        return Ok(false);
      }
    }
  }
  // 末段：抽帧降帧率（时长不变）
  let halved: Vec<anim::AnimFrame> = base.iter().step_by(2).map(|f| anim::AnimFrame { rgba: f.rgba.clone(), delay_ms: f.delay_ms * 2 }).collect();
  if !halved.is_empty() {
    if let Some(data) = anim::encode_webp_anim(&halved, 60.0) {
      // 过限写盘返回；压不进也写末版保底（over_limit 产出——旧版靠每档写盘副作用留底，现显式写）
      fs::write(out, &data).map_err(|e| format!("WebP 写入失败：{e}"))?;
      if (data.len() as u64) <= settings.max_bytes {
        return Ok(false);
      }
    }
  }
  if !out.is_file() {
    return Err("WebP 合成失败（编码异常）".to_string());
  }
  log::warn!("WebP 降级链压不进上限，保留产出（{}）", out.display());
  Ok(true)
}

/// 帧列表内存再缩放（降级链小档；base 已是 1024 档，不重读 PNG）
fn rescale_frames(frames: &[anim::AnimFrame], max_edge: u32) -> Vec<anim::AnimFrame> {
  frames
    .iter()
    .map(|f| {
      let img = image::DynamicImage::ImageRgba8(f.rgba.clone());
      anim::AnimFrame { rgba: thumbnail::shrink_long_edge(img, max_edge).to_rgba8(), delay_ms: f.delay_ms }
    })
    .collect()
}

fn within_limit(out: &Path, max_bytes: u64) -> bool {
  fs::metadata(out).map(|m| m.len() <= max_bytes).unwrap_or(false)
}

/// 阶段③（锁内）：导入两资产（Move 同盘剪切）→ hidden 绑定 → folders 落盘 → 清场
pub fn commit(lib: &mut Library, p: &Prepared, over_limit: bool, webp_bytes: u64) -> Result<ImportOutcome, String> {
  let fla_id = new_asset_id();
  let webp_id = new_asset_id();
  let result = import_pair(lib, p, &fla_id, &webp_id);
  if let Err(e) = &result {
    // 半截清理：失败不留孤儿 info（operations/document.rs 同款口径）
    let _ = fs::remove_dir_all(lib.info_dir(&fla_id));
    let _ = fs::remove_dir_all(lib.info_dir(&webp_id));
    return Err(e.clone());
  }
  asset_menu::set_thumb_asset(lib, &fla_id, &webp_id)?;
  operations::finish(lib)?;
  // 伴生 webp 提交小动画生成（动态 = 卡片播放源 info/{stem}_anim.webp；静态伴生 is_anim 判定跳过）
  crate::services::anim_preview_queue::global().submit(crate::services::anim_preview_queue::AnimPreviewTask {
    assets_dir: lib.assets_dir.clone(),
    asset_id: webp_id.clone(),
  });
  // 清场：序列帧目录与静图 PNG（fla / webp 本体已 Move 进库）
  if let Some(seq) = &p.payload.seq_dir {
    let _ = fs::remove_dir_all(seq);
  }
  if let Some(png) = &p.payload.png_path {
    let _ = fs::remove_file(png);
  }
  let _ = &p.cep_out; // 目录保留（下一单复用）
  Ok(ImportOutcome { fla_asset_id: fla_id, webp_asset_id: webp_id, over_limit, webp_bytes })
}

/// 两资产入库：build_asset（探测 + 缩略图 + metadata 落盘）→ 索引 → placement（同名单一化）
fn import_pair(lib: &mut Library, p: &Prepared, fla_id: &str, webp_id: &str) -> Result<(), String> {
  let name = p.payload.name.trim();
  let mut anchor: HashMap<String, usize> = HashMap::new();
  for (src, id, file_name) in [
    (p.fla_src.clone(), fla_id, format!("{name}.fla")),
    (p.webp_tmp.clone(), webp_id, format!("{name}.webp")),
  ] {
    let pf = PendingFile { path: src.clone(), chain: Vec::new() };
    let hash = ops::sha256_file(&src).map_err(|e| format!("读取产物失败（{}）：{e}", src.display()))?;
    let meta = ops::build_asset_with_id(&lib.assets_dir, &pf, &hash, id, Source::Move, None)?;
    lib.index.assets.insert(id.to_string(), meta);
    ops::add_placement(lib, id.to_string(), p.folder.clone(), file_name, true, &mut anchor);
  }
  Ok(())
}

// —— 批次 2（2026-09-02）：仅缩略图（不导新 fla）——
// 场景：工程文件本身已是整理好的独立 fla，只差缩略图（PRD 07-03「导出缩略图」手动兜底条）——
// 面板对当前文档直接导画面（静态 = 播放头帧；动态 = 主时间轴帧范围），PNG/序列照常落 temp/cep-out
// 由 APP 合成 WebP（合成能力只在 APP）。绑定目标 = 文档磁盘路径 ↔ 库内 fla 资产池内实体路径精确匹配：
//   命中 = WebP 入库（placement 落 root）+ set_thumb_asset 绑定（已有绑定 = 换绑，旧 hidden 不动——PRD 口径）；
//   未命中 = 不入库，WebP 挪到原 fla 旁边（{词干}.webp，重名 -序号唯一化），用户自行导入（用户定）。

/// 仅缩略图载荷（WS thumbOnly 消息）
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThumbPayload {
  /// 当前文档 fla 磁盘绝对路径（库内匹配键 + 池外兜底落点目录）
  pub doc_path: String,
  pub dynamic: bool,
  /// 动态 = PNG 序列目录；静态 = 单帧 PNG 路径（同 ExportPayload 口径）
  #[serde(default)]
  pub seq_dir: Option<String>,
  #[serde(default)]
  pub png_path: Option<String>,
  #[serde(default = "default_fps")]
  pub fps: f64,
}

/// 仅缩略图 prepare 产物
#[derive(Debug)]
pub struct ThumbPrepared {
  payload: ThumbPayload,
  /// 库内命中资产 id（None = 池外兜底）
  matched: Option<String>,
  /// 文档词干（webp 命名 + placement 显示名）
  stem: String,
  webp_tmp: PathBuf,
  settings: crate::core::config::CepExportSettings,
}

/// 仅缩略图结果（WS thumbDone 回执载荷）：Bound = 入库绑定；Beside = 落原文件旁边
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "mode")]
pub enum ThumbOutcome {
  #[serde(rename_all = "camelCase")]
  Bound { asset_id: String, webp_asset_id: String, name: String, over_limit: bool, webp_bytes: u64 },
  #[serde(rename_all = "camelCase")]
  Beside { out_path: String, webp_bytes: u64 },
}

/// 路径规范化（Windows 大小写不敏感）：统一正斜杠 + 小写 + 去尾斜杠
fn normalize_path(p: &str) -> String {
  p.replace('\\', "/").trim_end_matches('/').to_lowercase()
}

/// 库内 fla 资产匹配：池内实体路径（assets/{ID}.info/{originName}）== 文档路径（精确，不模糊猜）
pub fn match_fla_asset(lib: &Library, doc_path: &str) -> Option<String> {
  let target = normalize_path(doc_path);
  lib.index.assets.values().find_map(|m| {
    if m.ext != "fla" {
      return None;
    }
    let p = paths::origin_file(&lib.assets_dir, &m.id, &m.origin_name);
    (normalize_path(&p.to_string_lossy()) == target).then(|| m.id.clone())
  })
}

/// 阶段①（短锁）：产物校验 + 词干解析 + 库内匹配
pub fn thumb_prepare(lib: &Library, payload: &ThumbPayload) -> Result<ThumbPrepared, String> {
  if payload.dynamic {
    let seq = payload.seq_dir.as_deref().unwrap_or("");
    if seq.is_empty() || !Path::new(seq).is_dir() {
      return Err(format!("序列帧目录不存在：{seq}"));
    }
  } else {
    let png = payload.png_path.as_deref().unwrap_or("");
    if png.is_empty() || !Path::new(png).is_file() {
      return Err(format!("静图 PNG 不存在：{png}"));
    }
  }
  let stem = Path::new(&payload.doc_path)
    .file_stem()
    .and_then(|s| s.to_str())
    .map(|s| s.trim().to_string())
    .filter(|s| !s.is_empty())
    .ok_or_else(|| format!("文档路径无效：{}", payload.doc_path))?;
  let matched = match_fla_asset(lib, &payload.doc_path);
  let cep_out = lib.root.join(TEMP_DIR).join("cep-out");
  fs::create_dir_all(&cep_out).map_err(|e| format!("cep-out 目录创建失败：{e}"))?;
  let webp_tmp = cep_out.join(format!("{stem}.webp"));
  let settings = crate::core::config::cep_export_settings().unwrap_or_default();
  Ok(ThumbPrepared { payload: payload.clone(), matched, stem, webp_tmp, settings })
}

/// 阶段②（锁外慢 IO）：与元件导出同一合成核心
pub fn thumb_compose(p: &ThumbPrepared) -> Result<(bool, u64), String> {
  compose_webp(p.payload.dynamic, p.payload.seq_dir.as_deref(), p.payload.png_path.as_deref(), p.payload.fps, &p.webp_tmp, &p.settings)
}

/// 阶段③（锁内）：命中 = WebP 入库 + hidden 绑定（换绑不动旧 hidden）；未命中 = 挪到原 fla 旁边
pub fn thumb_commit(lib: &mut Library, p: &ThumbPrepared, over_limit: bool, webp_bytes: u64) -> Result<ThumbOutcome, String> {
  // 清场（序列目录 / 静图 PNG；webp_tmp 由两条路径各自消费）
  let cleanup = |p: &ThumbPrepared| {
    if let Some(seq) = &p.payload.seq_dir {
      let _ = fs::remove_dir_all(seq);
    }
    if let Some(png) = &p.payload.png_path {
      let _ = fs::remove_file(png);
    }
  };
  match &p.matched {
    Some(asset_id) => {
      let webp_id = new_asset_id();
      let r = (|| {
        let pf = PendingFile { path: p.webp_tmp.clone(), chain: Vec::new() };
        let hash = ops::sha256_file(&p.webp_tmp).map_err(|e| format!("读取产物失败：{e}"))?;
        let meta = ops::build_asset_with_id(&lib.assets_dir, &pf, &hash, &webp_id, Source::Move, None)?;
        lib.index.assets.insert(webp_id.clone(), meta);
        let mut anchor: HashMap<String, usize> = HashMap::new();
        ops::add_placement(lib, webp_id.clone(), "root".to_string(), format!("{}.webp", p.stem), true, &mut anchor);
        asset_menu::set_thumb_asset(lib, asset_id, &webp_id)
      })();
      if let Err(e) = r {
        let _ = fs::remove_dir_all(lib.info_dir(&webp_id)); // 半截清理（同 commit 口径）
        return Err(e);
      }
      operations::finish(lib)?;
      crate::services::anim_preview_queue::global().submit(crate::services::anim_preview_queue::AnimPreviewTask {
        assets_dir: lib.assets_dir.clone(),
        asset_id: webp_id.clone(),
      });
      let name = lib.index.assets.get(asset_id).map(|m| m.name.clone()).unwrap_or_default();
      cleanup(p);
      Ok(ThumbOutcome::Bound { asset_id: asset_id.clone(), webp_asset_id: webp_id, name, over_limit, webp_bytes })
    }
    None => {
      let dir = Path::new(&p.payload.doc_path).parent().ok_or_else(|| "文档路径无效".to_string())?;
      let target = unique_beside(dir, &p.stem, "webp");
      fs::rename(&p.webp_tmp, &target).map_err(|e| format!("挪至原文件旁边失败（{}）：{e}", target.display()))?;
      cleanup(p);
      Ok(ThumbOutcome::Beside { out_path: target.to_string_lossy().into_owned(), webp_bytes })
    }
  }
}

/// 池外兜底落点唯一化：{stem}.webp 存在则 {stem}-2.webp 递增
fn unique_beside(dir: &Path, stem: &str, ext: &str) -> PathBuf {
  let mut c = dir.join(format!("{stem}.{ext}"));
  let mut n = 2;
  while c.exists() {
    c = dir.join(format!("{stem}-{n}.{ext}"));
    n += 1;
  }
  c
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::library;
  use crate::core::json_io::read_json;
  use crate::core::models::AssetMeta;

  fn make_lib() -> (tempfile::TempDir, Library) {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    (dir, lib)
  }

  fn payload(dir: &Path, name: &str, dynamic: bool) -> ExportPayload {
    ExportPayload {
      name: name.to_string(),
      folder_id: None,
      dynamic,
      fla_path: dir.join(format!("{name}.fla")).to_string_lossy().into_owned(),
      seq_dir: None,
      png_path: Some(dir.join(format!("{name}.png")).to_string_lossy().into_owned()),
      fps: 25.0,
    }
  }

  /// 静态导出端到端（锁内 prepare + commit；compose 的 image 转换不依赖 sidecar）：
  /// fla + webp 入库 → hidden 绑定 → placement 落 root → 清场
  #[test]
  fn static_export_end_to_end() {
    let (dir, mut lib) = make_lib();
    let src_dir = dir.path().join("out");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("跑.fla"), b"fake-fla").unwrap();
    // 静图 PNG（真图——compose 要解码）
    image::RgbaImage::from_pixel(64, 32, image::Rgba([200, 30, 30, 255])).save(src_dir.join("跑.png")).unwrap();
    let p = payload(&src_dir, "跑", false);

    let prepared = prepare(&lib, &p).unwrap();
    let (over_limit, bytes) = compose(&prepared).unwrap();
    assert!(!over_limit && bytes > 0);
    let outcome = commit(&mut lib, &prepared, over_limit, bytes).unwrap();

    // 两资产入库 + placement 落 root
    assert!(lib.index.assets.contains_key(&outcome.fla_asset_id));
    assert!(lib.index.assets.contains_key(&outcome.webp_asset_id));
    assert!(lib.folders.items.iter().any(|x| x.asset == outcome.fla_asset_id && x.folder == "root"));
    // hidden 绑定：webp hidden + fla 引用（落盘一致）
    assert_eq!(lib.index.assets[&outcome.webp_asset_id].hidden, Some(true));
    assert_eq!(lib.index.assets[&outcome.fla_asset_id].thumb_asset_id.as_deref(), Some(outcome.webp_asset_id.as_str()));
    let disk: AssetMeta = read_json(&lib.info_dir(&outcome.fla_asset_id).join("metadata.json")).unwrap();
    assert_eq!(disk.thumb_asset_id.as_deref(), Some(outcome.webp_asset_id.as_str()));
    // Move 语义：源文件已入 info，cep 产物原位置已清
    assert!(!src_dir.join("跑.fla").exists(), "fla 已剪切入库");
    assert!(!prepared.webp_tmp.exists(), "webp 已剪切入库");
    assert!(!src_dir.join("跑.png").exists(), "静图 PNG 已清场");
    // webp 资产经动图探测（frame_count 有值 = animated 卡片可播）
    assert!(lib.index.assets[&outcome.webp_asset_id].frame_count.is_some() || lib.index.assets[&outcome.webp_asset_id].thumbnail.is_some());
  }

  /// 护栏：产物缺失 / 空名称 / 失效目标文件夹回退 root
  #[test]
  fn prepare_guards() {
    let (dir, lib) = make_lib();
    let src_dir = dir.path().join("out");
    fs::create_dir_all(&src_dir).unwrap();
    let p = payload(&src_dir, "缺", false);
    assert!(prepare(&lib, &p).unwrap_err().contains("fla 产物不存在"));
    fs::write(src_dir.join("缺.fla"), b"x").unwrap();
    assert!(prepare(&lib, &p).unwrap_err().contains("静图 PNG 不存在"));
    let mut p2 = payload(&src_dir, "  ", false);
    p2.png_path = p.png_path.clone();
    fs::write(src_dir.join("缺.png"), b"x").unwrap();
    p2.fla_path = p.fla_path.clone();
    assert!(prepare(&lib, &p2).unwrap_err().contains("名称为空"));
    let mut p3 = payload(&src_dir, "缺", false);
    p3.folder_id = Some("f-ghost".into());
    assert_eq!(prepare(&lib, &p3).unwrap().folder, "root", "失效文件夹回退未分类");
  }

  /// WebP 动画组装 = 全帧矩形 + blend=0 源覆盖（2026-09-01 实机踩坑：ffmpeg libwebp_anim
  /// 差分帧 + image-webp 画布合成缺陷 → 播放重影）；本测试 = 回归钉：组包后经 image-webp
  /// 解码路径（帧流 / 缩略图同路径）逐帧内容必须与输入一致（全异色块 = 差分极端反差）
  #[test]
  fn webp_anim_full_frames_decode_clean() {
    let dir = tempfile::tempdir().unwrap();
    let seq = dir.path().join("seq");
    fs::create_dir_all(&seq).unwrap();
    let colors = [[220, 40, 40, 255u8], [40, 200, 40, 255], [40, 40, 220, 255], [240, 240, 40, 255]];
    for (i, c) in colors.iter().enumerate() {
      image::RgbaImage::from_pixel(64, 48, image::Rgba(*c)).save(seq.join(format!("f_{:04}.png", i + 1))).unwrap();
    }
    let frames = load_seq_frames(&seq, 25.0, 0, 1).unwrap();
    assert_eq!(frames.len(), 4);
    assert_eq!(frames[0].delay_ms, 40, "1000/25 = 40ms");
    let data = anim::encode_webp_anim(&frames, 80.0).unwrap();
    let out = dir.path().join("anim.webp");
    fs::write(&out, &data).unwrap();
    // 动图探测：帧数 / 时长
    let info = crate::services::anim::probe(&out).expect("组装的 webp 应被识别为动图");
    assert_eq!(info.frame_count, 4);
    // 解码逐帧断言（image-webp 同播放路径）：内容 = 输入帧（有损容差 ±8），无叠加
    crate::services::anim::decode_frames(&out, |idx, img, _delay| {
      let px = img.get_pixel(10, 10);
      let c = colors[idx as usize];
      for ch in 0..3 {
        assert!((px[ch] as i16 - c[ch] as i16).abs() <= 8, "帧 {idx} 通道 {ch}：{:?} ≠ {:?}", px, c);
      }
      true
    })
    .expect("解码应成功");
  }

  /// 残影回归钉（2026-09-01 实机三连报）：flags bit1=0（alpha 混合）时透明区不覆盖画布 →
  /// 移动内容在透明区残留旧帧。组包恒 bit1=1（源覆盖含透明区）——透明底移动色块场景，
  /// 帧 2 在帧 1 色块位置必须恢复透明（无残留）
  #[test]
  fn webp_anim_no_ghost_with_translucent_move() {
    let dir = tempfile::tempdir().unwrap();
    let make = |left: bool| {
      let mut img = image::RgbaImage::from_pixel(64, 48, image::Rgba([0, 0, 0, 0])); // 全透明底
      let (x0, x1) = if left { (4, 24) } else { (40, 60) };
      for y in 8..40 {
        for x in x0..x1 {
          img.put_pixel(x, y, image::Rgba([220, 40, 40, 255]));
        }
      }
      anim::AnimFrame { rgba: img, delay_ms: 40 }
    };
    let frames = vec![make(true), make(false)];
    let out = dir.path().join("anim.webp");
    fs::write(&out, anim::encode_webp_anim(&frames, 80.0).unwrap()).unwrap();
    crate::services::anim::decode_frames(&out, |idx, img, _| {
      if idx == 1 {
        // 帧 2：帧 1 色块区（左）必须透明（无残留），帧 2 色块区（右）不透明
        assert!(img.get_pixel(10, 20)[3] < 10, "帧 2 左侧残留旧帧内容（alpha={}）", img.get_pixel(10, 20)[3]);
        assert!(img.get_pixel(50, 20)[3] > 245, "帧 2 右侧色块缺失");
      }
      true
    })
    .expect("解码应成功");
  }

  /// 降级链首档即过（小帧必然远低于 2M）；抽帧末段的 delay 翻倍语义
  #[test]
  fn compose_fallback_and_halve() {
    let dir = tempfile::tempdir().unwrap();
    let seq = dir.path().join("seq");
    fs::create_dir_all(&seq).unwrap();
    for i in 0..6u32 {
      image::RgbaImage::from_pixel(64, 48, image::Rgba([40 + (i * 30) as u8, 120, 200, 255]))
        .save(seq.join(format!("f_{:04}.png", i + 1)))
        .unwrap();
    }
    let out = dir.path().join("anim.webp");
    let settings = crate::core::config::CepExportSettings::default();
    assert_eq!(compose_with_fallback(&seq, 25.0, &out, &settings).unwrap(), false, "首档即过");
    assert!(within_limit(&out, settings.max_bytes));
    // 抽帧（step=2）：帧数减半 + delay 翻倍（时长不变）
    let frames = load_seq_frames(&seq, 25.0, 0, 2).unwrap();
    assert_eq!(frames.len(), 3);
    assert_eq!(frames[0].delay_ms, 80);
  }

  /// 黑底回归钉（2026-09-01 用户实机报）：含透明区的 webp 动画经缩略图管线 generate
  /// 必须保 alpha——image crate 对 VP8X 动画容器解码丢 alpha 落成黑底（不透明），
  /// 动图须走 anim 解码首帧（thumbnail::decode_origin_image 的选路语义）
  #[test]
  fn webp_anim_thumbnail_keeps_alpha() {
    let dir = tempfile::tempdir().unwrap();
    // 组 2 帧动画：全透明底 + 中心红色块
    let frames: Vec<anim::AnimFrame> = (0..2)
      .map(|_| {
        let mut img = image::RgbaImage::from_pixel(64, 48, image::Rgba([0, 0, 0, 0]));
        for y in 12..36 {
          for x in 16..48 {
            img.put_pixel(x, y, image::Rgba([220, 40, 40, 255]));
          }
        }
        anim::AnimFrame { rgba: img, delay_ms: 40 }
      })
      .collect();
    let webp_path = dir.path().join("anim.webp");
    fs::write(&webp_path, anim::encode_webp_anim(&frames, 80.0).unwrap()).unwrap();
    let info_dir = dir.path().join("info");
    fs::create_dir_all(&info_dir).unwrap();
    let thumb = thumbnail::generate(&info_dir, &webp_path, "anim").unwrap();
    let timg = thumbnail::decode_by_signature(&info_dir.join(&thumb.name)).unwrap().to_rgba8();
    assert!(timg.get_pixel(2, 2)[3] < 10, "角落透明区 alpha 应≈0，实际 {}", timg.get_pixel(2, 2)[3]);
    assert!(timg.get_pixel(32, 24)[3] > 245, "中心角色区 alpha 应≈255，实际 {}", timg.get_pixel(32, 24)[3]);
  }

  // —— 批次 2：仅缩略图 ——

  /// 经既有链路入库一个 fla（带伴生 webp），返回 (fla_asset_id, webp_asset_id, fla 池内路径)
  fn import_fla_into_lib(lib: &mut Library, dir: &Path) -> (String, String, PathBuf) {
    let src = dir.join("out");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("跑.fla"), b"fake-fla").unwrap();
    image::RgbaImage::from_pixel(64, 32, image::Rgba([200, 30, 30, 255])).save(src.join("跑.png")).unwrap();
    let p = payload(&src, "跑", false);
    let prepared = prepare(lib, &p).unwrap();
    let (o, b) = compose(&prepared).unwrap();
    let outcome = commit(lib, &prepared, o, b).unwrap();
    let pool = paths::origin_file(&lib.assets_dir, &outcome.fla_asset_id, "跑.fla");
    (outcome.fla_asset_id, outcome.webp_asset_id, pool)
  }

  fn thumb_payload(doc: &Path, png: &Path) -> ThumbPayload {
    ThumbPayload {
      doc_path: doc.to_string_lossy().into_owned(),
      dynamic: false,
      seq_dir: None,
      png_path: Some(png.to_string_lossy().into_owned()),
      fps: 25.0,
    }
  }

  /// 路径匹配：池内实体路径精确命中（反斜杠 / 大小写规范化同命中）；池外、非 fla 资产不命中
  #[test]
  fn match_fla_asset_by_pool_path() {
    let (dir, mut lib) = make_lib();
    let (fla_id, webp_id, pool) = import_fla_into_lib(&mut lib, dir.path());
    assert_eq!(match_fla_asset(&lib, &pool.to_string_lossy()).as_deref(), Some(fla_id.as_str()));
    let alt = pool.to_string_lossy().replace('/', "\\").to_uppercase();
    assert_eq!(match_fla_asset(&lib, &alt).as_deref(), Some(fla_id.as_str()), "大小写/斜杠规范化");
    assert_eq!(match_fla_asset(&lib, "D:/elsewhere/跑.fla"), None, "池外不命中");
    let webp_pool = paths::origin_file(&lib.assets_dir, &webp_id, "跑.webp");
    assert_eq!(match_fla_asset(&lib, &webp_pool.to_string_lossy()), None, "非 fla 不命中");
  }

  /// 命中端到端：WebP 入库 + 换绑（旧 hidden 不动，PRD 口径）+ 清场
  #[test]
  fn thumb_only_bound_end_to_end_and_rebind() {
    let (dir, mut lib) = make_lib();
    let (fla_id, old_webp_id, pool) = import_fla_into_lib(&mut lib, dir.path());
    let tdir = dir.path().join("thumb");
    fs::create_dir_all(&tdir).unwrap();
    image::RgbaImage::from_pixel(32, 32, image::Rgba([30, 30, 200, 255])).save(tdir.join("跑.png")).unwrap();
    let tp = thumb_payload(&pool, &tdir.join("跑.png"));

    let prepared = thumb_prepare(&lib, &tp).unwrap();
    assert_eq!(prepared.matched.as_deref(), Some(fla_id.as_str()));
    let (o, b) = thumb_compose(&prepared).unwrap();
    let out = thumb_commit(&mut lib, &prepared, o, b).unwrap();
    let ThumbOutcome::Bound { asset_id, webp_asset_id, name, .. } = out else { panic!("应命中绑定") };
    assert_eq!(asset_id, fla_id);
    assert_eq!(name, "跑");
    assert_eq!(lib.index.assets[&fla_id].thumb_asset_id.as_deref(), Some(webp_asset_id.as_str()), "换绑新缩略图");
    assert_eq!(lib.index.assets[&webp_asset_id].hidden, Some(true));
    assert_eq!(lib.index.assets[&old_webp_id].hidden, Some(true), "旧 hidden 不动（PRD：手动解除或删除）");
    assert!(!tdir.join("跑.png").exists(), "静图 PNG 已清场");
    assert!(!prepared.webp_tmp.exists(), "webp 已剪切入库");
  }

  /// 池外兜底：不入库，WebP 挪到原 fla 旁边；再导一次重名 -2 唯一化
  #[test]
  fn thumb_only_beside_when_outside_pool() {
    let (dir, mut lib) = make_lib();
    let proj = dir.path().join("proj");
    fs::create_dir_all(&proj).unwrap();
    fs::write(proj.join("项目.fla"), b"fake-fla").unwrap();
    let run = |lib: &mut Library| -> ThumbOutcome {
      image::RgbaImage::from_pixel(32, 32, image::Rgba([30, 200, 30, 255])).save(proj.join("项目.png")).unwrap();
      let tp = thumb_payload(&proj.join("项目.fla"), &proj.join("项目.png"));
      let prepared = thumb_prepare(lib, &tp).unwrap();
      assert!(prepared.matched.is_none(), "池外不匹配");
      let (o, b) = thumb_compose(&prepared).unwrap();
      thumb_commit(lib, &prepared, o, b).unwrap()
    };
    let ThumbOutcome::Beside { out_path, .. } = run(&mut lib) else { panic!("应走池外兜底") };
    assert_eq!(normalize_path(&out_path), normalize_path(&proj.join("项目.webp").to_string_lossy()));
    assert!(proj.join("项目.webp").is_file());
    assert!(lib.index.assets.is_empty(), "不入库");
    assert!(!proj.join("项目.png").exists(), "清场");
    let ThumbOutcome::Beside { out_path: second, .. } = run(&mut lib) else { panic!() };
    assert_eq!(normalize_path(&second), normalize_path(&proj.join("项目-2.webp").to_string_lossy()), "重名 -2 唯一化");
  }
}
