use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use crate::adapters::ffmpeg::{self, FrameClipFormat};
use crate::core::models::AssetMeta;
use crate::services::{anim, media, sequence};

use super::{media::MediaKind, playback, types::SegmentRange};

pub struct ExportedClip {
  pub path: PathBuf,
  pub extension: &'static str,
}

pub fn export(source: &AssetMeta, assets_dir: &Path, range: &SegmentRange, work_dir: &Path, name: &str, cancel: &AtomicBool) -> Result<ExportedClip, String> {
  let playback = playback::resolve(assets_dir, &source.id)?;
  fs::create_dir_all(work_dir).map_err(|e| format!("创建片段临时目录失败：{e}"))?;
  let kind = media_kind(source, assets_dir, &playback.origin)?;
  match kind {
    MediaKind::Video => {
      let fps = source.fps.filter(|fps| fps.is_finite() && *fps > 0.0).unwrap_or(24.0);
      let duration = if range.duration_secs() > 0.0 { range.duration_secs() } else { 1.0 / fps };
      let output = work_dir.join(format!("{name}.mp4"));
      ffmpeg::clip_video(&playback.origin, &output, range.start_ms as f64 / 1000.0, duration, cancel)?;
      Ok(ExportedClip { path: output, extension: "mp4" })
    }
    MediaKind::Anim | MediaKind::Sequence => {
      let frames = selected_frames(source, assets_dir, &playback.origin, range, work_dir, kind)?;
      let format = if kind == MediaKind::Sequence {
        FrameClipFormat::Gif
      } else {
        match source.ext.to_ascii_lowercase().as_str() {
          "webp" => FrameClipFormat::Webp,
          "png" => FrameClipFormat::Apng,
          _ => FrameClipFormat::Gif,
        }
      };
      let extension = match format {
        FrameClipFormat::Gif => "gif",
        FrameClipFormat::Webp => "webp",
        FrameClipFormat::Apng => "png",
      };
      let script = write_concat(work_dir, &frames)?;
      let duration = frames.iter().map(|(_, delay)| u64::from(*delay)).sum::<u64>() as f64 / 1000.0;
      let output = work_dir.join(format!("{name}.{extension}"));
      ffmpeg::clip_frames(&script, &output, format, duration, cancel)?;
      Ok(ExportedClip { path: output, extension })
    }
  }
}

pub fn write_evidence_frame(source: &AssetMeta, assets_dir: &Path, range: &SegmentRange, work_dir: &Path, cancel: &AtomicBool) -> Result<PathBuf, String> {
  let playback = playback::resolve(assets_dir, &source.id)?;
  fs::create_dir_all(work_dir).map_err(|e| format!("创建片段封面目录失败：{e}"))?;
  let path = work_dir.join("cover.png");
  let kind = media_kind(source, assets_dir, &playback.origin)?;
  match kind {
    MediaKind::Video => {
      let (png, _) = ffmpeg::video_frame_png_precise(&playback.origin, range.start_ms as f64 / 1000.0, cancel)
        .ok_or_else(|| "无法读取片段起点的视频画面".to_string())?;
      fs::write(&path, png).map_err(|e| format!("写入片段封面失败：{e}"))?;
    }
    MediaKind::Anim => {
      let index = range.frame_bounds()?.0;
      let (image, _) = anim::decode_frame(&playback.origin, index).ok_or_else(|| "无法读取片段起点的动图原始帧".to_string())?;
      image.save(&path).map_err(|e| format!("写入片段封面失败：{e}"))?;
    }
    MediaKind::Sequence => {
      let index = range.frame_bounds()?.0 as usize;
      let frames_dir = sequence::frames_dir_if_sequence(assets_dir, &source.id, source).ok_or_else(|| "序列帧目录不存在".to_string())?;
      let frames = sequence::ordered_frame_paths(&frames_dir)?;
      let frame = frames.get(index).ok_or_else(|| "序列帧原始索引已失效，请重新分析".to_string())?;
      fs::copy(frame, &path).map_err(|e| format!("读取序列帧封面失败：{e}"))?;
    }
  }
  Ok(path)
}

pub(crate) fn media_kind(source: &AssetMeta, assets_dir: &Path, origin: &Path) -> Result<MediaKind, String> {
  if sequence::frames_dir_if_sequence(assets_dir, &source.id, source).is_some() {
    Ok(MediaKind::Sequence)
  } else if anim::is_animated_file(source, origin) {
    Ok(MediaKind::Anim)
  } else if media::is_video(origin) {
    Ok(MediaKind::Video)
  } else {
    Err("只能从视频、动图或序列帧创建片段".into())
  }
}

fn selected_frames(source: &AssetMeta, assets_dir: &Path, origin: &Path, range: &SegmentRange, work_dir: &Path, kind: MediaKind) -> Result<Vec<(PathBuf, u32)>, String> {
  let (start, end) = range.frame_bounds()?;
  if kind == MediaKind::Sequence {
    let dir = sequence::frames_dir_if_sequence(assets_dir, &source.id, source).ok_or_else(|| "序列帧目录不存在".to_string())?;
    let paths = sequence::ordered_frame_paths(&dir)?;
    let delay = (1000.0 / source.fps.unwrap_or(sequence::DEFAULT_FPS)).round().max(1.0) as u32;
    return (start..=end).map(|index| {
      paths.get(index as usize).map(|path| (path.clone(), delay)).ok_or_else(|| "序列帧原始索引已失效，请重新分析".to_string())
    }).collect();
  }
  let mut frames = Vec::new();
  let mut failure = None;
  let decoded = anim::decode_frames(origin, |index, image, delay| {
    if index < start {
      return true;
    }
    if index > end {
      return false;
    }
    let path = work_dir.join(format!("frame-{index}.png"));
    if let Err(error) = image.save(&path) {
      failure = Some(format!("保存截取的原始帧失败：{error}"));
      return false;
    }
    frames.push((path, delay.max(1)));
    true
  });
  if decoded.is_none() || frames.is_empty() {
    return Err("无法按原始帧索引读取动图".into());
  }
  if let Some(error) = failure {
    return Err(error);
  }
  Ok(frames)
}

fn write_concat(work_dir: &Path, frames: &[(PathBuf, u32)]) -> Result<PathBuf, String> {
  let mut text = String::from("ffconcat version 1.0\n");
  for (path, delay) in frames {
    writeln!(text, "file '{}'", concat_path(path)).map_err(|e| e.to_string())?;
    writeln!(text, "duration {:.6}", f64::from(*delay) / 1000.0).map_err(|e| e.to_string())?;
  }
  let last = frames.last().ok_or_else(|| "片段没有可导出的帧".to_string())?;
  writeln!(text, "file '{}'", concat_path(&last.0)).map_err(|e| e.to_string())?;
  let path = work_dir.join("frames.concat");
  fs::write(&path, text).map_err(|e| format!("写入帧时间轴失败：{e}"))?;
  Ok(path)
}

fn concat_path(path: &Path) -> String {
  path.to_string_lossy().replace('\\', "/").replace('\'', "'\\''")
}
