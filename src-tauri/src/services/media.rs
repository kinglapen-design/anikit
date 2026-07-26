//! 音视频元数据与预览图（5.1）：ffprobe / ffmpeg sidecar（adapters::ffmpeg）。
//! sidecar 缺失 / 解码失败一律返回 None（"能力探测"语义，同 thumbnail）——音视频退化为
//! 格式图标保底，导入不失败。波形图 = PCM 分桶峰值 → 320×320 透明底竖条图
//! （柱 #F8F5FA 60% alpha，2026-07-26 用户定；生成进 webp 后前端不可再调色）。

use std::path::Path;

use image::{DynamicImage, RgbaImage};

use crate::adapters::ffmpeg;

/// 视频格式（PRD 05-01 MVP 口径；与收件箱白名单同步）
pub const VIDEO_EXTS: &[&str] = &["mp4", "mov", "avi", "mpeg4", "m4v"];
/// 音频格式（PRD 05-01 MVP 口径；5.1 波形/元数据就位后扩白名单）
pub const AUDIO_EXTS: &[&str] = &["mp3", "wav", "wma"];

/// 波形 PCM 采样率（Hz，mono f32）：8kHz 足够描波形，3 分钟音频约 5.6MB 内存
const PCM_RATE: u32 = 8000;
/// 波形图边长（px，方形透明底，卡片 contain 显示）
const WAVEFORM_SIZE: u32 = 320;
/// 波形柱数：320 宽 / 4px 槽位（2px 柱 + 2px 间距）
const WAVEFORM_BARS: usize = 80;
/// 波形柱色（#F8F5FA 60% alpha，2026-07-26 用户定，与 font-icon-3 同档观感）
const BAR_RGBA: [u8; 4] = [0xF8, 0xF5, 0xFA, 153];
/// 波形上下留白（px）
const WAVEFORM_MARGIN: u32 = 10;

/// 视频元数据（导入写 metadata.width/height/duration）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VideoInfo {
  pub width: u32,
  pub height: u32,
  pub duration: f64,
}

/// 音频元数据（导入写 metadata.duration/bpm）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioInfo {
  pub duration: f64,
  pub bpm: Option<f64>,
}

fn ext_of(path: &Path) -> String {
  path.extension().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default()
}

pub fn is_video(path: &Path) -> bool {
  VIDEO_EXTS.contains(&ext_of(path).as_str())
}

pub fn is_audio(path: &Path) -> bool {
  AUDIO_EXTS.contains(&ext_of(path).as_str())
}

/// 视频元数据（ffprobe）：首个视频流宽高 + format.duration（秒）；非视频格式 / 失败返回 None
pub fn probe_video(path: &Path) -> Option<VideoInfo> {
  if !is_video(path) {
    return None;
  }
  let json = ffmpeg::probe_json(path)?;
  let streams = json.get("streams")?.as_array()?;
  let video = streams.iter().find(|s| s.get("codec_type").and_then(|t| t.as_str()) == Some("video"))?;
  let width = video.get("width")?.as_u64()? as u32;
  let height = video.get("height")?.as_u64()? as u32;
  let duration = json.get("format")?.get("duration")?.as_str()?.parse::<f64>().ok()?;
  Some(VideoInfo { width, height, duration })
}

/// 音频元数据（ffprobe）：format.duration（秒）+ 内嵌标签 BPM（TBPM/bpm 大小写不敏感，
/// 只读标签不做自动节拍检测，2026-07-26 用户定）；非音频格式 / 失败返回 None
pub fn probe_audio(path: &Path) -> Option<AudioInfo> {
  if !is_audio(path) {
    return None;
  }
  let json = ffmpeg::probe_json(path)?;
  let format = json.get("format")?;
  let duration = format.get("duration")?.as_str()?.parse::<f64>().ok()?;
  let bpm = format.get("tags")?.as_object().and_then(|tags| {
    tags
      .iter()
      .find(|(k, _)| matches!(k.to_lowercase().as_str(), "tbpm" | "bpm"))
      .and_then(|(_, v)| v.as_str()?.parse::<f64>().ok())
  });
  Some(AudioInfo { duration, bpm })
}

/// 视频首帧（0s）解码为 RGBA；缩略图与冲突弹窗预览共用
pub fn video_first_frame(path: &Path) -> Option<RgbaImage> {
  if !is_video(path) {
    return None;
  }
  let png = ffmpeg::video_frame_png(path, 0.0)?;
  image::load_from_memory(&png).ok().map(|i| i.to_rgba8())
}

/// 音频波形预览图（320×320 透明底竖条）；缩略图与冲突弹窗预览共用
pub fn waveform_image(path: &Path) -> Option<RgbaImage> {
  if !is_audio(path) {
    return None;
  }
  let samples = ffmpeg::audio_pcm_f32(path, PCM_RATE)?;
  let peaks = peaks_from_samples(&samples, WAVEFORM_BARS);
  Some(render_waveform(&peaks, WAVEFORM_SIZE, WAVEFORM_SIZE))
}

