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
/// 波形图宽（px，长方形透明底，Eagle 同形态约 2.35:1——2026-07-26 用户截图定；
/// 音频 metadata.width/height 同写此尺寸，卡片按宽比例布局，不再方形挤压）
pub const WAVEFORM_WIDTH: u32 = 640;
/// 波形图高（px）
pub const WAVEFORM_HEIGHT: u32 = 272;
/// 波形列数（= 图宽，逐像素 1 列连续填充，Eagle 同形态实心波形）
const WAVEFORM_COLS: usize = WAVEFORM_WIDTH as usize;
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
  let cols = rms_from_samples(&samples, WAVEFORM_COLS);
  Some(render_waveform(&cols, WAVEFORM_WIDTH, WAVEFORM_HEIGHT))
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

/// PCM 分桶 RMS 能量（逐列响度口径，2026-07-26 用户截图指正后改）：
/// 逐桶"最大峰值"会让每桶都打到波形振荡峰顶——整段一样高、糊成一坨、动态小的内容无波峰波谷；
/// RMS（能量均值开方）反映真实响度包络，安静段低、爆发段高。全局归一到 0..=1。
fn rms_from_samples(samples: &[f32], buckets: usize) -> Vec<f32> {
  if samples.is_empty() || buckets == 0 {
    return vec![0.0; buckets];
  }
  let bucket_len = samples.len().div_ceil(buckets);
  let mut cols: Vec<f32> = samples
    .chunks(bucket_len)
    .map(|c| (c.iter().map(|s| s * s).sum::<f32>() / c.len() as f32).sqrt())
    .collect();
  cols.resize(buckets, 0.0);
  let max = cols.iter().cloned().fold(0f32, f32::max);
  if max > 0.0 {
    for v in &mut cols {
      *v /= max;
    }
  }
  cols
}

/// RMS 列渲染（Eagle 同形态实心波形）：每列 1px 从 中线-v 填到 中线+v（连续无间距）；
/// 静音列也有最低 3px 中线（v=0 → half=1，安静段呈现为细横线）
fn render_waveform(cols: &[f32], w: u32, h: u32) -> RgbaImage {
  let mut img = RgbaImage::new(w, h);
  let cy = h / 2;
  let max_half = (h / 2).saturating_sub(WAVEFORM_MARGIN);
  let color = image::Rgba(BAR_RGBA);
  for (x, &v) in cols.iter().enumerate().take(w as usize) {
    let half = ((v.clamp(0.0, 1.0) * max_half as f32) as u32).max(1);
    for y in (cy - half)..=(cy + half).min(h - 1) {
      img.put_pixel(x as u32, y, color);
    }
  }
  img
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rms_reflects_loudness_envelope() {
    // 前 4 样本响（±0.5 振荡）、后 4 样本静音，4 桶 → [1, 1, 0, 0]（归一）
    let samples = [0.5, -0.5, 0.5, -0.5, 0.0, 0.0, 0.0, 0.0];
    assert_eq!(rms_from_samples(&samples, 4), vec![1.0, 1.0, 0.0, 0.0]);
    // 半响段 = 全响段能量一半 → 0.5（峰值口径下两者同为 1.0，分不清动态）
    let samples = [1.0, -1.0, 0.5, -0.5];
    assert_eq!(rms_from_samples(&samples, 2), vec![1.0, 0.5]);
  }

  #[test]
  fn rms_empty_and_zero_buckets() {
    assert_eq!(rms_from_samples(&[], 4), vec![0.0; 4]);
    assert_eq!(rms_from_samples(&[1.0], 0), Vec::<f32>::new());
    // 桶多于样本：后补零
    assert_eq!(rms_from_samples(&[0.5], 3), vec![1.0, 0.0, 0.0]);
  }

  #[test]
  fn render_waveform_fills_columns_continuously() {
    let mut cols = vec![0.0; WAVEFORM_COLS];
    cols[0] = 1.0;
    let img = render_waveform(&cols, WAVEFORM_WIDTH, WAVEFORM_HEIGHT);
    assert_eq!((img.width(), img.height()), (WAVEFORM_WIDTH, WAVEFORM_HEIGHT));
    // 第 1 列满幅实心：x=0 从 margin 到 h-margin 连续有柱色
    assert_eq!(img.get_pixel(0, WAVEFORM_MARGIN).0, BAR_RGBA);
    assert_eq!(img.get_pixel(0, WAVEFORM_HEIGHT / 2).0, BAR_RGBA);
    assert_eq!(img.get_pixel(0, WAVEFORM_HEIGHT - WAVEFORM_MARGIN - 1).0, BAR_RGBA);
    // 第 2 列（能量 0）只有中线：y=0 透明、中线有色
    assert_eq!(img.get_pixel(1, 0).0[3], 0);
    assert_eq!(img.get_pixel(1, WAVEFORM_HEIGHT / 2).0, BAR_RGBA);
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
      let img = waveform_image(Path::new(&a)).expect("波形应成功");
      // 目检调试用：波形落盘 target/waveform_debug.png（不入库）
      let _ = img.save(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/waveform_debug.png"));
    }
  }
}
