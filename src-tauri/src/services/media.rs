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
/// 每像素叠加的 min/max 窗口数：alpha 累积让稳态主体叠成实心，瞬态尖峰只有少量窗口
/// 覆盖、呈半透明细发丝（Eagle 同形态"主体实心 + 峰顶发丝"，2026-07-26 用户定）；
/// 平滑口径（RMS / 滑动平均）会把尖峰抹成圆角一坨，被用户指正后弃用
const HAIR_WINDOWS: usize = 4;
/// 单次叠加 alpha（over 合成）：发丝 ~70，主体经 4 次累积趋近 180
const HAIR_ALPHA: u8 = 70;
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
  Some(render_waveform(&l, &r, WAVEFORM_WIDTH, WAVEFORM_HEIGHT))
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
/// 每像素 HAIR_WINDOWS 个 min/max 窗口以低 alpha 逐条叠加——稳态段所有窗口高度趋同，
/// alpha 累积成实心主体；瞬态尖峰只有个别窗口够得着，保持半透明细发丝。
/// 内部 2 倍超采样（1280×544）渲染后 Triangle 降采样回 640×272，边缘自然抗锯齿；
/// 左右声道联合归一（保持相对响度）；静音段为细中线。
fn render_waveform(samples_l: &[f32], samples_r: &[f32], w: u32, h: u32) -> RgbaImage {
  const SS: u32 = 2;
  let (sw, sh) = (w * SS, h * SS);
  let cy = (sh / 2) as f32;
  let max_half = (sh / 2).saturating_sub(WAVEFORM_MARGIN * SS) as f32;
  let max_abs = samples_l
    .iter()
    .chain(samples_r.iter())
    .fold(0f32, |m, s| m.max(s.abs()));
  let mut big = RgbaImage::new(sw, sh);
  if max_abs > 0.0 {
    draw_channel(&mut big, samples_l, max_abs, true, cy, max_half);
    draw_channel(&mut big, samples_r, max_abs, false, cy, max_half);
  } else {
    // 全静音：画中线
    draw_channel(&mut big, &[0.0], 1.0, true, cy, max_half);
  }
  // 降采样得到带软覆盖的掩码后统一填色：覆盖 ≥ UNIFY_THRESHOLD 一律 60% 实色
  // （2026-07-26 用户定——先填色再降采样会把 2px 细发丝稀释成"第二层"半透明）。
  // 发丝靠形状（细条）体现；阈值挡掉降采样产生的亚像素光晕
  let img = image::imageops::resize(&big, w, h, image::imageops::FilterType::Triangle);
  const UNIFY_THRESHOLD: u8 = 16;
  let mut img = img;
  for p in img.pixels_mut() {
    // 降采样会把边缘 RGB 与透明黑混合成脏灰，连色带 alpha 一起归一
    *p = if p.0[3] >= UNIFY_THRESHOLD { image::Rgba(BAR_RGBA) } else { image::Rgba([0, 0, 0, 0]) };
  }
  img
}

/// 单声道半区叠加绘制：top_half = 上半（裁在中线以上），false = 下半
fn draw_channel(img: &mut RgbaImage, samples: &[f32], max_abs: f32, top_half: bool, cy: f32, max_half: f32) {
  let (sw, sh) = (img.width(), img.height());
  let windows = sw as usize * HAIR_WINDOWS;
  let step = samples.len() as f32 / windows as f32;
  let (lo_bound, hi_bound) = if top_half { (cy - max_half, cy) } else { (cy, cy + max_half) };
  for i in 0..windows {
    let s0 = (i as f32 * step) as usize;
    let s1 = (((i + 1) as f32 * step) as usize).max(s0 + 1).min(samples.len());
    let (mut hi, mut lo) = (0f32, 0f32);
    for &s in &samples[s0..s1] {
      hi = hi.max(s);
      lo = lo.min(s);
    }
    // 该窗口的纵线区间 [cy - hi*H, cy - lo*H]，裁剪到所属半区
    let y0 = (cy - (hi / max_abs) * max_half).max(lo_bound);
    let y1 = (cy - (lo / max_abs) * max_half).min(hi_bound);
    if y1 < y0 {
      continue;
    }
    let x = (i / HAIR_WINDOWS) as u32;
    for y in (y0 as u32)..=(y1 as u32).min(sh - 1) {
      blend_alpha(img, x, y, HAIR_ALPHA);
    }
  }
}

/// 同色 over 合成：a' = 1 - (1-a)(1-A)——主体随叠加次数趋实，单次覆盖保持半透明发丝
fn blend_alpha(img: &mut RgbaImage, x: u32, y: u32, a: u8) {
  let p = img.get_pixel_mut(x, y);
  let old = p.0[3] as u32;
  let new = 255 - (255 - old) * (255 - a as u32) / 255;
  *p = image::Rgba([BAR_RGBA[0], BAR_RGBA[1], BAR_RGBA[2], new as u8]);
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn blend_alpha_accumulates() {
    let mut img = RgbaImage::new(1, 1);
    blend_alpha(&mut img, 0, 0, HAIR_ALPHA);
    let a1 = img.get_pixel(0, 0).0[3];
    assert_eq!(a1, HAIR_ALPHA);
    blend_alpha(&mut img, 0, 0, HAIR_ALPHA);
    assert!(img.get_pixel(0, 0).0[3] > a1, "同色叠加 alpha 递增");
  }

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
  fn render_waveform_transient_peak_keeps_shape_uniform_color() {
    // 前半稳态振荡 0.4、末尾一个孤立尖峰 1.0 → 尖峰形状保留，且主体内部统一填色无分层
    let mut samples: Vec<f32> = (0..65536).map(|i| if i % 2 == 0 { 0.4 } else { -0.4 }).collect();
    samples[32768..].fill(0.0);
    samples[65535] = 1.0;
    let img = render_waveform(&samples, &samples, WAVEFORM_WIDTH, WAVEFORM_HEIGHT);
    let cy = WAVEFORM_HEIGHT / 2;
    // 主体内部 = 统一实色（无 alpha 分层）
    assert_eq!(img.get_pixel(10, cy - 20).0, BAR_RGBA);
    assert_eq!(img.get_pixel(100, cy + 10).0, BAR_RGBA);
    // 孤立尖峰形状保留且与主体完全同色（降采样后阈值统一填色，无"第二层"）
    assert_eq!(img.get_pixel(WAVEFORM_WIDTH - 1, WAVEFORM_MARGIN + 2).0, BAR_RGBA);
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
