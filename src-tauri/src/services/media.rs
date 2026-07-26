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
  waveform_image_sized(path, WAVEFORM_WIDTH, WAVEFORM_HEIGHT)
}

/// 按任意尺寸重采样波形图（渲染口径同 5.1 定稿——满色纵线 / 竖向柔化 / 声道分绘 / 静音中线 /
/// 60% alpha，仅宽高为参数）：播放条 / 预览大图按实际显示像素请求重渲（1px 级清晰，2026-07-26 定）
pub fn waveform_image_sized(path: &Path, width: u32, height: u32) -> Option<RgbaImage> {
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
  Some(render_waveform(&l, &r, width, height))
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

/// 波形渲染（Eagle 同形态"实心主体 + 峰顶发丝"）：**上半 = 左声道、下半 = 右声道**。
/// 每像素 1 个 min/max 窗口满色纵线（细发丝恒 1px 宽、峰值辨识度不丢），
/// 仅竖向边缘柔化（横向零晕染，2026-07-26 用户定：线条要细不要粗硬）。
/// 左右声道联合归一；静音段为细中线。
fn render_waveform(samples_l: &[f32], samples_r: &[f32], w: u32, h: u32) -> RgbaImage {
  let cy = (h / 2) as f32;
  let max_half = (h / 2).saturating_sub(WAVEFORM_MARGIN) as f32;
  let max_abs = samples_l
    .iter()
    .chain(samples_r.iter())
    .fold(0f32, |m, s| m.max(s.abs()));
  let mut img = RgbaImage::new(w, h);
  if max_abs > 0.0 {
    draw_channel(&mut img, samples_l, max_abs, true, cy, max_half);
    draw_channel(&mut img, samples_r, max_abs, false, cy, max_half);
  } else {
    // 全静音：画中线
    draw_channel(&mut img, &[0.0], 1.0, true, cy, max_half);
  }
  soften_vertical(&mut img);
  img
}

/// 仅竖向边缘柔化：横向绝不晕染（细发丝恒 1px 宽），峰尖与主体上下缘向外 1px 过渡
/// （max(原图, 竖向 3 行均值)，核心零稀释）
fn soften_vertical(img: &mut RgbaImage) {
  let src = img.clone();
  let (w, h) = (img.width(), img.height());
  for y in 0..h {
    for x in 0..w {
      let mut sum = 0u32;
      for dy in -1i32..=1 {
        let yy = (y as i32 + dy).clamp(0, h as i32 - 1) as u32;
        sum += src.get_pixel(x, yy).0[3] as u32;
      }
      let a = src.get_pixel(x, y).0[3].max((sum / 3) as u8);
      if a > 0 {
        *img.get_pixel_mut(x, y) = image::Rgba([BAR_RGBA[0], BAR_RGBA[1], BAR_RGBA[2], a]);
      }
    }
  }
}

/// 单声道半区绘制：每像素 1 个 min/max 窗口满色纵线（多窗口对满色无意义：
/// max of maxes 等价于一个大窗口）；top_half = 上半（裁在中线以上），false = 下半
fn draw_channel(img: &mut RgbaImage, samples: &[f32], max_abs: f32, top_half: bool, cy: f32, max_half: f32) {
  let (sw, sh) = (img.width(), img.height());
  let step = samples.len() as f32 / sw as f32;
  let (lo_bound, hi_bound) = if top_half { (cy - max_half, cy) } else { (cy, cy + max_half) };
  for x in 0..sw {
    let s0 = (x as f32 * step) as usize;
    let s1 = (((x + 1) as f32 * step) as usize).max(s0 + 1).min(samples.len());
    let (mut hi, mut lo) = (0f32, 0f32);
    for &s in &samples[s0..s1] {
      hi = hi.max(s);
      lo = lo.min(s);
    }
    // 该列的纵线区间 [cy - hi*H, cy - lo*H]，裁剪到所属半区
    let y0 = (cy - (hi / max_abs) * max_half).max(lo_bound);
    let y1 = (cy - (lo / max_abs) * max_half).min(hi_bound);
    if y1 < y0 {
      continue;
    }
    for y in (y0 as u32)..=(y1 as u32).min(sh - 1) {
      img.put_pixel(x, y, image::Rgba(BAR_RGBA));
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn render_waveform_body_solid_and_silence_is_thin_line() {
    let cy = WAVEFORM_HEIGHT / 2;
    // 过零振荡稳态信号（每窗口 hi=0.5 / lo=-0.5）→ 主体多次叠加趋实
    let samples: Vec<f32> = (0..4096).map(|i| if i % 2 == 0 { 0.5 } else { -0.5 }).collect();
    let img = render_waveform(&samples, &samples, WAVEFORM_WIDTH, WAVEFORM_HEIGHT);
    assert_eq!((img.width(), img.height()), (WAVEFORM_WIDTH, WAVEFORM_HEIGHT));
    assert!(img.get_pixel(10, cy).0[3] > 100, "稳态主体应趋实");
    assert_eq!(img.get_pixel(10, 0).0[3], 0, "图外缘透明");
    // 静音 → 只在中线附近有波形
    let silence = vec![0.0f32; 4096];
    let img = render_waveform(&silence, &silence, WAVEFORM_WIDTH, WAVEFORM_HEIGHT);
    assert!(img.get_pixel(10, cy).0[3] > 0);
    assert_eq!(img.get_pixel(10, cy - 4).0[3], 0);
  }

  #[test]
  fn render_waveform_transient_peak_full_color_with_soft_edge() {
    // 前半稳态振荡 0.4、中后部一个孤立尖峰 1.0 → 尖峰满色（辨识度不丢），
    // 仅边缘外晕 1px 软边（存在 0<alpha<满色 的像素）
    let mut samples: Vec<f32> = (0..65536).map(|i| if i % 2 == 0 { 0.4 } else { -0.4 }).collect();
    samples[32768..].fill(0.0);
    samples[60000] = 1.0; // 约 91.6% 处
    let img = render_waveform(&samples, &samples, WAVEFORM_WIDTH, WAVEFORM_HEIGHT);
    let cy = WAVEFORM_HEIGHT / 2;
    assert_eq!(img.get_pixel(10, cy - 20).0, BAR_RGBA);
    // 孤立尖峰满色（位置 60000/65536 ≈ x 585~586）
    let hair = (585..=586).map(|x| img.get_pixel(x, WAVEFORM_MARGIN + 2).0[3]).max().unwrap();
    assert_eq!(hair, BAR_RGBA[3], "尖峰应满色保持辨识度");
    // 边缘柔化存在：满色与透明之间有过渡像素
    let has_soft = (0..img.height()).any(|y| {
      let a = img.get_pixel(585, y).0[3];
      a > 0 && a < BAR_RGBA[3]
    });
    assert!(has_soft, "边缘应有 1px 柔化过渡");
  }

  #[test]
  fn render_waveform_scales_to_arbitrary_size() {
    // 参数化渲染：任意宽高产物尺寸正确（播放条 1200×48 / 钳制下限 32×8 / 预览大图 1920×400）
    let samples: Vec<f32> = (0..4096).map(|i| if i % 2 == 0 { 0.5 } else { -0.5 }).collect();
    for (w, h) in [(1200u32, 48u32), (32, 8), (1920, 400)] {
      let img = render_waveform(&samples, &samples, w, h);
      assert_eq!((img.width(), img.height()), (w, h));
      assert!(img.get_pixel(w / 2, h / 2).0[3] > 0, "{w}x{h} 中线应有波形");
    }
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
