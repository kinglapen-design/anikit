//! 图片格式解码补充（4.2）：image crate 覆盖不了的 PSD / SVG，无 Adobe 依赖出内容缩略图。
//! - PSD：psd crate 解文件内嵌合成图；宽高走轻量固定头解析，不整文件解码。
//! - SVG：resvg（usvg 解析 + tiny-skia 渲染）；宽高取 usvg tree size（pt/mm 等单位换算已处理）。
//! 解析 / 解码失败返回 None 属"能力探测"语义，由 thumbnail 回退到格式图标。

use std::fs;
use std::io::Read;
use std::path::Path;

use image::RgbaImage;

use crate::services::thumbnail::THUMB_SHORT_EDGE;

/// 探测 PSD / SVG 宽高；其他格式 / 解析失败返回 None
pub fn probe(path: &Path) -> Option<(u32, u32)> {
  match ext_of(path).as_str() {
    "psd" => probe_psd(path),
    "svg" => probe_svg(path),
    _ => None,
  }
}

/// 解码 PSD / SVG 为 RGBA（SVG 按缩略图短边渲染，PSD 取合成图原尺寸）；失败返回 None
pub fn decode(path: &Path) -> Option<RgbaImage> {
  match ext_of(path).as_str() {
    "psd" => decode_psd(path),
    "svg" => decode_svg(path),
    _ => None,
  }
}

fn ext_of(path: &Path) -> String {
  path.extension().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default()
}

// ---------- PSD ----------

/// PSD 固定头（大端）："8BPS" 签名(0-3) + 版本(4-5) + 保留(6-11) + 通道数(12-13) + 高(14-17) + 宽(18-21)
fn probe_psd(path: &Path) -> Option<(u32, u32)> {
  let mut head = [0u8; 22];
  fs::File::open(path).ok()?.read_exact(&mut head).ok()?;
  if &head[0..4] != b"8BPS" {
    return None;
  }
  let h = u32::from_be_bytes(head[14..18].try_into().ok()?);
  let w = u32::from_be_bytes(head[18..22].try_into().ok()?);
  (w > 0 && h > 0).then_some((w, h))
}

/// 解 PSD 内嵌合成图；合成数据全透明 = 未存合成图（保存时关"最大兼容性"），返回 None 回退格式图标
fn decode_psd(path: &Path) -> Option<RgbaImage> {
  let bytes = fs::read(path).ok()?;
  let doc = psd::Psd::from_bytes(&bytes).map_err(|e| log::warn!("PSD 解析失败（{}）：{e}", path.display())).ok()?;
  let (w, h) = (doc.width(), doc.height());
  let rgba = doc.rgba();
  if w == 0 || h == 0 || rgba.len() != (w as usize) * (h as usize) * 4 {
    return None;
  }
  if rgba.chunks_exact(4).all(|px| px[3] == 0) {
    log::warn!("PSD 无内嵌合成图（未开最大兼容性保存）：{}", path.display());
    return None;
  }
  RgbaImage::from_raw(w, h, rgba)
}

// ---------- SVG ----------

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

  /// 构造 PSD 头（签名 + 版本 + 保留 + 通道 + 高 + 宽），尾部补零
  fn make_fake_psd(path: &Path, w: u32, h: u32) {
    let mut data = vec![0u8; 64];
    data[0..4].copy_from_slice(b"8BPS");
    data[4..6].copy_from_slice(&1u16.to_be_bytes());
    data[12..14].copy_from_slice(&3u16.to_be_bytes());
    data[14..18].copy_from_slice(&h.to_be_bytes());
    data[18..22].copy_from_slice(&w.to_be_bytes());
    fs::write(path, data).unwrap();
  }

  #[test]
  fn probe_psd_reads_fixed_header() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("画.psd");
    make_fake_psd(&f, 200, 100);
    assert_eq!(probe(&f), Some((200, 100)));

    // 签名不对不算 PSD
    let bad = dir.path().join("坏.psd");
    fs::write(&bad, b"not a psd at all, just some bytes").unwrap();
    assert_eq!(probe(&bad), None);
  }

  #[test]
  fn decode_psd_returns_none_for_fake_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("假.psd");
    fs::write(&f, "fake psd bytes").unwrap();
    assert!(decode(&f).is_none());
  }

  #[test]
  fn probe_and_decode_svg() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("图标.svg");
    fs::write(
      &f,
      r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50"><rect width="100" height="50" fill="#ff0000"/></svg>"##,
    )
    .unwrap();

    assert_eq!(probe(&f), Some((100, 50)));

    // 短边 50 < 320 不放大；中心像素应为红色
    let img = decode(&f).unwrap();
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
    let img = decode(&big).unwrap();
    assert_eq!((img.width(), img.height()), (427, 320));
  }

  #[test]
  fn probe_and_decode_ignore_other_ext() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("文档.txt");
    fs::write(&f, "hello").unwrap();
    assert_eq!(probe(&f), None);
    assert_eq!(decode(&f), None);
  }
}
