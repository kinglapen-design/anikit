//! 命令 API：封面帧切换（4.3 序列帧起，5.2 扩展动图 / 视频）。
//! 帧来源三路：序列帧 = info/frames/ 帧原图；**动图 = origin 运行时解码抽帧**（`f:{索引}`，
//! 2026-08-20 解包废除后无磁盘帧图，anim::decode_frame 单帧解码）；视频 = ffmpeg 任意时间点截帧。
//! 三路共用同一条缩略图再生成管线（thumbnail::generate，
//! 与导入同参数同命名 {资源名}_thumbnail.webp），先落盘 metadata.json 再同步内存索引（规范第 7 条）。

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::State;

use crate::adapters::ffmpeg;
use crate::commands::library::AppState;
use crate::commands::preview::locate_asset;
use crate::core::json_io::write_json;
use crate::core::library::{Library, CACHE_DIR};
use crate::core::models::AssetMeta;
use crate::services::{media, thumbnail};

/// 封面帧切换结果
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetThumbResult {
  /// 新缩略图绝对路径
  pub thumb_path: String,
  /// metadata.modifiedAt（前端同步进 PlacementView 作缩略图 URL 版本戳——
  /// asset 协议同 URL 有启发式缓存，版本戳驱动 img 重新加载，2026-08-20 用户实机"静帧不热刷新"修复）
  pub modified_at: String,
}

/// 切换序列帧 / 动图封面帧：用指定帧重新生成缩略图 → metadata 落盘（thumbFrame / thumbnail /
/// modifiedAt）→ 同步内存索引。
/// frame_name：序列帧 = frames/ 内帧文件名；**动图 = `f:{帧索引0起}`**（如 f:41——origin 解码抽帧）。
/// async：涉及 IO（缩略图再生成），一律走线程池（规范第 2 条）
#[tauri::command]
pub async fn set_thumb_frame(
  state: State<'_, AppState>,
  asset_id: String,
  frame_name: String,
) -> Result<SetThumbResult, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  apply_thumb_frame(lib, &asset_id, &frame_name)
}

fn apply_thumb_frame(lib: &mut Library, asset_id: &str, frame_name: &str) -> Result<SetThumbResult, String> {
  let (info_dir, origin, meta) = locate_asset(lib, asset_id)?;
  // 动图分支（解包废除）：`f:{索引}` → origin 解码抽帧，thumbFrame 同格式记录
  if crate::services::anim::is_anim(&meta) {
    let idx = parse_anim_frame_key(frame_name)?;
    let (img, _delay) = crate::services::anim::decode_frame(&origin, idx)
      .ok_or_else(|| format!("第 {} 帧解码失败，请重试", idx + 1))?;
    // thumbnail::generate 需要文件路径：抽帧先落 cache/tmp 临时 png，生成完即删（同视频分支套路）
    let tmp = lib.root.join(CACHE_DIR).join("tmp").join(format!("anim-thumb-{asset_id}.png"));
    if let Some(dir) = tmp.parent() {
      fs::create_dir_all(dir).map_err(|e| format!("临时目录创建失败：{e}"))?;
    }
    img.save(&tmp).map_err(|e| format!("临时文件写入失败：{e}"))?;
    let thumb = regenerate(&info_dir, &meta, &tmp);
    let _ = fs::remove_file(&tmp);
    let thumb = thumb.ok_or_else(|| "封面帧缩略图生成失败，请重试".to_string())?;
    return persist(lib, info_dir, meta, asset_id, thumb, frame_name.to_string());
  }
  let frame_path = resolve_frame_path(&info_dir, &meta, frame_name)?;
  let thumb = regenerate(&info_dir, &meta, &frame_path).ok_or_else(|| format!("帧图片解码失败：{frame_name}"))?;
  persist(lib, info_dir, meta, asset_id, thumb, frame_name.to_string())
}

/// 动图封面帧 key 解析：`f:{帧索引0起}`（如 f:41）。非法格式 / 非数字报中文错
fn parse_anim_frame_key(key: &str) -> Result<u32, String> {
  let Some(num) = key.strip_prefix("f:") else {
    return Err(format!("动图封面帧定位应为 f:帧序号（收到：{key}）"));
  };
  num.parse::<u32>().map_err(|_| format!("动图封面帧序号不合法：{key}"))
}

