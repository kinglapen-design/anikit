use std::{collections::HashSet, fs, path::Path};

use crate::{adapters::ffmpeg, core::models::AssetMeta};

use super::{control::AnalysisControl, sampling::{embed_window, sample_times, video_windows, FrameSample, FrameWindow}, types::SegmentInput};

pub fn analyze(origin: &Path, meta: &AssetMeta, work_dir: &Path, control: &AnalysisControl) -> Result<(Vec<SegmentInput>, String), String> {
  let duration = meta.duration.or_else(|| probe_duration(origin)).ok_or_else(|| format!("无法读取视频时长：{}", origin.display()))?;
  let duration_ms = (duration * 1000.0).round() as i64;
  if duration_ms <= 0 {
    return Err("视频时长无效".into());
  }
  let cuts = ffmpeg::scene_cut_times(origin, &control.cancel_flag()).unwrap_or_default();
  let mut segments = Vec::new();
  let mut backend = "unknown".to_string();
  for (start, end) in video_windows(duration_ms, &cuts) {
    let mut seen_pts = HashSet::new();
    let mut samples = Vec::new();
    for at in sample_times(start, end) {
      control.check()?;
      let (png, pts) = ffmpeg::video_frame_png_precise(origin, at as f64 / 1000.0, &control.cancel_flag()).ok_or_else(|| format!("视频抽帧失败：{at}ms"))?;
      let actual_ms = (pts * 1000.0).round() as i64;
      if actual_ms < 0 || !seen_pts.insert(actual_ms) {
        continue;
      }
      let path = work_dir.join(format!("frame-{start}-{at}.png"));
      fs::write(&path, png).map_err(|e| format!("写入抽帧失败：{e}"))?;
      samples.push(FrameSample::path(None, path, actual_ms, end.max(actual_ms)));
    }
    let (embedded, actual_backend) = embed_window(FrameWindow { samples }, work_dir, control)?;
    segments.extend(embedded);
    backend = actual_backend;
  }
  Ok((segments, backend))
}

fn probe_duration(path: &Path) -> Option<f64> {
  let json = ffmpeg::probe_json(path)?;
  json.get("format")?.get("duration")?.as_str()?.parse().ok()
}
