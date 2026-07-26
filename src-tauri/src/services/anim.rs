//! 动图（GIF / WEBP 动图）探测与逐帧解码（5.1）：纯 Rust，无 ffmpeg 依赖。
//! GIF 走 image crate 的 gif feature（AnimationDecoder）；WEBP 动图走 image-webp 直接依赖
//! （image crate 门面未暴露动画 API）。
//! 帧数据按需解析 + cache 会话级（不进 info，2026-07-25 用户定）——本模块只解码，
//! 缓存与播放归 5.2 预览播放套。静态 gif/webp（单帧）一律返回 None（按静态图片现状处理）。

use std::path::Path;

use image::{AnimationDecoder, ImageDecoder as _, RgbaImage};

/// 动图信息：导入时写 metadata（width/height/duration/frameCount）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimInfo {
  pub width: u32,
  pub height: u32,
  pub frame_count: u32,
  pub duration_secs: f64,
}

/// 探测动图信息；非 gif/webp、静态（单帧）、解析失败均返回 None（"能力探测"语义）
pub fn probe(path: &Path) -> Option<AnimInfo> {
  match ext_of(path).as_str() {
    "gif" => probe_gif(path),
    "webp" => probe_webp(path),
    _ => None,
  }
}

/// 解码第 index 帧（0 起）与其显示时长（ms）；越界 / 非动图 / 失败返回 None
#[allow(dead_code)] // 5.2 预览播放套消费（帧条 / 播放 / 设封面帧），本轮只到服务层 + 测试
pub fn decode_frame(path: &Path, index: u32) -> Option<(RgbaImage, u32)> {
  match ext_of(path).as_str() {
    "gif" => decode_gif_frame(path, index),
    "webp" => decode_webp_frame(path, index),
    _ => None,
  }
}

fn ext_of(path: &Path) -> String {
  path.extension().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default()
}

fn probe_gif(path: &Path) -> Option<AnimInfo> {
  let decoder = image::codecs::gif::GifDecoder::new(std::io::BufReader::new(std::fs::File::open(path).ok()?)).ok()?;
  let (width, height) = decoder.dimensions();
  let mut frame_count = 0u32;
  let mut ms = 0u64;
  for frame in decoder.into_frames() {
    let frame = frame.ok()?;
    frame_count += 1;
    ms += delay_ms(&frame) as u64;
  }
  if frame_count < 2 {
    return None;
  }
  Some(AnimInfo { width, height, frame_count, duration_secs: ms as f64 / 1000.0 })
}

#[allow(dead_code)] // 经 decode_frame 调用（5.2 消费）；clippy 直达性分析按非 test 构建判定
fn decode_gif_frame(path: &Path, index: u32) -> Option<(RgbaImage, u32)> {
  let decoder = image::codecs::gif::GifDecoder::new(std::io::BufReader::new(std::fs::File::open(path).ok()?)).ok()?;
  let frames = decoder.into_frames().collect_frames().ok()?;
  let frame = frames.get(index as usize)?;
  Some((frame.buffer().clone(), delay_ms(frame)))
}

fn delay_ms(frame: &image::Frame) -> u32 {
  let (n, d) = frame.delay().numer_denom_ms();
  n / d.max(1)
}

fn probe_webp(path: &Path) -> Option<AnimInfo> {
  let decoder = image_webp::WebPDecoder::new(std::io::BufReader::new(std::fs::File::open(path).ok()?)).ok()?;
  if !decoder.is_animated() {
    return None;
  }
  let (width, height) = decoder.dimensions();
  Some(AnimInfo {
    width,
    height,
    frame_count: decoder.num_frames(),
    duration_secs: decoder.loop_duration() as f64 / 1000.0,
  })
}

