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
/// 波形列数（RMS 采样网格：320 列；渲染 2 倍超采样逐像素插值 + Triangle 降采样——
/// 1px 细柱"线条感"、逐列整数阶跃"阶梯像素感"、2px 网格折角，均被 2026-07-26 用户截图指正）
const WAVEFORM_COLS: usize = 320;
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

/// 波形渲染（Eagle 同形态块状波形）：**2 倍超采样**——1280×544 逐像素硬填充（高度 = 320 列
/// 采样网格的列间线性插值），再 Triangle 降采样回 640×272。逐列整数阶跃与 2px 网格折角
/// 由降采样的多点混合消除，边缘自然抗锯齿（对齐 Eagle 的连续平滑，2026-07-26 用户放大截图定）。
/// 上半 = 左声道、下半 = 右声道；静音列 2× 下 2px 中线（降采样后约 1px 细线）
fn render_waveform(cols_l: &[f32], cols_r: &[f32], w: u32, h: u32) -> RgbaImage {
  const SS: u32 = 2;
  let (sw, sh) = (w * SS, h * SS);
  let cy = (sh / 2) as f32;
  let max_half = (sh / 2).saturating_sub(WAVEFORM_MARGIN * SS) as f32;
  let mut big = RgbaImage::new(sw, sh);
  let color = image::Rgba(BAR_RGBA);
  for x in 0..sw {
    let p = x as f32 * WAVEFORM_COLS as f32 / sw as f32;
    let top = (height_at(cols_l, p).clamp(0.0, 1.0) * max_half).max(1.0);
    let bot = (height_at(cols_r, p).clamp(0.0, 1.0) * max_half).max(1.0);
    let y0 = (cy - top).max(0.0) as u32;
    let y1 = ((cy + bot) as u32).min(sh - 1);
    for y in y0..=y1 {
      big.put_pixel(x, y, color);
    }
  }
  image::imageops::resize(&big, w, h, image::imageops::FilterType::Triangle)
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
  fn height_at_interpolates_between_columns() {
    let cols = [0.0, 1.0, 0.0];
    assert_eq!(height_at(&cols, 0.0), 0.0);
    assert_eq!(height_at(&cols, 0.5), 0.5);
    assert_eq!(height_at(&cols, 1.0), 1.0);
    assert_eq!(height_at(&cols, 9.0), 0.0, "越界回退 0");
  }

  #[test]
  fn render_waveform_supersampled_shape() {
    let mut cols_l = vec![0.0; WAVEFORM_COLS];
    let mut cols_r = vec![0.0; WAVEFORM_COLS];
    cols_l[0] = 1.0;
    cols_r[0] = 1.0;
    let img = render_waveform(&cols_l, &cols_r, WAVEFORM_WIDTH, WAVEFORM_HEIGHT);
    assert_eq!((img.width(), img.height()), (WAVEFORM_WIDTH, WAVEFORM_HEIGHT));
    let cy = WAVEFORM_HEIGHT / 2;
    // 第 1 列满幅：中心有波形、图外缘透明
    assert!(img.get_pixel(0, cy).0[3] > 0);
    assert_eq!(img.get_pixel(0, 0).0[3], 0);
    // 静音列（列 1 在 x=4，p = x * 320 / 1280）：中线附近有波形、远离中线透明
    assert!(img.get_pixel(4, cy).0[3] > 0);
    assert_eq!(img.get_pixel(4, cy - 4).0[3], 0);
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
