use std::{io::Cursor, path::Path};

use base64::{Engine, engine::general_purpose::STANDARD};
use image::{DynamicImage, RgbaImage};

use crate::{adapters::ffmpeg, core::models::AssetMeta, services::{anim, media, sequence}};

use super::{control::AnalysisControl, media::source_fingerprint, provider_http::EvidenceImage, types::SegmentHit};

const MAX_FRAMES: usize = 24;
const MAX_EDGE: u32 = 768;

pub struct EvidenceSet {
  pub frames: Vec<EvidenceImage>,
  pub duration_ms: i64,
}

pub fn collect(meta: &AssetMeta, assets_dir: &Path, asset_id: &str, segments: &[SegmentHit], expected_fingerprint: &str, control: &AnalysisControl) -> Result<EvidenceSet, String> {
  if source_fingerprint(meta, assets_dir, asset_id)? != expected_fingerprint {
    return Err("源文件已变化，请先重新进行基础分析".into());
  }
  let mut visual = segments.iter().filter(|segment| segment.kind == "visual").collect::<Vec<_>>();
  visual.sort_by_key(|segment| (segment.start_ms, segment.segment_id));
  if visual.is_empty() {
    return Err("请先完成基础画面分析，再进行详细分析".into());
  }
  let indexes = evenly_indexes(visual.len(), MAX_FRAMES);
  let origin = crate::core::paths::info_dir(assets_dir, asset_id).join(&meta.origin_name);
  let sequence_dir = sequence::frames_dir_if_sequence(assets_dir, asset_id, meta);
  let mut frames = Vec::new();
  for index in indexes {
    control.check()?;
    let segment = visual[index];
    let (image, timestamp_ms) = if let Some(frames_dir) = &sequence_dir {
      let frame = segment.start_frame.ok_or_else(|| "序列帧命中缺少原始帧索引".to_string())?;
      let paths = sequence::ordered_frame_paths(frames_dir)?;
      let path = paths.get(frame as usize).ok_or_else(|| "序列帧命中索引已失效，请重新分析".to_string())?;
      (image::open(path).map_err(|e| format!("读取序列帧失败：{e}"))?.to_rgba8(), segment.start_ms)
    } else if anim::is_animated_file(meta, &origin) {
      let index = segment.start_frame.ok_or_else(|| "动图命中缺少原始帧索引".to_string())?;
      let (image, _) = anim::decode_frame(&origin, index).ok_or_else(|| "动图命中帧解码失败，请重新分析".to_string())?;
      (image, segment.start_ms)
    } else if media::is_video(&origin) {
      let (png, pts) = ffmpeg::video_frame_png_precise(&origin, segment.start_ms as f64 / 1000.0, &control.cancel_flag()).ok_or_else(|| "详细分析抽帧失败".to_string())?;
      (image::load_from_memory(&png).map_err(|e| format!("读取视频帧失败：{e}"))?.to_rgba8(), (pts * 1000.0).round() as i64)
    } else {
      return Err("该素材不支持详细画面分析".into());
    };
    frames.push(EvidenceImage {
      timestamp_ms,
      source_frame: segment.start_frame,
      png_base64: STANDARD.encode(encode_png(&image)?),
    });
  }
  let visual_end_ms = visual.iter().map(|segment| segment.end_ms).max().unwrap_or(0);
  Ok(EvidenceSet { frames, duration_ms: media_duration_ms(meta.duration, visual_end_ms) })
}

fn media_duration_ms(duration: Option<f64>, visual_end_ms: i64) -> i64 {
  duration.filter(|seconds| seconds.is_finite() && *seconds > 0.0)
    .map(|seconds| (seconds * 1000.0).round() as i64)
    .unwrap_or(visual_end_ms)
}

fn evenly_indexes(total: usize, limit: usize) -> Vec<usize> {
  if total <= limit {
    return (0..total).collect();
  }
  (0..limit).map(|index| index * (total - 1) / (limit - 1)).collect()
}

fn encode_png(image: &RgbaImage) -> Result<Vec<u8>, String> {
  let scale = MAX_EDGE as f32 / image.width().max(image.height()).max(1) as f32;
  let width = ((image.width() as f32 * scale.min(1.0)).round() as u32).max(1);
  let height = ((image.height() as f32 * scale.min(1.0)).round() as u32).max(1);
  let resized = image::imageops::resize(image, width, height, image::imageops::FilterType::Lanczos3);
  let mut bytes = Cursor::new(Vec::new());
  DynamicImage::ImageRgba8(resized).write_to(&mut bytes, image::ImageFormat::Png).map_err(|e| format!("编码证据帧失败：{e}"))?;
  Ok(bytes.into_inner())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn sampling_keeps_first_middle_and_last_frames() {
    assert_eq!(evenly_indexes(4, 24), vec![0, 1, 2, 3]);
    assert_eq!(evenly_indexes(100, 4), vec![0, 33, 66, 99]);
    assert_eq!(evenly_indexes(1, 24), vec![0]);
  }

  #[test]
  fn png_encoding_preserves_aspect_ratio_and_bounds() {
    let image = RgbaImage::new(1600, 800);
    let resized = image::load_from_memory(&encode_png(&image).unwrap()).unwrap();
    assert_eq!((resized.width(), resized.height()), (768, 384));
    let small = RgbaImage::new(32, 16);
    let resized = image::load_from_memory(&encode_png(&small).unwrap()).unwrap();
    assert_eq!((resized.width(), resized.height()), (32, 16));
  }

  #[test]
  fn duration_uses_media_metadata_before_sampled_ranges() {
    assert_eq!(media_duration_ms(Some(12.3456), 9000), 12346);
    assert_eq!(media_duration_ms(None, 9000), 9000);
    assert_eq!(media_duration_ms(Some(0.0), 9000), 9000);
    assert_eq!(media_duration_ms(Some(f64::NAN), 9000), 9000);
  }
}
