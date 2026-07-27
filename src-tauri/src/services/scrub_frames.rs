//! 视频悬停 scrub 预览帧（5.3，2026-07-28 Eagle 帧图方案 + **导入期预抽**定案）：
//! 全片均匀抽帧（≤30s 按 4fps / >30s 按 0.5fps，8~150 帧封顶，480 高 libwebp）落
//! `cache/scrub/{assetId}/f-NNN.webp`（**会话级**，开库清空名单收录）+ `.stamp` 源戳
//! （"大小:mtime秒"，源变化整目录重抽）。帧数加密对齐 Eagle 连贯度（用户实测 40 帧
//! 跳跃幅度大）；全片真解码（关键帧复制无信息增量），解码成本经**导入期异步预抽**
//! 摊付（unpack_queue 挂钩，Eagle 毫秒级手感的来源即"帧图提前就绪"），悬停命令补抽
//! 覆盖存量 / resolve_conflicts "copy" 路径 / 预抽失败。生成中防重（running 集合）：
//! 重入返 Pending（前端维持 video seek 兜底，下次再取）。

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

use crate::adapters::ffmpeg;
use crate::core::json_io::read_json;
use crate::core::library::CACHE_DIR;
use crate::core::models::AssetMeta;
use crate::core::library::ASSTES_DIR;

/// 帧高（480 高 ≈ 30KB/帧，卡片显示足够清晰）
const SCRUB_FRAME_HEIGHT: u32 = 480;
/// 短 / 长档分界（秒）与采样率：短视频 4fps（动作连贯），长视频 0.5fps（2 秒一帧）
const SHORT_CLIP_MAX_SECS: f64 = 30.0;
const RATE_SHORT: f64 = 4.0;
const RATE_LONG: f64 = 0.5;
/// 帧数上下限（防超短视频帧太少、长视频体积膨胀）
const MIN_FRAMES: u32 = 8;
const MAX_FRAMES: u32 = 150;

/// 抽帧数量（纯函数）：按时长分档，8~150 封顶
pub fn scrub_frame_count(duration_secs: f64) -> u32 {
  if duration_secs <= 0.0 {
    return MIN_FRAMES;
  }
  let rate = if duration_secs <= SHORT_CLIP_MAX_SECS { RATE_SHORT } else { RATE_LONG };
  ((duration_secs * rate).round() as u32).clamp(MIN_FRAMES, MAX_FRAMES)
}

/// 生成中登记（防重入：导入预抽 / 悬停命令 / 快速滚动并发触发同资产抽帧，ffmpeg 全片解码不可叠加）
static SCRUB_RUNNING: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn running() -> &'static Mutex<HashSet<String>> {
  SCRUB_RUNNING.get_or_init(|| Mutex::new(HashSet::new()))
}

/// ensure 结果
#[derive(Debug, PartialEq, Eq)]
pub enum ScrubOutcome {
  /// 帧图绝对路径清单（按名排序）
  Ready(Vec<String>),
  /// 另一调用正在生成（前端维持 video seek 兜底，下次悬停再取）
  Pending,
}

/// 已缓存帧清单（帧目录非空 + 源戳匹配）；未命中 None。命中判定先于 ffmpeg 存在性检查（同转码口径）
pub fn cached(root: &Path, asset_id: &str, origin: &Path) -> Option<Vec<String>> {
  let dir = root.join(CACHE_DIR).join("scrub").join(asset_id);
  let frames = cached_frames(&dir)?;
  if source_stamp(origin).is_some_and(|s| read_stamp(&dir.join(".stamp")) == Some(s)) {
    Some(frames)
  } else {
    None
  }
}

/// 命中直给、未命中同步生成（防重：生成中重入返 Pending）。命令层用
pub fn ensure(root: &Path, asset_id: &str, origin: &Path) -> Result<ScrubOutcome, String> {
  if let Some(frames) = cached(root, asset_id, origin) {
    return Ok(ScrubOutcome::Ready(frames));
  }
  {
    let mut r = running().lock().map_err(|e| e.to_string())?;
    if !r.insert(asset_id.to_string()) {
      return Ok(ScrubOutcome::Pending);
    }
  }
  let result = generate(root, asset_id, origin);
  if let Ok(mut r) = running().lock() {
    r.remove(asset_id);
  }
  result.map(ScrubOutcome::Ready)
}

/// 按资产 ID 预抽（导入期队列挂钩用）：origin 由 info/metadata.json 推导（worker 无 lib 句柄，
/// 同 anim_unpack 口径）；已缓存直接 Ok（幂等），失败中文原因（调用方记日志，资产仍可用）
pub fn generate_by_id(root: &Path, asset_id: &str) -> Result<Vec<String>, String> {
  let info_dir = root.join(ASSTES_DIR).join(format!("{asset_id}.info"));
  let meta: AssetMeta = read_json(&info_dir.join("metadata.json")).map_err(|e| e.to_string())?;
  let origin = info_dir.join(&meta.origin_name);
  if let Some(frames) = cached(root, asset_id, &origin) {
    return Ok(frames);
  }
  // 与 ensure 同一防重集合：导入预抽与悬停命令撞车时后者返 Pending
  {
    let mut r = running().lock().map_err(|e| e.to_string())?;
    if !r.insert(asset_id.to_string()) {
      return Ok(Vec::new()); // 另一路正在生成，本任务视为完成（结果等价）
    }
  }
  let result = generate(root, asset_id, &origin);
  if let Ok(mut r) = running().lock() {
    r.remove(asset_id);
  }
  result
}

