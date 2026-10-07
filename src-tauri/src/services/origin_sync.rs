//! origin 外部编辑同步（2026-10-08，哈希漂移治理第 2 层落地——待办 2026-09-22）：
//! 库内 origin 可能被外部编辑器改写（Typora 存 md、Word 存 docx、外部工具覆盖导出），
//! metadata 的 size / hash / 宽高停留在建档时刻，卡片文本预览（modifiedAt 版本戳缓存）随之陈旧。
//!
//! 两条路径：
//! - `heal_if_drifted` = 读取触点自愈（详情 get_asset_detail / 文本卡 read_text_asset /
//!   docx 卡 read_docx_asset）：stat 检测 size 不符或文件 mtime 晚于 modifiedAt → 回写 size /
//!   hash / 宽高 + 非音视频重生成内容缩略图 + modifiedAt 刷新（先落盘后同步索引，规范第 7 条；
//!   与 ensure_asset_colors 同属"读命令带维护性写副作用"先例——不进撤销、失败记日志不阻断读取）。
//!   hash 重算有体积上限：读触点同步执行容不下大文件哈希，超出维持旧 hash 记日志，真值交手动托底。
//! - `refresh_from_origin` = 手动托底（右键「刷新缩略图」）：无条件重算 hash + 全格式重跑导入同
//!   管线缩略图（含音视频首帧 / 波形）；文本等无内容缩略图格式不再报"不支持"——数据照刷、
//!   缩略图回退格式图标（meta.thumbnail 置 None，自定义缩略图随之回退，与图片格式"刷新回退
//!   origin 默认图"同语义）。
//!
//! 序列帧（size = 全帧合计）与虚拟片段（origin 无实体）不适用：自愈直接跳过；手动刷新按
//! 旧口径报「源文件缺失」。检测精度：modifiedAt 秒级落盘，同秒内的外部编辑漏检由手动托底兜住。

use std::fs;
use std::path::Path;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::core::json_io::write_json;
use crate::core::library::Library;
use crate::core::models::AssetMeta;
use crate::services::{import, media, thumbnail};

/// 自愈路径 hash 重算体积上限（64MB）：读触点同步执行，大文件流式哈希是秒级阻塞——
/// 超出只同步 size / 宽高 / modifiedAt，hash 维持旧值（log 标记），真值交手动「刷新缩略图」托底
const HEAL_HASH_MAX: u64 = 64 * 1024 * 1024;

/// 同步产物（手动托底返回前端；自愈路径取 size / modified_at 广播）
#[derive(Debug)]
pub struct SyncOutcome {
  pub size: u64,
  pub modified_at: String,
  /// 重生成的内容缩略图绝对路径；None = 无内容缩略图格式（文本等）或音视频自愈跳过——
  /// 前端吐司分流（已刷新缩略图 / 已更新文件数据）
  pub thumb_path: Option<String>,
}

/// 手动托底结果（commands/asset_menu refresh_thumbnail 返回，camelCase 上前端）
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshResult {
  pub thumb_path: Option<String>,
  pub size: u64,
  pub modified_at: String,
}

/// 自愈广播负载（asset-origin-healed）：前端同步该资产全部 Placement 的 size / modifiedAt
///（卡片尺寸标签即时正确 + 文本卡 modifiedAt 版本戳失效自动重拉，同 video-fps-healed 口径）
pub fn emit_healed(app: &AppHandle, asset_id: &str, size: u64, modified_at: &str) {
  let _ = app.emit(
    "asset-origin-healed",
    serde_json::json!({ "assetId": asset_id, "size": size, "modifiedAt": modified_at }),
  );
}

/// 同步适用对象：普通单文件资产（序列帧 size 语义不同、虚拟片段 origin 无实体，均排除）
fn eligible(lib: &Library, asset_id: &str, meta: &AssetMeta) -> bool {
  meta.r#type.is_none()
    && crate::services::sequence::frames_dir_if_sequence(&lib.assets_dir, asset_id, meta).is_none()
}

/// RFC3339 → epoch 秒（modified_at 由本库写盘恒为合法 RFC3339；解析失败归 0 = 恒漂移，自愈一次收敛）
fn rfc3339_secs(s: &str) -> u64 {
  chrono::DateTime::parse_from_rfc3339(s).map(|d| d.timestamp().max(0) as u64).unwrap_or(0)
}