/// 预览图统一入口（thumbnail::load_origin 兜底调用）：视频首帧 / 音频波形；其他格式 None
pub fn decode_preview_image(path: &Path) -> Option<DynamicImage> {
  if is_video(path) {
    return video_first_frame(path).map(DynamicImage::ImageRgba8);
  }
  if is_audio(path) {
    return waveform_image(path).map(DynamicImage::ImageRgba8);
  }
  None
}

/// PCM 样本分桶取峰值并全局归一（0..=1）；空输入返回全零（静音波形仍出图，一条中线）
fn peaks_from_samples(samples: &[f32], buckets: usize) -> Vec<f32> {
  if samples.is_empty() || buckets == 0 {
    return vec![0.0; buckets];
  }
  let bucket_len = samples.len().div_ceil(buckets);
  let mut peaks: Vec<f32> = samples
    .chunks(bucket_len)
    .map(|c| c.iter().fold(0f32, |m, s| m.max(s.abs())))
    .collect();
  peaks.resize(buckets, 0.0);
  let max = peaks.iter().cloned().fold(0f32, f32::max);
  if max > 0.0 {
    for p in &mut peaks {
      *p /= max;
    }
  }
  peaks
}

/// 峰值数组渲染为透明底竖条图：柱 2px、槽位均分、垂直居中、上下留白 WAVEFORM_MARGIN
fn render_waveform(peaks: &[f32], w: u32, h: u32) -> RgbaImage {
  let mut img = RgbaImage::new(w, h);
  if peaks.is_empty() {
    return img;
  }
  let slot = (w / peaks.len() as u32).max(1);
  let bar_w = (slot / 2).max(1);
  let cy = h / 2;
  let max_half = (h / 2).saturating_sub(WAVEFORM_MARGIN);
  let color = image::Rgba(BAR_RGBA);
  for (i, &p) in peaks.iter().enumerate() {
    let x0 = i as u32 * slot + (slot - bar_w) / 2;
    let half = ((p.clamp(0.0, 1.0) * max_half as f32) as u32).max(1);
    for x in x0..(x0 + bar_w).min(w) {
      for y in (cy - half)..=(cy + half).min(h - 1) {
        img.put_pixel(x, y, color);
      }
    }
  }
  img
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn peaks_normalizes_and_buckets() {
    // 8 样本 4 桶：每桶 2 样本取 max abs → [0.5, 1.0, 0.25, 0.75] 归一 → [0.5, 1.0, 0.25, 0.75]（max=1）
    let samples = [0.1, -0.5, 0.9, -1.0, 0.2, -0.25, 0.7, -0.75];
    let peaks = peaks_from_samples(&samples, 4);
    assert_eq!(peaks, vec![0.5, 1.0, 0.25, 0.75]);
  }

  #[test]
  fn peaks_empty_and_zero_buckets() {
    assert_eq!(peaks_from_samples(&[], 4), vec![0.0; 4]);
    assert_eq!(peaks_from_samples(&[1.0], 0), Vec::<f32>::new());
    // 桶多于样本：后补零
    assert_eq!(peaks_from_samples(&[0.5], 3), vec![1.0, 0.0, 0.0]);
  }

  #[test]
  fn render_waveform_draws_centered_bars() {
    let mut peaks = vec![0.0; WAVEFORM_BARS];
    peaks[0] = 1.0;
    let img = render_waveform(&peaks, WAVEFORM_SIZE, WAVEFORM_SIZE);
    assert_eq!((img.width(), img.height()), (WAVEFORM_SIZE, WAVEFORM_SIZE));
    // 第 1 柱满幅：槽位 4px 内柱占 x=1..=2（居中），中心行 x=1 处有柱色
    assert_eq!(img.get_pixel(1, WAVEFORM_SIZE / 2).0, BAR_RGBA);
    // 满幅柱顶 = margin
    assert_eq!(img.get_pixel(1, WAVEFORM_MARGIN).0, BAR_RGBA);
    // 第 2 柱（峰值 0）只有中线 2px：y=0 处透明
    assert_eq!(img.get_pixel(4, 0).0[3], 0);
  }

  #[test]
  fn ext_gates_prevent_spawning_sidecar() {
    // 非音视频扩展名直接 None（不启动 ffprobe）
    let txt = std::path::Path::new("文档.txt");
    assert_eq!(probe_video(txt), None);
    assert_eq!(probe_audio(txt), None);
    assert!(decode_preview_image(txt).is_none());
  }

  /// 需要真二进制 + 真媒体文件：ANIKIT_TEST_VIDEO=某mp4 ANIKIT_TEST_AUDIO=某mp3
  /// cargo test -- --ignored media_probe --nocapture
  #[test]
  #[ignore]
  fn media_probe_smoke() {
    if let Ok(v) = std::env::var("ANIKIT_TEST_VIDEO") {
      let info = probe_video(Path::new(&v)).expect("视频元数据应成功");
      assert!(info.width > 0 && info.duration > 0.0);
      assert!(video_first_frame(Path::new(&v)).is_some());
    }
    if let Ok(a) = std::env::var("ANIKIT_TEST_AUDIO") {
      let info = probe_audio(Path::new(&a)).expect("音频元数据应成功");
      assert!(info.duration > 0.0);
      assert!(waveform_image(Path::new(&a)).is_some());
    }
  }
}