/// 视频设封面：ffmpeg 截取 at_secs 秒画面 → 临时 PNG 走同管线再生成缩略图 →
/// thumbFrame 记 `t:{秒}`（如 "t:12.500"，与序列帧 / 动图的帧名区分）→ 落盘 + 同步索引。
/// async：ffmpeg sidecar 截帧为 IO 重活，一律走线程池（规范第 2 条）
#[tauri::command]
pub async fn set_video_thumb(
  state: State<'_, AppState>,
  asset_id: String,
  at_secs: f64,
) -> Result<SetThumbResult, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  apply_video_thumb(lib, &asset_id, at_secs)
}

fn apply_video_thumb(lib: &mut Library, asset_id: &str, at_secs: f64) -> Result<SetThumbResult, String> {
  let (info_dir, origin, meta) = locate_asset(lib, asset_id)?;
  if !media::is_video(&origin) {
    return Err(format!("「{}」不是视频资源", meta.name));
  }
  let png = ffmpeg::video_frame_png(&origin, at_secs)
    .ok_or_else(|| "无法截取该时间点的画面，请换个时间点重试".to_string())?;
  // thumbnail::generate 需要文件路径：PNG 字节先落 cache/tmp/ 临时文件，生成完即删
  let tmp = lib.root.join(CACHE_DIR).join("tmp").join(format!("video-thumb-{asset_id}.png"));
  if let Some(dir) = tmp.parent() {
    fs::create_dir_all(dir).map_err(|e| format!("临时目录创建失败：{e}"))?;
  }
  fs::write(&tmp, &png).map_err(|e| format!("临时文件写入失败：{e}"))?;
  let thumb = regenerate(&info_dir, &meta, &tmp);
  let _ = fs::remove_file(&tmp);
  let thumb = thumb.ok_or_else(|| "封面帧图片解码失败，请重试".to_string())?;
  persist(lib, info_dir, meta, asset_id, thumb, format!("t:{at_secs:.3}"))
}

/// 帧文件定位：序列帧取 info/frames/ 帧原图（动图走 `f:{索引}` 抽帧分支，不进此函数）
fn resolve_frame_path(info_dir: &Path, meta: &AssetMeta, frame_name: &str) -> Result<PathBuf, String> {
  let frames_dir = info_dir.join("frames");
  if !frames_dir.is_dir() {
    return Err(format!("「{}」不是序列帧资源", meta.name));
  }
  let path = frames_dir.join(frame_name);
  path.is_file().then_some(path).ok_or_else(|| format!("帧不存在：{frame_name}"))
}

/// 重新生成缩略图：词干 = 资源名，与导入同管线同命名（{资源名}_thumbnail.webp）。
/// 同名交给 generate 直接覆盖（避免先删后生成失败留下悬空引用），仅不同名（改名遗留）才先删
fn regenerate(info_dir: &Path, meta: &AssetMeta, frame_path: &Path) -> Option<String> {
  if let Some(old) = meta.thumbnail.as_deref() {
    if old != format!("{}_thumbnail.webp", meta.name) && old != format!("{}_thumbnail.png", meta.name) {
      if let Err(e) = fs::remove_file(info_dir.join(old)) {
        log::warn!("旧缩略图删除失败（{old}）：{e}");
      }
    }
  }
  thumbnail::generate(info_dir, frame_path, &meta.name)
}