/// 漂移检测：size 不符，或 origin 文件 mtime 晚于 modifiedAt（秒级）。origin 缺失 / stat 失败
/// 不算漂移（读取触点只补数不报警，缺失报错属手动刷新路径）
pub fn drifted(meta: &AssetMeta, origin: &Path) -> bool {
  let Ok(fmd) = fs::metadata(origin) else { return false };
  if fmd.len() != meta.size {
    return true;
  }
  let mtime = fmd.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
  mtime > rfc3339_secs(&meta.modified_at)
}

/// 读取触点自愈：漂移 → 回写并返回 Some((size, modified_at))（调用方广播 emit_healed）；
/// 不适用 / 无漂移 / 失败（记日志不阻断读取）→ None
pub fn heal_if_drifted(lib: &mut Library, asset_id: &str) -> Option<(u64, String)> {
  let meta = lib.index.assets.get(asset_id)?.clone();
  if !eligible(lib, asset_id, &meta) {
    return None;
  }
  let origin = lib.info_dir(asset_id).join(&meta.origin_name);
  if !drifted(&meta, &origin) {
    return None;
  }
  match sync(lib, asset_id, SyncMode::Heal) {
    Ok(out) => Some((out.size, out.modified_at)),
    Err(e) => {
      log::warn!("origin 外部编辑自愈失败（{asset_id}）：{e}");
      None
    }
  }
}

/// 手动托底（右键「刷新缩略图」）：无条件重算 + 全格式缩略图管线。
/// 序列帧 / 虚拟片段 / origin 缺失 → 「源文件缺失」（旧 refresh_thumbnail 同口径）
pub fn refresh_from_origin(lib: &mut Library, asset_id: &str) -> Result<SyncOutcome, String> {
  sync(lib, asset_id, SyncMode::Refresh)
}

enum SyncMode {
  /// 自愈：drift 触发；hash ≤64MB；非音视频才重生成缩略图；无内容缩略图格式不动既有缩略图
  Heal,
  /// 手动：无条件；hash 不限体积；全格式缩略图管线；无内容缩略图格式回退格式图标
  Refresh,
}

