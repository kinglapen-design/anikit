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
/// 波形柱色 RGB（#5B6261，2026-08-10 用户定：文字/标签档的不透明实色版——
/// 原 #F8F5FA 60% 亮灰白与新体系脱节，静态 / 未播放波形统一为此实色）
const BAR_RGB: [u8; 3] = [0x5B, 0x62, 0x61];
/// 卡片缩略图柱 alpha（100%——柱色本身即按黑底调好的不透明实色，2026-08-10；
/// 原 60% 透明叠底方案随实色化作废）
const THUMB_ALPHA: u8 = 255;
/// 波形上下留白（px，272 高大图口径）；小高度按 h/27 等比收缩（见 render_waveform）——
/// 恒 10 会把 18px 播放条请求的振幅吃成 0（全图只剩 1px 中线 = 看起来全空，2026-09-25 实机报）
const WAVEFORM_MARGIN: u32 = 10;

/// 视频元数据（导入写 metadata.width/height/duration；fps 5.3 帧化改造起写入，
/// 2026-07-28 用户定：视频播放条 / 悬停胶囊 / 笔记以帧号显示，按素材自身帧率）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VideoInfo {
  pub width: u32,
  pub height: u32,
  pub duration: f64,
  /// 帧率（首个视频流 avg_frame_rate，"0/0" 或缺失回退 r_frame_rate；皆不可得为 None）
  pub fps: Option<f64>,
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

/// 视频元数据（ffprobe）：首个视频流宽高 + format.duration（秒）+ 帧率；非视频格式 / 失败返回 None
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
  let fps = parse_rate(video.get("avg_frame_rate")).or_else(|| parse_rate(video.get("r_frame_rate")));
  Some(VideoInfo { width, height, duration, fps })
}

/// ffprobe 分数形式帧率解析："30000/1001" → 约 29.97，"24/1" → 24；
/// "0/0"（VFR 未知）/ 缺字段 / 非分数 / 零值 → None（调用方回退或放弃）
fn parse_rate(value: Option<&serde_json::Value>) -> Option<f64> {
  let s = value?.as_str()?;
  let (num, den) = s.split_once('/')?;
  let num = num.parse::<f64>().ok()?;
  let den = den.parse::<f64>().ok()?;
  (num > 0.0 && den > 0.0).then_some(num / den)
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

/// 音频波形预览图（640×272 透明底 60% alpha；**上半 = 左声道、下半 = 右声道**立体声分绘，
/// mono 源 ffmpeg 复制成双声道自然对称）；缩略图与冲突弹窗预览共用
pub fn waveform_image(path: &Path) -> Option<RgbaImage> {
  waveform_image_sized_alpha(path, WAVEFORM_WIDTH, WAVEFORM_HEIGHT, THUMB_ALPHA)
}

/// 按尺寸与柱 alpha 渲染波形图（内部共用）
fn waveform_image_sized_alpha(path: &Path, width: u32, height: u32, bar_alpha: u8) -> Option<RgbaImage> {
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
  Some(render_waveform(&l, &r, width, height, bar_alpha))
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
fn render_waveform(samples_l: &[f32], samples_r: &[f32], w: u32, h: u32, bar_alpha: u8) -> RgbaImage {
  let cy = (h / 2) as f32;
  // 留白随高度等比收缩（10/272 原比例）：小高度不被吃光——18px 播放条高 margin 0 满幅、
  // 48px margin 1；272 及以上恒 10（原 5.1 大图口径不变）
  let margin = WAVEFORM_MARGIN.min(h / 27);
  let max_half = (h / 2).saturating_sub(margin) as f32;
  let max_abs = samples_l
    .iter()
    .chain(samples_r.iter())
    .fold(0f32, |m, s| m.max(s.abs()));
  let mut img = RgbaImage::new(w, h);
  if max_abs > 0.0 {
    draw_channel(&mut img, samples_l, max_abs, true, cy, max_half, bar_alpha);
    draw_channel(&mut img, samples_r, max_abs, false, cy, max_half, bar_alpha);
  } else {
    // 全静音：画中线
    draw_channel(&mut img, &[0.0], 1.0, true, cy, max_half, bar_alpha);
  }
  soften_vertical(&mut img);
  img
}

/// 仅竖向边缘柔化：横向绝不晕染（细发丝恒 1px 宽），峰尖与主体上下缘向外 1px 过渡
/// （max(原图, 竖向 3 行均值)，核心零稀释；alpha 沿用原像素，不经参数）
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
        *img.get_pixel_mut(x, y) = image::Rgba([BAR_RGB[0], BAR_RGB[1], BAR_RGB[2], a]);
      }
    }
  }
}

/// 单声道半区绘制：每像素 1 个 min/max 窗口满色纵线（多窗口对满色无意义：
/// max of maxes 等价于一个大窗口）；top_half = 上半（裁在中线以上），false = 下半
fn draw_channel(img: &mut RgbaImage, samples: &[f32], max_abs: f32, top_half: bool, cy: f32, max_half: f32, bar_alpha: u8) {
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
      img.put_pixel(x, y, image::Rgba([BAR_RGB[0], BAR_RGB[1], BAR_RGB[2], bar_alpha]));
    }
  }
}

