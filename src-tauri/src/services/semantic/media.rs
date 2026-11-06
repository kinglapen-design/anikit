use std::{fs, path::{Path, PathBuf}};

use sha2::{Digest, Sha256};

use crate::{core::models::AssetMeta, services::{anim, media, sequence}};

use super::{control::AnalysisControl, frames, types::SegmentInput, video};

pub const ANALYSIS_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
  Video,
  Anim,
  Sequence,
  Image,
}

impl MediaKind {
  pub fn as_str(self) -> &'static str {
    match self {
      Self::Video => "video",
      Self::Anim => "anim",
      Self::Sequence => "sequence",
      Self::Image => "image",
    }
  }
}

pub fn is_static_image_ext(ext: &str) -> bool {
  matches!(ext.to_ascii_lowercase().as_str(), "png" | "jpg" | "jpeg" | "webp" | "bmp")
}

pub struct MediaAnalysis {
  pub kind: MediaKind,
  pub fingerprint: String,
  pub segments: Vec<SegmentInput>,
  pub actual_backend: String,
}

pub fn analyze_asset(meta: &AssetMeta, assets_dir: &Path, asset_id: &str, work_dir: &Path, control: &AnalysisControl) -> Result<MediaAnalysis, String> {
  let info_dir = crate::core::paths::info_dir(assets_dir, asset_id);
  let origin = info_dir.join(&meta.origin_name);
  let fingerprint = source_fingerprint(meta, assets_dir, asset_id)?;
  let _guard = TempGuard::new(work_dir)?;
  let kind = if sequence::frames_dir_if_sequence(assets_dir, asset_id, meta).is_some() {
    MediaKind::Sequence
  } else if anim::is_animated_file(meta, &origin) {
    MediaKind::Anim
  } else if media::is_video(&origin) {
    MediaKind::Video
  } else if is_static_image_ext(&meta.ext) {
    MediaKind::Image
  } else {
    return Err(format!("「{}」不是可分析的视频、动图、序列帧或静态图片", meta.name));
  };
  let (segments, actual_backend) = match kind {
    MediaKind::Sequence => frames::analyze_sequence(&info_dir.join("frames"), meta.fps.unwrap_or(sequence::DEFAULT_FPS), work_dir, control)?,
    MediaKind::Anim => frames::analyze_anim(&origin, work_dir, control)?,
    MediaKind::Video => video::analyze(&origin, meta, work_dir, control)?,
    MediaKind::Image => frames::analyze_image(&origin, work_dir, control)?,
  };
  if segments.is_empty() {
    return Err(format!("「{}」没有可分析的画面", meta.name));
  }
  Ok(MediaAnalysis { kind, fingerprint, segments, actual_backend })
}

pub fn source_fingerprint(meta: &AssetMeta, assets_dir: &Path, asset_id: &str) -> Result<String, String> {
  fingerprint(&crate::core::paths::info_dir(assets_dir, asset_id).join(&meta.origin_name), meta)
}

fn fingerprint(origin: &Path, meta: &AssetMeta) -> Result<String, String> {
  let mut hasher = Sha256::new();
  hasher.update(meta.hash.as_bytes());
  hasher.update(meta.fps.unwrap_or(0.0).to_bits().to_le_bytes());
  if origin.is_file() {
    let stat = fs::metadata(origin).map_err(|e| format!("读取源文件状态失败：{e}"))?;
    hash_stat(&mut hasher, &stat);
  }
  let frames_dir = origin.parent().map(|parent| parent.join("frames")).filter(|path| path.is_dir());
  if let Some(frames_dir) = frames_dir {
    for frame in sequence::ordered_frame_paths(&frames_dir)? {
      hasher.update(frame.file_name().unwrap_or_default().to_string_lossy().as_bytes());
      let stat = fs::metadata(&frame).map_err(|e| format!("读取序列帧指纹失败：{e}"))?;
      hash_stat(&mut hasher, &stat);
    }
  }
  Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn hash_stat(hasher: &mut Sha256, stat: &fs::Metadata) {
  hasher.update(stat.len().to_le_bytes());
  if let Some(duration) = stat.modified().ok().and_then(|modified| modified.duration_since(std::time::UNIX_EPOCH).ok()) {
    hasher.update(duration.as_secs().to_le_bytes());
    hasher.update(duration.subsec_nanos().to_le_bytes());
  }
}

struct TempGuard {
  path: PathBuf,
}

impl TempGuard {
  fn new(path: &Path) -> Result<Self, String> {
    fs::create_dir_all(path).map_err(|e| format!("创建分析临时目录失败：{e}"))?;
    Ok(Self { path: path.to_path_buf() })
  }
}

impl Drop for TempGuard {
  fn drop(&mut self) {
    let _ = fs::remove_dir_all(&self.path);
  }
}
