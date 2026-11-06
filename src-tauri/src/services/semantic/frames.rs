use std::path::Path;

use crate::services::{anim, sequence};

use super::{control::AnalysisControl, sampling::{embed_window, FrameSample, FrameWindow, WindowSampler}, types::SegmentInput};

pub fn analyze_image(origin: &Path, work_dir: &Path, control: &AnalysisControl) -> Result<(Vec<SegmentInput>, String), String> {
  let sample = FrameSample::path(None, origin.to_path_buf(), 0, 0);
  embed_window(FrameWindow { samples: vec![sample] }, work_dir, control)
}

pub fn analyze_anim(origin: &Path, work_dir: &Path, control: &AnalysisControl) -> Result<(Vec<SegmentInput>, String), String> {
  let duration_ms = anim::probe(origin).map(|info| (info.duration_secs * 1000.0).round() as i64).filter(|duration| *duration > 0);
  let mut sampler = WindowSampler::new(duration_ms);
  let mut segments = Vec::new();
  let mut backend = "unknown".to_string();
  let mut elapsed = 0_i64;
  let mut failure = None;
  let decoded = anim::decode_frames(origin, |index, image, delay| {
    let result = (|| {
      control.check()?;
      let start_ms = elapsed;
      elapsed += i64::from(delay);
      let sample = FrameSample::pixels(index, image, start_ms, elapsed);
      let windows = sampler.push(sample)?;
      append_windows(windows, work_dir, control, &mut segments, &mut backend)
    })();
    if let Err(error) = result {
      failure = Some(error);
      return false;
    }
    true
  });
  if let Some(error) = failure {
    return Err(error);
  }
  if decoded.is_none() {
    return Err(format!("动图解码失败：{}", origin.display()));
  }
  append_windows(sampler.finish(), work_dir, control, &mut segments, &mut backend)?;
  Ok((segments, backend))
}

pub fn analyze_sequence(frames_dir: &Path, fps: f64, work_dir: &Path, control: &AnalysisControl) -> Result<(Vec<SegmentInput>, String), String> {
  let frames = sequence::ordered_frame_paths(frames_dir)?;
  if frames.is_empty() {
    return Err("序列帧没有可用帧".into());
  }
  if !fps.is_finite() || fps <= 0.0 {
    return Err("序列帧帧率无效".into());
  }
  let duration_ms = (frames.len() as f64 * 1000.0 / fps).round() as i64;
  let mut sampler = WindowSampler::new(Some(duration_ms));
  let mut segments = Vec::new();
  let mut backend = "unknown".to_string();
  for (index, path) in frames.into_iter().enumerate() {
    control.check()?;
    let index = u32::try_from(index).map_err(|_| "序列帧数量超出索引上限".to_string())?;
    let start_ms = (f64::from(index) * 1000.0 / fps).round() as i64;
    let end_ms = (f64::from(index + 1) * 1000.0 / fps).round() as i64;
    let sample = FrameSample::path(Some(index), path, start_ms, end_ms);
    let windows = sampler.push(sample)?;
    append_windows(windows, work_dir, control, &mut segments, &mut backend)?;
  }
  append_windows(sampler.finish(), work_dir, control, &mut segments, &mut backend)?;
  Ok((segments, backend))
}

fn append_windows(windows: Vec<FrameWindow>, work_dir: &Path, control: &AnalysisControl, segments: &mut Vec<SegmentInput>, backend: &mut String) -> Result<(), String> {
  for window in windows {
    let (embedded, actual) = embed_window(window, work_dir, control)?;
    if !embedded.is_empty() {
      segments.extend(embedded);
      *backend = actual;
    }
  }
  Ok(())
}
