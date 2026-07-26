//! SVG 适配器：resvg（usvg 解析 + tiny-skia 渲染）；宽高取 usvg tree size（pt/mm 等单位换算已处理）。

use std::fs;
use std::path::Path;

use image::RgbaImage;

use super::FormatAdapter;
use crate::services::thumbnail::THUMB_SHORT_EDGE;

pub const ADAPTER: FormatAdapter = FormatAdapter { exts: &["svg"], probe: probe_svg, decode: decode_svg };

fn parse_svg(path: &Path) -> Option<resvg::usvg::Tree> {
  let data = fs::read(path).ok()?;
  let opt = resvg::usvg::Options::default();
  resvg::usvg::Tree::from_data(&data, &opt).map_err(|e| log::warn!("SVG 解析失败（{}）：{e}", path.display())).ok()
}

fn probe_svg(path: &Path) -> Option<(u32, u32)> {
  let size = parse_svg(path)?.size();
  let (w, h) = (size.width().round() as u32, size.height().round() as u32);
  (w > 0 && h > 0).then_some((w, h))
}

/// 短边缩到 THUMB_SHORT_EDGE 渲染、长边等比；原图短边不足不放大（与 thumbnail 同策略）
fn decode_svg(path: &Path) -> Option<RgbaImage> {
  let tree = parse_svg(path)?;
  let size = tree.size();
  let (w, h) = (size.width(), size.height());
  if !w.is_finite() || !h.is_finite() || w <= 0.0 || h <= 0.0 {
    return None;
  }
  let scale = if w.min(h) > THUMB_SHORT_EDGE as f32 { THUMB_SHORT_EDGE as f32 / w.min(h) } else { 1.0 };
  let tw = ((w * scale).round() as u32).max(1);
  let th = ((h * scale).round() as u32).max(1);
  let mut pixmap = resvg::tiny_skia::Pixmap::new(tw, th)?;
  resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
  let mut data = pixmap.take();
  // tiny-skia 输出预乘 alpha，解除预乘以匹配 webp 编码输入
  for px in data.chunks_exact_mut(4) {
    let a = px[3];
    if a > 0 && a < 255 {
      px[0] = ((px[0] as u32 * 255 + a as u32 / 2) / a as u32).min(255) as u8;
      px[1] = ((px[1] as u32 * 255 + a as u32 / 2) / a as u32).min(255) as u8;
      px[2] = ((px[2] as u32 * 255 + a as u32 / 2) / a as u32).min(255) as u8;
    }
  }
  RgbaImage::from_raw(tw, th, data)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn probe_and_decode_svg() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("图标.svg");
    fs::write(
      &f,
      r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50"><rect width="100" height="50" fill="#ff0000"/></svg>"##,
    )
    .unwrap();

    assert_eq!(probe_svg(&f), Some((100, 50)));

    // 短边 50 < 320 不放大；中心像素应为红色
    let img = decode_svg(&f).unwrap();
    assert_eq!((img.width(), img.height()), (100, 50));
    let px = img.get_pixel(50, 25);
    assert!(px[0] > 200 && px[1] < 50 && px[2] < 50, "中心应为红色，实际 {px:?}");

    // 大于 320 短边的按短边 320 等比缩小
    let big = dir.path().join("大.svg");
    fs::write(
      &big,
      r##"<svg xmlns="http://www.w3.org/2000/svg" width="800" height="600"><rect width="800" height="600" fill="#00ff00"/></svg>"##,
    )
    .unwrap();
    let img = decode_svg(&big).unwrap();
    assert_eq!((img.width(), img.height()), (427, 320));
  }
}