/// 同步核心：重算 size / hash / 宽高 → 缩略图（按模式门禁）→ modifiedAt 刷新 →
/// 先落盘 metadata.json 再同步内存索引（规范第 7 条）。不进撤销（维护性写，同 fps 自愈口径）
fn sync(lib: &mut Library, asset_id: &str, mode: SyncMode) -> Result<SyncOutcome, String> {
  let mut meta = lib.index.assets.get(asset_id).ok_or_else(|| "资源不存在，可能已被删除".to_string())?.clone();
  let info_dir = lib.info_dir(asset_id);
  let origin = info_dir.join(&meta.origin_name);
  if !origin.is_file() {
    return Err(format!("「{}」的源文件缺失", meta.name));
  }

  let heal = matches!(mode, SyncMode::Heal);
  meta.size = fs::metadata(&origin).map(|m| m.len()).unwrap_or(0);
  meta.hash = if heal && meta.size > HEAL_HASH_MAX {
    log::info!("自愈跳过大文件 hash 重算（{}，{}B）——手动「刷新缩略图」可求真值", meta.name, meta.size);
    meta.hash.clone()
  } else {
    import::sha256_file(&origin).map_err(|e| format!("内容指纹计算失败：{e}"))?
  };
  // 宽高重探（图片族：魔数 + PSD/SVG/字体；音视频尺寸维持——ffprobe 重活不进同步路径）
  if let Some((w, h)) = thumbnail::probe(&origin) {
    meta.width = Some(w);
    meta.height = Some(h);
  }

  // 内容缩略图：手动 = 全格式强跑（音视频首帧 / 波形，旧 refresh_thumbnail 语义）；
  // 自愈 = 仅非音视频（读触点成本闸；音视频外部覆盖后缩略图靠手动刷新托底）
  let is_media = media::is_video(&origin) || media::is_audio(&origin);
  let thumb = if !heal || !is_media { thumbnail::generate(&info_dir, &origin, &meta.name) } else { None };
  let thumb_path = match thumb {
    Some(t) => {
      let path = info_dir.join(&t.name);
      meta.thumbnail = Some(t.name);
      meta.thumb_width = Some(t.width);
      meta.thumb_height = Some(t.height);
      meta.colors = t.colors;
      Some(path.to_string_lossy().into_owned())
    }
    None if !heal && !is_media => {
      // 手动刷新且无内容缩略图格式（文本等）：回退格式图标——自定义缩略图一并回退
      //（与图片格式"刷新回退 origin 默认图"同语义）；旧缩略图文件顺手清理
      if let Some(old) = meta.thumbnail.take() {
        let _ = fs::remove_file(info_dir.join(old));
      }
      meta.thumb_width = None;
      meta.thumb_height = None;
      meta.colors = Vec::new();
      None
    }
    None => None, // 自愈遇无内容缩略图格式 / 音视频缩略图失败：维持既有缩略图不动
  };

  meta.modified_at = super::operations::now();
  write_json(&info_dir.join("metadata.json"), &meta).map_err(|e| e.to_string())?;
  let out = SyncOutcome { size: meta.size, modified_at: meta.modified_at.clone(), thumb_path };
  lib.index.assets.insert(asset_id.to_string(), meta);
  Ok(out)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::json_io::read_json;
  use crate::core::library;
  use crate::services::import::{self as ops, PendingFile, Source};
  use crate::services::operations::finish;
  use std::collections::HashMap;
  use std::path::PathBuf;

  struct Harness {
    dir: tempfile::TempDir,
    lib: Library,
  }

  fn make_lib() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    Harness { dir, lib }
  }

  /// 造文本资产入库（真实内容），返回 info 目录
  fn add_text(h: &mut Harness, id: &str, name: &str, content: &str) -> PathBuf {
    let src = h.dir.path().join(name);
    fs::write(&src, content).unwrap();
    let pf = PendingFile { path: src, chain: Vec::new() };
    let meta = ops::build_asset_with_id(&h.lib.assets_dir, &pf, "sha256:t", id, Source::Copy, None).unwrap();
    h.lib.index.assets.insert(id.into(), meta);
    ops::add_placement(&mut h.lib, id.into(), "root".into(), name.into(), true, &mut HashMap::new());
    finish(&mut h.lib).unwrap();
    h.lib.info_dir(id)
  }

  /// 外部编辑模拟：覆写 origin 文件（Typora 存盘等价物）
  fn external_edit(info_dir: &Path, origin_name: &str, content: &str) {
    fs::write(info_dir.join(origin_name), content).unwrap();
  }

  #[test]
  fn heal_detects_external_edit_and_syncs() {
    let mut h = make_lib();
    let info = add_text(&mut h, "AN-T", "提示词.md", "# 旧");
    external_edit(&info, "提示词.md", "# 新内容\n第二行 🎬");

    let (size, modified_at) = heal_if_drifted(&mut h.lib, "AN-T").expect("外部编辑应检到漂移");
    assert_eq!(size, "# 新内容\n第二行 🎬".len() as u64);
    let meta = &h.lib.index.assets["AN-T"];
    assert_eq!(meta.size, size);
    assert_eq!(meta.hash, import::sha256_bytes("# 新内容\n第二行 🎬".as_bytes()), "hash 跟上内容");
    assert_eq!(meta.modified_at, modified_at);
    // 先落盘后内存（规范第 7 条）
    let disk: AssetMeta = read_json(&info.join("metadata.json")).unwrap();
    assert_eq!(disk.size, size);
    assert_eq!(disk.hash, meta.hash);
    // 文本格式：无内容缩略图，thumbnail 恒 None
    assert_eq!(meta.thumbnail, None);
  }

  #[test]
  fn heal_clean_asset_noop() {
    let mut h = make_lib();
    add_text(&mut h, "AN-T", "提示词.md", "# 原样");
    assert_eq!(heal_if_drifted(&mut h.lib, "AN-T"), None, "无漂移不自愈");
  }

  #[test]
  fn heal_detects_mtime_drift_same_size() {
    let mut h = make_lib();
    let info = add_text(&mut h, "AN-T", "提示词.md", "abc");
    external_edit(&info, "提示词.md", "xyz"); // 同长度覆写：size 不变
    // 同秒完成时 mtime 不晚于 modifiedAt（秒级精度）——把 modifiedAt 拨回过去模拟跨秒编辑
    h.lib.index.assets.get_mut("AN-T").unwrap().modified_at = "2020-01-01T00:00:00+08:00".into();
    let (size, _) = heal_if_drifted(&mut h.lib, "AN-T").expect("mtime 漂移应检到");
    assert_eq!(size, 3);
    assert_eq!(h.lib.index.assets["AN-T"].hash, import::sha256_bytes(b"xyz"));
  }

  #[test]
  fn heal_missing_origin_noop_not_error() {
    let mut h = make_lib();
    let info = add_text(&mut h, "AN-T", "提示词.md", "abc");
    fs::remove_file(info.join("提示词.md")).unwrap();
    assert_eq!(heal_if_drifted(&mut h.lib, "AN-T"), None, "读取触点只补数不报警");
  }

  #[test]
  fn refresh_text_doc_syncs_and_resets_thumb() {
    let mut h = make_lib();
    let info = add_text(&mut h, "AN-T", "提示词.md", "");
    external_edit(&info, "提示词.md", "# 外部写入");
    // 模拟用户曾自定义缩略图（info 内放一张图并登记）
    let custom = info.join("提示词_thumbnail.webp");
    image::RgbaImage::from_pixel(8, 8, image::Rgba([1, 2, 3, 255])).save(&custom).unwrap();
    {
      let m = h.lib.index.assets.get_mut("AN-T").unwrap();
      m.thumbnail = Some("提示词_thumbnail.webp".into());
      m.thumb_width = Some(8);
      m.thumb_height = Some(8);
    }

    let out = refresh_from_origin(&mut h.lib, "AN-T").unwrap();
    let meta = &h.lib.index.assets["AN-T"];
    assert_eq!(meta.size, "# 外部写入".len() as u64);
    assert_eq!(meta.hash, import::sha256_bytes("# 外部写入".as_bytes()));
    assert_eq!(out.thumb_path, None, "文本无内容缩略图");
    assert_eq!(meta.thumbnail, None, "回退格式图标");
    assert_eq!(meta.thumb_width, None);
    assert!(meta.colors.is_empty());
    assert!(!custom.exists(), "旧缩略图文件已清理");
  }

  #[test]
  fn refresh_rehashes_even_without_drift() {
    // 托底语义：自愈漏检场景（同秒同长度编辑 + modifiedAt 已被后续数据编辑刷新）
    // 手动刷新仍无条件重算 hash
    let mut h = make_lib();
    let info = add_text(&mut h, "AN-T", "提示词.md", "abc");
    external_edit(&info, "提示词.md", "xyz");
    let now = super::super::operations::now();
    h.lib.index.assets.get_mut("AN-T").unwrap().modified_at = now; // 抹掉 mtime 漂移信号
    assert!(!drifted(&h.lib.index.assets["AN-T"], &info.join("提示词.md")), "前置：漂移信号已抹平");
    refresh_from_origin(&mut h.lib, "AN-T").unwrap();
    assert_eq!(h.lib.index.assets["AN-T"].hash, import::sha256_bytes(b"xyz"), "手动刷新无条件求真值");
  }

  #[test]
  fn refresh_missing_origin_errors() {
    let mut h = make_lib();
    let info = add_text(&mut h, "AN-T", "提示词.md", "abc");
    fs::remove_file(info.join("提示词.md")).unwrap();
    let err = refresh_from_origin(&mut h.lib, "AN-T").unwrap_err();
    assert!(err.contains("源文件缺失"), "{err}");
    assert!(refresh_from_origin(&mut h.lib, "AN-不存在").unwrap_err().contains("不存在"));
  }

  #[test]
  fn refresh_image_regenerates_thumbnail() {
    let mut h = make_lib();
    let src = h.dir.path().join("图.png");
    image::RgbaImage::from_pixel(100, 80, image::Rgba([30, 30, 200, 255])).save(&src).unwrap();
    let pf = PendingFile { path: src, chain: Vec::new() };
    let meta = ops::build_asset_with_id(&h.lib.assets_dir, &pf, "sha256:t", "AN-P", Source::Copy, None).unwrap();
    h.lib.index.assets.insert("AN-P".into(), meta);
    ops::add_placement(&mut h.lib, "AN-P".into(), "root".into(), "图.png".into(), true, &mut HashMap::new());
    finish(&mut h.lib).unwrap();

    let out = refresh_from_origin(&mut h.lib, "AN-P").unwrap();
    let thumb = out.thumb_path.expect("图片应重生成内容缩略图");
    assert!(thumb.ends_with("图_thumbnail.webp"), "{thumb}");
    assert!(PathBuf::from(&thumb).is_file());
    let meta = &h.lib.index.assets["AN-P"];
    assert_eq!(meta.thumb_width, Some(100));
    assert_eq!(meta.thumb_height, Some(80));
  }
}
