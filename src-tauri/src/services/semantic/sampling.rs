use std::{collections::HashSet, path::{Path, PathBuf}, sync::Arc};

use image::RgbaImage;

use super::{control::AnalysisControl, types::SegmentInput, worker};

pub const WINDOW_MS: i64 = 4000;
pub const STEP_MS: i64 = 3000;

#[derive(Clone)]
pub struct FrameSample {
  pub index: Option<u32>,
  pub start_ms: i64,
  pub end_ms: i64,
  pub path: PathBuf,
  pub pixels: Option<Arc<RgbaImage>>,
}

impl FrameSample {
  pub fn path(index: Option<u32>, path: PathBuf, start_ms: i64, end_ms: i64) -> Self {
    Self { index, start_ms, end_ms, path, pixels: None }
  }

  pub fn pixels(index: u32, image: RgbaImage, start_ms: i64, end_ms: i64) -> Self {
    Self { index: Some(index), start_ms, end_ms, path: PathBuf::new(), pixels: Some(Arc::new(image)) }
  }

  pub fn materialize(&self, work_dir: &Path) -> Result<PathBuf, String> {
    let Some(image) = &self.pixels else { return Ok(self.path.clone()) };
    let path = work_dir.join(format!("frame-{}.png", self.index.unwrap_or(0)));
    image.save(&path).map_err(|e| format!("写入动图帧失败：{e}"))?;
    Ok(path)
  }
}

pub struct FrameWindow {
  pub samples: Vec<FrameSample>,
}

struct OpenWindow {
  start_ms: i64,
  end_ms: i64,
  selected: [Option<FrameSample>; 3],
  last: Option<FrameSample>,
}

impl OpenWindow {
  fn new(start_ms: i64, end_ms: i64) -> Self {
    Self { start_ms, end_ms, selected: [None, None, None], last: None }
  }

  fn add(&mut self, sample: FrameSample) {
    if sample.start_ms >= self.end_ms || sample.end_ms < self.start_ms {
      return;
    }
    let targets = [self.start_ms, self.start_ms + (self.end_ms - self.start_ms) / 2, self.end_ms - 1];
    for (index, target) in targets.into_iter().enumerate() {
      if self.selected[index].is_none() && sample.start_ms >= target {
        let visible = if sample.start_ms == target { sample.clone() } else { self.last.clone().unwrap_or_else(|| sample.clone()) };
        self.selected[index] = Some(visible);
      }
    }
    self.last = Some(sample);
  }

  fn finish(self, duration_ms: i64) -> FrameWindow {
    let end_ms = self.end_ms.min(duration_ms);
    let targets = [self.start_ms, self.start_ms + (end_ms - self.start_ms) / 2, end_ms - 1];
    let mut seen = HashSet::new();
    let mut samples = Vec::new();
    for (index, _) in targets.into_iter().enumerate() {
      let sample = self.selected[index].clone().or_else(|| self.last.clone());
      if let Some(sample) = sample.filter(|sample| sample.start_ms < end_ms && seen.insert(sample.index)) {
        samples.push(sample);
      }
    }
    FrameWindow { samples }
  }
}

#[derive(Default)]
pub struct WindowSampler {
  windows: Vec<OpenWindow>,
  next_start: i64,
  previous: Option<FrameSample>,
  duration_ms: Option<i64>,
}

impl WindowSampler {
  pub fn new(duration_ms: Option<i64>) -> Self {
    Self { duration_ms, ..Self::default() }
  }

  pub fn push(&mut self, sample: FrameSample) -> Result<Vec<FrameWindow>, String> {
    if sample.start_ms < 0 || sample.end_ms < sample.start_ms {
      return Err("帧时间区间无效".into());
    }
    if self.previous.as_ref().is_some_and(|previous| sample.start_ms < previous.start_ms) {
      return Err("帧时间轴必须按先后顺序提供".into());
    }
    while self.next_start <= sample.start_ms {
      let start = self.next_start;
      self.next_start += STEP_MS;
      let end = self.duration_ms.map_or(start + WINDOW_MS, |duration| (start + WINDOW_MS).min(duration));
      let mut window = OpenWindow::new(start, end);
      if let Some(previous) = &self.previous {
        if previous.start_ms < window.end_ms && previous.end_ms > window.start_ms {
          window.last = Some(previous.clone());
        }
      }
      self.windows.push(window);
    }
    for window in &mut self.windows {
      window.add(sample.clone());
    }
    self.previous = Some(sample.clone());
    let mut finished = Vec::new();
    while self.windows.first().is_some_and(|window| window.end_ms <= sample.start_ms) {
      let window = self.windows.remove(0);
      let end_ms = window.end_ms;
      finished.push(window.finish(end_ms));
    }
    Ok(finished)
  }

