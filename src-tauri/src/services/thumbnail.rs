//! 缩略图服务：图片宽高探测 + 缩略图生成。
//! 缩略图存 info 文件夹（{名}_thumbnail.webp），是缓存可重建、不进备份（PRD 06-04）。
//! 探测 / 解码失败返回 None 属"能力探测"语义（不是图片），不算吞错误。

use std::fs;
use std::path::Path;

use image::DynamicImage;

/// 缩略图短边长度（像素）：短边缩到 320、长边等比（对齐 Eagle 同级策略；
/// 768 长边对千像素级原图过剩，体积不合算）
pub const THUMB_SHORT_EDGE: u32 = 320;

/// 有损 webp 质量（libwebp q80：目视接近无损，体积约为无损编码 1/5）
const WEBP_QUALITY: f32 = 80.0;

/// 探测图片宽高；非图片 / 探测失败返回 None
pub fn probe(path: &Path) -> Option<(u32, u32)> {
  image::image_dimensions(path).ok()
}

/// 生成缩略图：短边缩到 320、长边等比（原图短边不足 320 不放大），
/// 优先存有损 webp，失败退回 PNG。gif 取首帧（image::open 行为即首帧）。
/// 返回缩略图文件名；非图片 / 解码失败返回 None。
pub fn generate(info_dir: &Path, origin: &Path, stem: &str) -> Option<String> {
  let img = image::open(origin).ok()?;
  let thumb = shrink_to_thumb(img);

  let webp_name = format!("{stem}_thumbnail.webp");
  if save_webp(&thumb, &info_dir.join(&webp_name)).is_ok() {
    return Some(webp_name);
  }
  log::warn!("webp 缩略图编码失败，退回 PNG：{}", origin.display());
  let png_name = format!("{stem}_thumbnail.png");
  if thumb.save(&info_dir.join(&png_name)).is_ok() {
    return Some(png_name);
  }
  None
}

/// 短边缩到 THUMB_SHORT_EDGE、长边等比（整数除法 floor，短边恰好 320）；原图短边不足不放大
fn shrink_to_thumb(img: DynamicImage) -> DynamicImage {
  let (w, h) = (img.width(), img.height());
  if w.min(h) <= THUMB_SHORT_EDGE {
    return img;
  }
  let (nw, nh) = if w <= h {
    (THUMB_SHORT_EDGE, (h as u64 * THUMB_SHORT_EDGE as u64 / w as u64) as u32)
  } else {
    ((w as u64 * THUMB_SHORT_EDGE as u64 / h as u64) as u32, THUMB_SHORT_EDGE)
  };
  // imageops::resize 输出严格 nw×nh（thumbnail 的 fit-within 是长边语义，不合短边策略）
  let rgba = img.to_rgba8();
  DynamicImage::ImageRgba8(image::imageops::resize(&rgba, nw, nh, image::imageops::FilterType::Triangle))
}

/// 有损 webp 编码落盘（libwebp 官方绑定；image crate 的 WebPEncoder 只支持无损故不用）
fn save_webp(img: &DynamicImage, path: &Path) -> std::io::Result<()> {
  let rgba = img.to_rgba8();
  let mem = webp::Encoder::from_rgba(rgba.as_raw(), rgba.width(), rgba.height()).encode(WEBP_QUALITY);
  if mem.is_empty() {
    // libwebp 编码失败返回空缓冲，交上层退 PNG
    return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "libwebp 编码失败"));
  }
  fs::write(path, &*mem)
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 造一张纯色测试图
  fn make_png(path: &Path, w: u32, h: u32) {
    let img = image::RgbaImage::from_pixel(w, h, image::Rgba([200, 30, 30, 255]));
    img.save(path).unwrap();
  }

  #[test]
  fn probe_reads_image_dimensions() {
    let dir = tempfile::tempdir().unwrap();
    let png = dir.path().join("a.png");
    make_png(&png, 640, 480);
    assert_eq!(probe(&png), Some((640, 480)));

    // 非图片返回 None
    let txt = dir.path().join("a.txt");
    std::fs::write(&txt, "not an image").unwrap();
    assert_eq!(probe(&txt), None);
  }

  #[test]
  fn generate_shrinks_to_short_edge() {
    let dir = tempfile::tempdir().unwrap();

    // 横向 800x600 → 426x320（高缩到 320、宽等比）
    let land = dir.path().join("land.png");
    make_png(&land, 800, 600);
    let name = generate(dir.path(), &land, "land").unwrap();
    let out = dir.path().join(&name);
    assert_eq!(name, "land_thumbnail.webp");
    assert!(out.is_file());
    // 产物可再解码为 webp
    assert_eq!(image::image_dimensions(&out).unwrap(), (426, 320));
    // 有损压缩体积量级：纯色图应只有几 KB（无损 webp 同图为数十 KB 起）
    assert!(fs::metadata(&out).unwrap().len() < 50 * 1024, "缩略图体积异常");

    // 竖向 600x800 → 320x426（宽缩到 320、高等比）
    let port = dir.path().join("port.png");
    make_png(&port, 600, 800);
    let name = generate(dir.path(), &port, "port").unwrap();
    assert_eq!(image::image_dimensions(dir.path().join(&name)).unwrap(), (320, 426));

    // 方形 500x500 → 320x320
    let square = dir.path().join("square.png");
    make_png(&square, 500, 500);
    let name = generate(dir.path(), &square, "square").unwrap();
    assert_eq!(image::image_dimensions(dir.path().join(&name)).unwrap(), (320, 320));
  }

  #[test]
  fn generate_does_not_upscale_small_image() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("small.png");
    make_png(&src, 100, 50);

    let name = generate(dir.path(), &src, "small").unwrap();
    let (w, h) = image::image_dimensions(dir.path().join(&name)).unwrap();
    assert_eq!((w, h), (100, 50));
  }

  #[test]
  fn generate_returns_none_for_non_image() {
    let dir = tempfile::tempdir().unwrap();
    let txt = dir.path().join("工程.fla");
    std::fs::write(&txt, "fake fla bytes").unwrap();
    assert_eq!(generate(dir.path(), &txt, "工程"), None);
  }
}