// ---------- 卡片 peaks（2026-09-22 R6 卡片三变体接线 · audio 变体，稿 = 组件/卡片/文件卡片-音频） ----------

/// 卡片波形 peaks 根数（恒定不随时长——稿 prose「不论是 5 秒音频还是 3 分钟音频都保证根数一致，
/// 美观第一位」；前端按卡宽节距 8px（4 线 + 4 距）max 池化重采样，128 根铺满约 1m 宽卡）
pub const CARD_PEAKS: usize = 128;

/// 音频卡片波形 peaks（0..1 归一化）：mono 混音 PCM 均分 n 桶取 max abs 联合归一
/// （不分声道——稿 prose「不用分析上下立体声差异」）；非音频 / 解码失败 None（能力探测语义同缩略图）
pub fn audio_peaks(path: &Path, n: usize) -> Option<Vec<f32>> {
  if !is_audio(path) {
    return None;
  }
  let pcm = ffmpeg::audio_pcm_f32(path, PCM_RATE, 1)?;
  Some(peaks_from_pcm(&pcm, n))
}

/// PCM → n 桶 max-abs 归一化 peaks；空样本 / n=0 → 空；全静音 → 全 0（前端最低高度档兜显细线）
fn peaks_from_pcm(samples: &[f32], n: usize) -> Vec<f32> {
  if n == 0 || samples.is_empty() {
    return Vec::new();
  }
  let max_abs = samples.iter().fold(0f32, |m, s| m.max(s.abs()));
  if max_abs <= 0.0 {
    return vec![0.0; n];
  }
  let step = samples.len() as f32 / n as f32;
  (0..n)
    .map(|i| {
      let s0 = (i as f32 * step) as usize;
      let s1 = (((i + 1) as f32 * step) as usize).max(s0 + 1).min(samples.len());
      let peak = samples[s0..s1].iter().fold(0f32, |m, s| m.max(s.abs()));
      peak / max_abs
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn render_waveform_body_solid_and_silence_is_thin_line() {
    let cy = WAVEFORM_HEIGHT / 2;
    // 过零振荡稳态信号（每窗口 hi=0.5 / lo=-0.5）→ 主体多次叠加趋实
    let samples: Vec<f32> = (0..4096).map(|i| if i % 2 == 0 { 0.5 } else { -0.5 }).collect();
    let img = render_waveform(&samples, &samples, WAVEFORM_WIDTH, WAVEFORM_HEIGHT, THUMB_ALPHA);
    assert_eq!((img.width(), img.height()), (WAVEFORM_WIDTH, WAVEFORM_HEIGHT));
    assert!(img.get_pixel(10, cy).0[3] > 100, "稳态主体应趋实");
    assert_eq!(img.get_pixel(10, 0).0[3], 0, "图外缘透明");
    // 静音 → 只在中线附近有波形
    let silence = vec![0.0f32; 4096];
    let img = render_waveform(&silence, &silence, WAVEFORM_WIDTH, WAVEFORM_HEIGHT, THUMB_ALPHA);
    assert!(img.get_pixel(10, cy).0[3] > 0);
    assert_eq!(img.get_pixel(10, cy - 4).0[3], 0);
  }

  #[test]
  fn render_waveform_transient_peak_full_color_with_soft_edge() {
    // 前半稳态振荡 0.4、中后部一个孤立尖峰 1.0 → 尖峰满色（辨识度不丢），
    // 仅边缘外晕 1px 软边（存在 0<alpha<满色 的像素）
    let bar = [BAR_RGB[0], BAR_RGB[1], BAR_RGB[2], THUMB_ALPHA];
    let mut samples: Vec<f32> = (0..65536).map(|i| if i % 2 == 0 { 0.4 } else { -0.4 }).collect();
    samples[32768..].fill(0.0);
    samples[60000] = 1.0; // 约 91.6% 处
    let img = render_waveform(&samples, &samples, WAVEFORM_WIDTH, WAVEFORM_HEIGHT, THUMB_ALPHA);
    let cy = WAVEFORM_HEIGHT / 2;
    assert_eq!(img.get_pixel(10, cy - 20).0, bar);
    // 孤立尖峰满色（位置 60000/65536 ≈ x 585~586）
    let hair = (585..=586).map(|x| img.get_pixel(x, WAVEFORM_MARGIN + 2).0[3]).max().unwrap();
    assert_eq!(hair, THUMB_ALPHA, "尖峰应满色保持辨识度");
    // 边缘柔化存在：满色与透明之间有过渡像素
    let has_soft = (0..img.height()).any(|y| {
      let a = img.get_pixel(585, y).0[3];
      a > 0 && a < THUMB_ALPHA
    });
    assert!(has_soft, "边缘应有 1px 柔化过渡");
  }

  #[test]
  fn render_waveform_mask_alpha_is_solid() {
    // bar_alpha 参数直通：255 入 = 柱 alpha 满色（历史 = 播放条 CSS mask 调色不被稀释，
    // mask 方案 2026-09-25 退役，参数行为保留作渲染函数契约）
    let samples: Vec<f32> = (0..4096).map(|i| if i % 2 == 0 { 0.5 } else { -0.5 }).collect();
    let img = render_waveform(&samples, &samples, 1200, 48, 255);
    assert_eq!(img.get_pixel(10, 24).0[3], 255, "mask 用途柱 alpha 必须满色");
  }

  #[test]
  fn render_waveform_scales_to_arbitrary_size() {
    // 参数化渲染：任意宽高产物尺寸正确（播放条 1200×48 / 钳制下限 32×8 / 预览大图 1920×400）
    let samples: Vec<f32> = (0..4096).map(|i| if i % 2 == 0 { 0.5 } else { -0.5 }).collect();
    for (w, h) in [(1200u32, 48u32), (32, 8), (1920, 400)] {
      let img = render_waveform(&samples, &samples, w, h, THUMB_ALPHA);
      assert_eq!((img.width(), img.height()), (w, h));
      assert!(img.get_pixel(w / 2, h / 2).0[3] > 0, "{w}x{h} 中线应有波形");
    }
  }

  #[test]
  fn render_waveform_small_height_keeps_amplitude() {
    // 回归钉（2026-09-25 实机报）：18px 播放条高请求，留白恒 10 时 max_half 被吃成 0、
    // 全图只剩 1px 中线（看起来全空）——留白等比收缩后 0.8 振幅柱顶距中线约 7px
    let samples: Vec<f32> = (0..4096).map(|i| if i % 2 == 0 { 0.8 } else { -0.8 }).collect();
    let img = render_waveform(&samples, &samples, 1200, 18, 255);
    assert!(img.get_pixel(10, 3).0[3] > 0, "18px 高小图振幅不应被留白吃光（cy-6 处应有波形）");
  }

  #[test]
  fn parse_rate_fraction_forms() {
    let v = |s: &str| Some(serde_json::Value::String(s.to_string()));
    // NTSC 分数帧率
    let fps = parse_rate(v("30000/1001").as_ref()).unwrap();
    assert!((fps - 29.97).abs() < 0.001, "{fps}");
    // 整数帧率
    assert_eq!(parse_rate(v("24/1").as_ref()), Some(24.0));
    assert_eq!(parse_rate(v("25/1").as_ref()), Some(25.0));
    // "0/0"（VFR 未知帧率）→ None（调用方回退 r_frame_rate）
    assert_eq!(parse_rate(v("0/0").as_ref()), None);
    // 缺字段 / 非分数 / 非法值 → None
    assert_eq!(parse_rate(None), None);
    assert_eq!(parse_rate(v("24").as_ref()), None);
    assert_eq!(parse_rate(v("abc/def").as_ref()), None);
    assert_eq!(parse_rate(v("24/0").as_ref()), None);
  }

  #[test]
  fn peaks_from_pcm_bucket_max_normalized() {
    // 桶 max-abs 联合归一：全局峰值所在桶 = 1.0，根数恒定不随时长（稿 prose）
    let mut samples = vec![0.0f32; 8000];
    samples[1000..2000].fill(0.5); // 第 2 桶（1/8 处）稳态 0.5
    samples[7000] = -1.0; // 末桶孤立尖峰（负峰也取 abs）
    let peaks = peaks_from_pcm(&samples, 8);
    assert_eq!(peaks.len(), 8);
    assert!((peaks[1] - 0.5).abs() < 1e-6, "{:?}", peaks);
    assert_eq!(peaks[7], 1.0, "孤立尖峰满度保持辨识度");
    assert_eq!(peaks[0], 0.0, "静音桶为 0（前端最低高度档兜显细线）");
    // 短音频（样本不足 n）：每桶至少 1 样本不 panic，根数仍恒定；全局峰 0.75 落在末两桶 = 1.0
    let short = peaks_from_pcm(&[0.25, -0.5, 0.75], 8);
    assert_eq!(short.len(), 8);
    assert_eq!(short[7], 1.0, "{:?}", short);
    assert!((short[0] - 0.25 / 0.75).abs() < 1e-6, "{:?}", short);
    // 全静音 → 全 0；空样本 / n=0 → 空
    assert_eq!(peaks_from_pcm(&vec![0.0; 100], 8), vec![0.0; 8]);
    assert!(peaks_from_pcm(&[], 8).is_empty());
    assert!(peaks_from_pcm(&[1.0], 0).is_empty());
  }

  #[test]
  fn ext_gates_prevent_spawning_sidecar() {
    // 非音视频扩展名直接 None（不启动 ffprobe）
    let txt = std::path::Path::new("文档.txt");
    assert_eq!(probe_video(txt), None);
    assert_eq!(probe_audio(txt), None);
    assert!(audio_peaks(txt, CARD_PEAKS).is_none());
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
