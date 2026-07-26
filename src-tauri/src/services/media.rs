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
/// 波形列数（RMS 采样网格：320 列 × 2px 间隔采样，渲染按列间线性插值 + 边缘抗锯齿——
/// 1px 细柱呈"线条感"、逐列整数阶跃呈"阶梯像素感"，均被 2026-07-26 用户截图指正）
const WAVEFORM_COLS: usize = 320;
/// 采样网格像素间隔（640 / 320 = 2；渲染逐像素插值，不再是"每列 2px 实心拼接"）
const COL_PX: u32 = 2;
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

/// 音频波形预览图（640×272 透明底；**上半 = 左声道、下半 = 右声道**立体声分绘，
/// mono 源 ffmpeg 复制成双声道自然对称）；缩略图与冲突弹窗预览共用
pub fn waveform_image(path: &Path) -> Option<RgbaImage> {
  if !is_audio(path) {
    return None;
  }
  let pcm = ffmpeg::audio_pcm_f32(path, PCM_RATE, 2)?;
  // 立体声交错去交错（LRLR… → L / R 两路）
  let (mut l, mut r) = (Vec::with_capacity(pcm.len() / 2), Vec::with_capacity(pcm.len() / 2));
  for pair in pcm.chunks_exact(2) {
    l.push(pair[0]);
    r.push(pair[1]);
  }
  let mut cols_l = rms_columns(&l, WAVEFORM_COLS);
  let mut cols_r = rms_columns(&r, WAVEFORM_COLS);
  // 联合归一（保持左右声道相对响度），再 3 点平滑连成连续包络
  let max = cols_l.iter().chain(cols_r.iter()).cloned().fold(0f32, f32::max);
  if max > 0.0 {
    for v in cols_l.iter_mut().chain(cols_r.iter_mut()) {
      *v /= max;
    }
  }
  smooth3(&mut cols_l);
  smooth3(&mut cols_r);
  Some(render_waveform(&cols_l, &cols_r, WAVEFORM_WIDTH, WAVEFORM_HEIGHT))
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

/// PCM 分桶 RMS 能量（逐列响度口径，未归一——左右声道联合归一在调用方做）：
/// 逐桶"最大峰值"会让每桶都打到波形振荡峰顶——整段一样高、糊成一坨、动态小的内容无波峰波谷；
/// RMS（能量均值开方）反映真实响度包络，安静段低、爆发段高（2026-07-26 用户截图指正后改）。
fn rms_columns(samples: &[f32], buckets: usize) -> Vec<f32> {
  if samples.is_empty() || buckets == 0 {
    return vec![0.0; buckets];
  }
  let bucket_len = samples.len().div_ceil(buckets);
  let mut cols: Vec<f32> = samples
    .chunks(bucket_len)
    .map(|c| (c.iter().map(|s| s * s).sum::<f32>() / c.len() as f32).sqrt())
    .collect();
  cols.resize(buckets, 0.0);
  cols
}

/// 3 点滑动平均（边缘重复端点）：相邻列 RMS 独立计算高频起伏大，
/// 平滑后连成连续包络，波形呈块状整体而非一根根细线
fn smooth3(cols: &mut [f32]) {
  if cols.len() < 3 {
    return;
  }
  let src = cols.to_vec();
  for (i, v) in cols.iter_mut().enumerate() {
    let a = src[i.saturating_sub(1)];
    let b = src[i];
    let c = src[(i + 1).min(src.len() - 1)];
    *v = (a + b + c) / 3.0;
  }
}

/// 列坐标 p（浮点）处的插值高度：相邻列线性插值——逐列整数阶跃是边缘"阶梯像素感"的根因
/// （2026-07-26 用户放大截图指正：Eagle 边缘连续平滑），插值后包络线为连续折线
fn height_at(cols: &[f32], p: f32) -> f32 {
  let i = p.floor() as usize;
  let f = p - i as f32;
  let a = cols.get(i).copied().unwrap_or(0.0);
  let b = cols.get(i + 1).copied().unwrap_or(a);
  a + (b - a) * f
}

/// 波形渲染（Eagle 同形态块状波形）：**上半 = 左声道、下半 = 右声道**；
/// 每像素按列间插值得浮点高度，边缘行按覆盖比例写 alpha（抗锯齿，无阶梯感）；
/// 静音列只画 1px 中线（最小高度 0.5px 上 + 0.5px 下）
fn render_waveform(cols_l: &[f32], cols_r: &[f32], w: u32, h: u32) -> RgbaImage {
  let mut img = RgbaImage::new(w, h);
  let cy = (h / 2) as f32;
  let max_half = (h / 2).saturating_sub(WAVEFORM_MARGIN) as f32;
  for x in 0..w {
    let p = x as f32 / COL_PX as f32;
    let top = (height_at(cols_l, p).clamp(0.0, 1.0) * max_half).max(0.5);
    let bot = (height_at(cols_r, p).clamp(0.0, 1.0) * max_half).max(0.5);
    let (y0, y1) = (cy - top, cy + bot);
    if top <= 0.5 && bot <= 0.5 {
      // 静音列：中线 1px 实心（跨行各 0.5 覆盖会淡成灰线）
      img.put_pixel(x, cy as u32, image::Rgba(BAR_RGBA));
      continue;
    }
    for y in 0..h {
      // 当前像素行 [y, y+1] 与波形区间 [y0, y1] 的覆盖比例（0..=1）
      let cov = (y1.min(y as f32 + 1.0) - y0.max(y as f32)).clamp(0.0, 1.0);
      if cov > 0.0 {
        let a = (BAR_RGBA[3] as f32 * cov).round() as u8;
        img.put_pixel(x, y, image::Rgba([BAR_RGBA[0], BAR_RGBA[1], BAR_RGBA[2], a]));
      }
    }
  }
  img
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rms_reflects_loudness_envelope() {
    // 前 4 样本响（±0.5 振荡）、后 4 样本静音，4 桶 → [0.5, 0.5, 0, 0]（未归一）
    let samples = [0.5, -0.5, 0.5, -0.5, 0.0, 0.0, 0.0, 0.0];
    assert_eq!(rms_columns(&samples, 4), vec![0.5, 0.5, 0.0, 0.0]);
    // 半响段 = 全响段能量一半（峰值口径下两者同为 1.0，分不清动态）
    let samples = [1.0, -1.0, 0.5, -0.5];
    assert_eq!(rms_columns(&samples, 2), vec![1.0, 0.5]);
  }

  #[test]
  fn rms_empty_and_zero_buckets() {
    assert_eq!(rms_columns(&[], 4), vec![0.0; 4]);
    assert_eq!(rms_columns(&[1.0], 0), Vec::<f32>::new());
    // 桶多于样本：后补零
    assert_eq!(rms_columns(&[0.5], 3), vec![0.5, 0.0, 0.0]);
  }

  #[test]
  fn smooth3_averages_neighbors() {
    let mut cols = vec![0.0, 0.0, 1.0, 0.0, 0.0];
    smooth3(&mut cols);
    let third = 1.0 / 3.0;
    assert!((cols[1] - third).abs() < 1e-6 && (cols[2] - third).abs() < 1e-6 && (cols[3] - third).abs() < 1e-6);
    assert_eq!(cols[0], 0.0);
  }

  #[test]
  fn render_waveform_interpolates_smooth_edges() {
    let mut cols_l = vec![0.0; WAVEFORM_COLS];
    let mut cols_r = vec![0.0; WAVEFORM_COLS];
    cols_l[0] = 1.0;
    cols_r[0] = 1.0;
    let img = render_waveform(&cols_l, &cols_r, WAVEFORM_WIDTH, WAVEFORM_HEIGHT);
    let cy = WAVEFORM_HEIGHT / 2;
    let max_half = cy - WAVEFORM_MARGIN;
    // x=0 满幅（最外边界行为半透明抗锯齿，取内行断言全色）
    assert_eq!(img.get_pixel(0, cy).0, BAR_RGBA);
    assert_eq!(img.get_pixel(0, WAVEFORM_MARGIN + 1).0, BAR_RGBA);
    // x=1 = 列 0→1 中点：插值高度约一半——半高以内实心、半高以外透明（无阶梯跳变）
    assert_eq!(img.get_pixel(1, cy - max_half / 2 + 2).0, BAR_RGBA);
    assert_eq!(img.get_pixel(1, cy - max_half / 2 - 2).0[3], 0);
    // x=2（列 1，能量 0）：只画 1px 中线，隔行透明
    assert_eq!(img.get_pixel(2, cy).0, BAR_RGBA);
    assert_eq!(img.get_pixel(2, cy - 2).0[3], 0);
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