/// metadata 落盘（thumbFrame / thumbnail / modifiedAt）→ 同步内存索引（先落盘再更新，规范第 7 条）；
/// modifiedAt 随结果返回（前端作缩略图 URL 版本戳）
fn persist(
  lib: &mut Library,
  info_dir: PathBuf,
  mut meta: AssetMeta,
  asset_id: &str,
  thumb: String,
  thumb_frame: String,
) -> Result<SetThumbResult, String> {
  meta.thumb_frame = Some(thumb_frame);
  meta.thumbnail = Some(thumb.clone());
  meta.modified_at = chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, false);
  let modified_at = meta.modified_at.clone();
  write_json(&info_dir.join("metadata.json"), &meta).map_err(|e| e.to_string())?;
  lib.index.assets.insert(asset_id.to_string(), meta);
  Ok(SetThumbResult { thumb_path: info_dir.join(&thumb).to_string_lossy().into_owned(), modified_at })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::json_io::read_json;
  use crate::core::library;
  use crate::services::import::{self, PendingFile, Source};
  use crate::services::sequence;

  struct Harness {
    dir: tempfile::TempDir,
    lib: Library,
  }

  fn make_lib() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    Harness { dir, lib }
  }

  fn make_png(path: &Path, w: u32, h: u32) {
    image::RgbaImage::from_pixel(w, h, image::Rgba([30, 30, 200, 255])).save(path).unwrap();
  }

  /// 造一个序列帧资源（走路_1..=3.png，入库归一 走路-001..003）并登记进内存索引
  fn add_sequence(h: &mut Harness, id: &str) -> AssetMeta {
    let src = h.dir.path().join(format!("源-{id}"));
    fs::create_dir_all(&src).unwrap();
    let frames: Vec<(PathBuf, u32)> = (1..=3)
      .map(|n| {
        let p = src.join(format!("走路_{n}.png"));
        make_png(&p, 640, 480);
        (p, n)
      })
      .collect();
    let meta =
      sequence::build_sequence_asset(&h.lib.assets_dir, "走路", "png", 3, &frames, "sha256:t", id, Source::Copy, None)
        .unwrap();
    h.lib.index.assets.insert(id.to_string(), meta.clone());
    meta
  }

  /// 造一个单文件资源（png 造真图，其他扩展名写假字节）并登记进内存索引
  fn add_file(h: &mut Harness, id: &str, name: &str) -> AssetMeta {
    let src = h.dir.path().join(name);
    if name.ends_with(".png") {
      make_png(&src, 100, 80);
    } else {
      fs::write(&src, b"fake").unwrap();
    }
    let pf = PendingFile { path: src, chain: Vec::new() };
    let meta = import::build_asset_with_id(&h.lib.assets_dir, &pf, "sha256:t", id, Source::Copy, None).unwrap();
    h.lib.index.assets.insert(id.to_string(), meta.clone());
    meta
  }

  /// 造一个动图资源（真实样本 anim3.gif 入库，3 帧）并登记进内存索引
  fn add_anim(h: &mut Harness, id: &str) -> AssetMeta {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/test-assets/anim3.gif");
    let pf = PendingFile { path: src, chain: Vec::new() };
    let meta = import::build_asset_with_id(&h.lib.assets_dir, &pf, "sha256:t", id, Source::Copy, None).unwrap();
    h.lib.index.assets.insert(id.to_string(), meta.clone());
    meta
  }

  #[test]
  fn set_thumb_frame_sequence_branch_regenerates_and_syncs_index() {
    let mut h = make_lib();
    let before = add_sequence(&mut h, "AN-SEQ3");
    let info = h.lib.info_dir("AN-SEQ3");

    let result = apply_thumb_frame(&mut h.lib, "AN-SEQ3", "走路-002.png").unwrap();
    // 新缩略图沿用导入命名（词干 = 资源名），绝对路径指向 info 内文件
    assert!(result.thumb_path.ends_with("走路_thumbnail.webp"), "{}", result.thumb_path);
    assert!(PathBuf::from(&result.thumb_path).is_file());

    // metadata 落盘：thumbFrame / thumbnail / modifiedAt 更新
    let disk: AssetMeta = read_json(&info.join("metadata.json")).unwrap();
    assert_eq!(disk.thumb_frame.as_deref(), Some("走路-002.png"));
    assert_eq!(disk.thumbnail.as_deref(), Some("走路_thumbnail.webp"));
    assert_ne!(disk.thumb_frame, before.thumb_frame);

    // 内存索引同步（先落盘再更新）
    let indexed = &h.lib.index.assets["AN-SEQ3"];
    assert_eq!(indexed.thumb_frame.as_deref(), Some("走路-002.png"));
    assert_eq!(indexed.thumbnail.as_deref(), Some("走路_thumbnail.webp"));

    // 帧不存在 / 非序列帧资源 → 中文报错
    assert!(apply_thumb_frame(&mut h.lib, "AN-SEQ3", "走路-999.png").is_err());
    add_file(&mut h, "AN-ONE2", "单张.png");
    let err = apply_thumb_frame(&mut h.lib, "AN-ONE2", "走路-001.png").unwrap_err();
    assert!(err.contains("不是序列帧"), "{err}");
  }

  /// 造一个 webp 动图资源（真实样本 anim3.webp 入库，3 帧 200ms）并登记进内存索引
  fn add_anim_webp(h: &mut Harness, id: &str) -> AssetMeta {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/test-assets/anim3.webp");
    let pf = PendingFile { path: src, chain: Vec::new() };
    let meta = import::build_asset_with_id(&h.lib.assets_dir, &pf, "sha256:t", id, Source::Copy, None).unwrap();
    h.lib.index.assets.insert(id.to_string(), meta.clone());
    meta
  }

  /// 动图封面帧（2026-08-20 解包废除）：`f:{索引}` → origin 运行时解码抽帧，thumbFrame 同格式记录
  #[test]
  fn set_thumb_frame_anim_branch_decodes_origin() {
    let mut h = make_lib();
    add_anim(&mut h, "AN-GIF");
    let info = h.lib.info_dir("AN-GIF");

    // f:1（第 2 帧）抽帧成功：缩略图生成 + thumbFrame 记 f:1 + 索引同步
    let result = apply_thumb_frame(&mut h.lib, "AN-GIF", "f:1").unwrap();
    assert!(result.thumb_path.ends_with("anim3_thumbnail.webp"), "{}", result.thumb_path);
    assert!(PathBuf::from(&result.thumb_path).is_file());
    let disk: AssetMeta = read_json(&info.join("metadata.json")).unwrap();
    assert_eq!(disk.thumb_frame.as_deref(), Some("f:1"));
    assert_eq!(h.lib.index.assets["AN-GIF"].thumb_frame.as_deref(), Some("f:1"));

    // 越界索引 / 非法 key → 中文报错，不动 metadata
    assert!(apply_thumb_frame(&mut h.lib, "AN-GIF", "f:99").is_err());
    let err = apply_thumb_frame(&mut h.lib, "AN-GIF", "anim3-00002.webp").unwrap_err();
    assert!(err.contains("f:帧序号"), "{err}");
    let after: AssetMeta = read_json(&info.join("metadata.json")).unwrap();
    assert_eq!(after.thumb_frame.as_deref(), Some("f:1"), "报错不应改动 metadata");
  }

  /// webp 动图同分支（image-webp 桥）：f:2 抽帧成功 + thumbFrame 记录
  #[test]
  fn set_thumb_frame_anim_webp_branch_decodes_origin() {
    let mut h = make_lib();
    add_anim_webp(&mut h, "AN-WEBP");
    let info = h.lib.info_dir("AN-WEBP");

    let result = apply_thumb_frame(&mut h.lib, "AN-WEBP", "f:2").unwrap();
    assert!(PathBuf::from(&result.thumb_path).is_file());
    assert!(!result.modified_at.is_empty(), "modifiedAt 应随结果返回（前端 URL 版本戳）");
    // 抽帧确实来自第 3 帧（anim3.webp 帧色按序递增：非首帧纯色即可区分——与 gif 分支同断言规格）
    let img = image::open(&result.thumb_path).unwrap().to_rgba8();
    let p = img.get_pixel(0, 0);
    assert!(p[3] == 255 || p[0] + p[1] + p[2] > 0, "缩略图应有内容：{p:?}");
    let disk: AssetMeta = read_json(&info.join("metadata.json")).unwrap();
    assert_eq!(disk.thumb_frame.as_deref(), Some("f:2"));
    assert!(apply_thumb_frame(&mut h.lib, "AN-WEBP", "f:99").is_err());
  }

  #[test]
  fn set_video_thumb_routes_and_errors() {
    let mut h = make_lib();
    // 非视频资源 → 中文报错
    add_file(&mut h, "AN-PIC", "参考图.png");
    let err = apply_video_thumb(&mut h.lib, "AN-PIC", 1.0).unwrap_err();
    assert!(err.contains("不是视频资源"), "{err}");

    // 假视频（fake bytes）：ffmpeg 截帧失败 → 中文人话文案（sidecar 缺失同走此路径）
    add_file(&mut h, "AN-VID", "片段.mp4");
    let err = apply_video_thumb(&mut h.lib, "AN-VID", 12.5).unwrap_err();
    assert!(err.contains("无法截取该时间点的画面"), "{err}");
  }
}