  pub fn finish(self) -> Vec<FrameWindow> {
    let duration = self.previous.as_ref().map_or(0, |sample| sample.end_ms);
    self.windows.into_iter().map(|window| window.finish(duration)).collect()
  }
}

pub fn video_windows(duration_ms: i64, cuts: &[f64]) -> Vec<(i64, i64)> {
  let mut windows = Vec::new();
  let mut start = 0;
  while start < duration_ms {
    let end = (start + WINDOW_MS).min(duration_ms);
    let mut boundaries = vec![start, end];
    boundaries.extend(cuts.iter().map(|cut| (cut * 1000.0).round() as i64).filter(|cut| *cut > start && *cut < end));
    boundaries.sort_unstable();
    boundaries.dedup();
    for pair in boundaries.windows(2) {
      if pair[1] > pair[0] {
        windows.push((pair[0], pair[1]));
      }
    }
    start += STEP_MS;
  }
  windows
}

pub fn sample_times(start_ms: i64, end_ms: i64) -> Vec<i64> {
  let mut times = vec![start_ms, start_ms + (end_ms - start_ms) / 2, (end_ms - 1).max(start_ms)];
  times.dedup();
  times
}

pub fn embed_window(window: FrameWindow, work_dir: &Path, control: &AnalysisControl) -> Result<(Vec<SegmentInput>, String), String> {
  let mut segments = Vec::new();
  let mut backend = "unknown".to_string();
  for sample in window.samples {
    control.check()?;
    let path = sample.materialize(work_dir)?;
    let response = worker::embed_image(&path, Some(control.cancel_flag().as_ref()))?;
    backend = response.actual_backend;
    segments.push(SegmentInput {
      start_ms: sample.start_ms,
      end_ms: sample.end_ms,
      start_frame: sample.index,
      end_frame: sample.index,
      kind: "visual".into(),
      text: format!("{}ms–{}ms", sample.start_ms, sample.end_ms),
      vector: response.embedding,
    });
  }
  Ok((segments, backend))
}

#[cfg(test)]
mod tests {
  use super::*;

  fn frame(index: u32, start_ms: i64, end_ms: i64) -> FrameSample {
    FrameSample::path(Some(index), PathBuf::from(format!("{index}.png")), start_ms, end_ms)
  }

  #[test]
  fn windows_split_at_scene_cuts() {
    let windows = video_windows(9000, &[1.5, 6.2]);
    assert!(windows.contains(&(0, 1500)));
    assert!(windows.contains(&(1500, 4000)));
    assert!(windows.contains(&(6000, 6200)));
    assert!(windows.contains(&(6200, 9000)));
  }

  #[test]
  fn sample_times_never_seek_past_the_end() {
    assert_eq!(sample_times(0, 4000), vec![0, 2000, 3999]);
    assert_eq!(sample_times(100, 100), vec![100]);
  }

  #[test]
  fn variable_delay_frames_keep_their_own_indexes_and_times() {
    let mut sampler = WindowSampler::default();
    assert!(sampler.push(frame(0, 0, 100)).unwrap().is_empty());
    assert!(sampler.push(frame(1, 100, 2100)).unwrap().is_empty());
    assert!(sampler.push(frame(2, 2100, 4000)).unwrap().is_empty());
    let windows = sampler.finish();
    assert_eq!(windows.len(), 1);
    let indexes = windows[0].samples.iter().map(|sample| sample.index).collect::<Vec<_>>();
    assert_eq!(indexes, vec![Some(0), Some(1), Some(2)]);
    assert_eq!(windows[0].samples[1].start_ms, 100);
    assert_eq!(windows[0].samples[1].end_ms, 2100);
    assert_eq!(windows[0].samples[2].start_ms, 2100);
    assert_eq!(windows[0].samples[2].end_ms, 4000);
  }

  #[test]
  fn sequence_samples_do_not_duplicate_a_single_frame() {
    let mut sampler = WindowSampler::default();
    assert!(sampler.push(frame(0, 0, 42)).unwrap().is_empty());
    let windows = sampler.finish();
    assert_eq!(windows.len(), 1);
    assert_eq!(windows[0].samples.len(), 1);
    assert_eq!(windows[0].samples[0].index, Some(0));
  }
}