#[allow(dead_code)] // 经 decode_frame 调用（5.2 消费）；clippy 直达性分析按非 test 构建判定
fn decode_webp_frame(path: &Path, index: u32) -> Option<(RgbaImage, u32)> {
  let mut decoder = image_webp::WebPDecoder::new(std::io::BufReader::new(std::fs::File::open(path).ok()?)).ok()?;
  if !decoder.is_animated() || index >= decoder.num_frames() {
    return None;
  }
  let (w, h) = decoder.dimensions();
  let bpp = if decoder.has_alpha() { 4usize } else { 3 };
  let mut buf = vec![0u8; decoder.output_buffer_size()?];
  let mut delay = 0u32;
  for _ in 0..=index {
    delay = decoder.read_frame(&mut buf).ok()?;
  }
  buf.truncate(w as usize * h as usize * bpp);
  let img = if bpp == 4 {
    RgbaImage::from_raw(w, h, buf)?
  } else {
    image::DynamicImage::ImageRgb8(image::RgbImage::from_raw(w, h, buf)?).to_rgba8()
  };
  Some((img, delay))
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::path::PathBuf;

  /// 造 3 帧测试 gif（红/绿/蓝各 200ms）
  fn make_gif(path: &Path, frames: u32) {
    let colors = [[220, 40, 40, 255], [40, 200, 40, 255], [40, 40, 220, 255]];
    let file = std::fs::File::create(path).unwrap();
    let mut enc = image::codecs::gif::GifEncoder::new(file);
    enc.set_repeat(image::codecs::gif::Repeat::Infinite).unwrap();
    let frames: Vec<image::Frame> = colors
      .iter()
      .take(frames as usize)
      .map(|c| image::Frame::from_parts(RgbaImage::from_pixel(64, 48, image::Rgba(*c)), 0, 0, image::Delay::from_numer_denom_ms(200, 1)))
      .collect();
    enc.encode_frames(frames).unwrap();
  }

  #[test]
  fn probe_gif_reads_frames_and_duration() {
    let dir = tempfile::tempdir().unwrap();
    let gif = dir.path().join("动图.gif");
    make_gif(&gif, 3);
    let info = probe(&gif).unwrap();
    assert_eq!((info.width, info.height), (64, 48));
    assert_eq!(info.frame_count, 3);
    assert!((info.duration_secs - 0.6).abs() < 0.01);
  }

  #[test]
  fn probe_returns_none_for_static_gif_and_non_anim() {
    let dir = tempfile::tempdir().unwrap();
    let gif = dir.path().join("静态.gif");
    make_gif(&gif, 1);
    assert_eq!(probe(&gif), None, "单帧 gif 按静态图片处理");

    let txt = dir.path().join("文档.txt");
    std::fs::write(&txt, "hello").unwrap();
    assert_eq!(probe(&txt), None);
  }

  #[test]
  fn decode_gif_frame_returns_frame_and_delay() {
    let dir = tempfile::tempdir().unwrap();
    let gif = dir.path().join("动图.gif");
    make_gif(&gif, 3);
    let (img, delay) = decode_frame(&gif, 1).unwrap();
    assert_eq!(delay, 200);
    assert_eq!((img.width(), img.height()), (64, 48));
    let p = img.get_pixel(10, 10);
    assert!(p[1] > 150 && p[0] < 100, "第 2 帧应为绿色");
    assert!(decode_frame(&gif, 3).is_none(), "越界返回 None");
  }

  /// 测试样本：tools/test-assets/anim3.webp（tools/gen-anim-samples.py 生成，3 帧各 200ms）
  fn sample(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/test-assets").join(name)
  }

  #[test]
  fn probe_webp_anim() {
    let info = probe(&sample("anim3.webp")).expect("anim3.webp 应判定为动图");
    assert_eq!((info.width, info.height), (64, 48));
    assert_eq!(info.frame_count, 3);
    assert!((info.duration_secs - 0.6).abs() < 0.01);
  }

  #[test]
  fn probe_webp_still_returns_none() {
    assert_eq!(probe(&sample("still.webp")), None, "静态 webp 按静态图片处理");
  }

  #[test]
  fn decode_webp_frame_returns_frame_and_delay() {
    let (img, delay) = decode_frame(&sample("anim3.webp"), 2).expect("第 3 帧应可解码");
    assert_eq!(delay, 200);
    let p = img.get_pixel(10, 10);
    assert!(p[2] > 150 && p[0] < 120, "第 3 帧应为蓝色（有损编码容差）");
  }
}