/// 生成主体：probe 时长 → 整目录清旧帧 → ffmpeg 抽帧 → 写源戳。<2 帧视为失败
/// （单帧无 scrub 意义，且前端按 len-1 插值会 0/0）
fn generate(root: &Path, asset_id: &str, origin: &Path) -> Result<Vec<String>, String> {
  if !ffmpeg::ffmpeg_available() {
    return Err("音视频组件缺失，无法生成预览帧".to_string());
  }
  let duration = ffmpeg::probe_json(origin)
    .and_then(|j| j.get("format")?.get("duration")?.as_str()?.parse::<f64>().ok())
    .ok_or_else(|| "视频时长探测失败".to_string())?;
  let dir = root.join(CACHE_DIR).join("scrub").join(asset_id);
  // 先整目录清旧帧：防源变化后新旧帧混杂 / 旧帧数更多残留尾部
  let _ = fs::remove_dir_all(&dir);
  fs::create_dir_all(&dir).map_err(|e| format!("预览帧缓存目录创建失败：{e}"))?;
  let files = ffmpeg::video_scrub_frames(origin, &dir, duration, scrub_frame_count(duration), SCRUB_FRAME_HEIGHT)
    .filter(|f| f.len() >= 2)
    .ok_or_else(|| "预览帧生成失败".to_string())?;
  if let Some(s) = source_stamp(origin) {
    if let Err(e) = fs::write(dir.join(".stamp"), s) {
      log::warn!("预览帧戳写入失败（{}）：{e}", dir.display());
    }
  }
  Ok(files.iter().map(|p| p.to_string_lossy().into_owned()).collect())
}

/// 已缓存帧清单（f-*.webp 按名排序）；目录不存在 / 无帧为 None
fn cached_frames(dir: &Path) -> Option<Vec<String>> {
  let mut names: Vec<String> = fs::read_dir(dir)
    .ok()?
    .filter_map(|e| e.ok())
    .map(|e| e.file_name().to_string_lossy().into_owned())
    .filter(|n| n.starts_with("f-") && n.ends_with(".webp"))
    .collect();
  if names.is_empty() {
    return None;
  }
  names.sort();
  Some(names.iter().map(|n| dir.join(n).to_string_lossy().into_owned()).collect())
}

/// 源文件戳："大小:mtime秒"（与产物 .stamp 比对判源是否变化）
fn source_stamp(origin: &Path) -> Option<String> {
  let meta = fs::metadata(origin).ok()?;
  let secs = meta.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
  Some(format!("{}:{secs}", meta.len()))
}

fn read_stamp(stamp: &Path) -> Option<String> {
  fs::read_to_string(stamp).ok().map(|t| t.trim().to_string())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn scrub_frame_count_tiers_by_duration() {
    // 短档 4fps：3s → 12、30s → 120；下限 8：0.5s → 8
    assert_eq!(scrub_frame_count(3.0), 12);
    assert_eq!(scrub_frame_count(30.0), 120);
    assert_eq!(scrub_frame_count(0.5), MIN_FRAMES);
    assert_eq!(scrub_frame_count(0.0), MIN_FRAMES);
    // 长档 0.5fps：60s → 30、185s → 93（92.5 半进）；上限 150：600s → 150
    assert_eq!(scrub_frame_count(60.0), 30);
    assert_eq!(scrub_frame_count(185.0), 93);
    assert_eq!(scrub_frame_count(600.0), MAX_FRAMES);
  }

  #[test]
  fn cached_requires_frames_and_matching_stamp() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let origin = root.join("源.mp4");
    fs::write(&origin, b"fake").unwrap();
    let scrub_dir = root.join(CACHE_DIR).join("scrub").join("AN-V");
    // 无目录 → None
    assert!(cached(root, "AN-V", &origin).is_none());
    // 有帧无戳 → None
    fs::create_dir_all(&scrub_dir).unwrap();
    fs::write(scrub_dir.join("f-001.webp"), b"x").unwrap();
    assert!(cached(root, "AN-V", &origin).is_none());
    // 有帧有匹配戳 → Some
    fs::write(scrub_dir.join(".stamp"), source_stamp(&origin).unwrap()).unwrap();
    assert_eq!(cached(root, "AN-V", &origin).map(|f| f.len()), Some(1));
    // 源变化（大小变）→ None
    fs::write(&origin, b"fake-longer").unwrap();
    assert!(cached(root, "AN-V", &origin).is_none());
  }

  #[test]
  fn ensure_reentrant_returns_pending() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let origin = root.join("源.mp4");
    fs::write(&origin, b"fake").unwrap();
    running().lock().unwrap().insert("AN-V2".to_string());
    let r = ensure(root, "AN-V2", &origin).unwrap();
    assert_eq!(r, ScrubOutcome::Pending);
    running().lock().unwrap().remove("AN-V2");
  }
}
