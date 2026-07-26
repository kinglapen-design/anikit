//! PSB（8BPB，大型文档格式）合成图解码（4.2 补）：psd crate 明确不支持 PSB，
//! 而素材常发 PSB——自研轻量解析，只取文件尾部 Image Data Section 的内嵌合成图（不解图层）。
//! 流式读取（按节长度 seek 跳转、RLE 逐行解码），整文件不入内存；不支持的颜色模式返回 None 回退格式图标。

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use image::RgbaImage;

/// 合成图像素上限（8000×8000 量级）：防爆内存，超出回退格式图标
const MAX_PIXELS: u64 = 64_000_000;

/// 解 PSB 内嵌合成图为 RGBA（8/16 位 RGB、灰度、CMYK 近似）；解析失败 / 不支持返回 None
pub fn decode_composite(path: &Path) -> Option<RgbaImage> {
  let mut f = File::open(path).ok()?;
  let head = read_header(&mut f, path)?;
  skip_to_image_data(&mut f, head.is_psb, path)?;
  let compression = read_u16(&mut f, path)?;
  let planes = match compression {
    0 => read_raw_planes(&mut f, &head, path)?,
    1 => read_rle_planes(&mut f, &head, path)?,
    other => {
      log::warn!("PSB 合成图压缩方式不支持（{other}）：{}", path.display());
      return None;
    }
  };
  compose_rgba(&planes, &head, path)
}

struct Head {
  channels: u16,
  w: u32,
  h: u32,
  depth: u16,
  color_mode: u16,
  is_psb: bool,
}

/// 文件头 26 字节（PSD / PSB 布局一致）：签名 4 + 版本 2 + 保留 6 + 通道 2 + 高 4 + 宽 4 + 位深 2 + 颜色模式 2。
/// PSB 结构判定看**版本号**（2 = 8 字节长度字段）而非签名——存在 8BPS+version2 的变体文件
///（部分工具链导出，2026-07-23 实测踩坑），按签名判定会把 8 字节长度误读为 4 字节导致 seek 错位
fn read_header(f: &mut File, path: &Path) -> Option<Head> {
  let mut head = [0u8; 26];
  f.read_exact(&mut head).ok()?;
  let version = u16::from_be_bytes(head[4..6].try_into().ok()?);
  let is_psb = match (&head[0..4], version) {
    (b"8BPS", 1) => false,
    (b"8BPS" | b"8BPB", 2) => true,
    _ => return None,
  };
  let h = u32::from_be_bytes(head[14..18].try_into().ok()?);
  let w = u32::from_be_bytes(head[18..22].try_into().ok()?);
  let depth = u16::from_be_bytes(head[22..24].try_into().ok()?);
  if w == 0 || h == 0 || w as u64 * h as u64 > MAX_PIXELS {
    log::warn!("PSB 尺寸超上限（{w}×{h}）：{}", path.display());
    return None;
  }
  if depth != 8 && depth != 16 {
    log::warn!("PSB 位深不支持（{depth}）：{}", path.display());
    return None;
  }
  let channels = u16::from_be_bytes(head[12..14].try_into().ok()?);
  let color_mode = u16::from_be_bytes(head[24..26].try_into().ok()?);
  Some(Head { channels, w, h, depth, color_mode, is_psb })
}

/// 跳过 Color Mode Data（4 字节长度）/ Image Resources（4 字节长度）/
/// Layer and Mask（PSB 8 字节长度，PSD 4 字节）三节，定位于 Image Data Section
fn skip_to_image_data(f: &mut File, is_psb: bool, path: &Path) -> Option<()> {
  let color_len = read_u32(f, path)? as i64;
  f.seek(SeekFrom::Current(color_len)).ok()?;
  let res_len = read_u32(f, path)? as i64;
  f.seek(SeekFrom::Current(res_len)).ok()?;
  let layer_len = if is_psb { read_u64(f, path)? } else { read_u32(f, path)? as u64 } as i64;
  f.seek(SeekFrom::Current(layer_len)).ok()?;
  Some(())
}

/// 每通道平面解码为 8 位样本（16 位取大端高字节）；返回 planes[通道][行主序像素]
type Planes = Vec<Vec<u8>>;

fn bytes_per_sample(head: &Head) -> usize {
  (head.depth / 8) as usize
}

/// Raw（compression=0）：平面数据按通道顺序连续存放，每通道 h 行 × w×bps 字节
fn read_raw_planes(f: &mut File, head: &Head, path: &Path) -> Option<Planes> {
  let bps = bytes_per_sample(head);
  let px = (head.w * head.h) as usize;
  let mut planes = Vec::with_capacity(head.channels as usize);
  for _ in 0..head.channels {
    let mut raw = vec![0u8; px * bps];
    f.read_exact(&mut raw).map_err(|e| log::warn!("PSB 合成图读取失败（{}）：{e}", path.display())).ok()?;
    planes.push(downsample_plane(&raw, bps));
  }
  Some(planes)
}

/// RLE（compression=1）：行长度表（条目 = 通道数 × 行数，PSB 4 字节 / PSD 2 字节）
/// + PackBits 流（按通道逐行）；逐行读取解码，流不全入内存
fn read_rle_planes(f: &mut File, head: &Head, path: &Path) -> Option<Planes> {
  let bps = bytes_per_sample(head);
  let rows = head.channels as usize * head.h as usize;
  let mut row_lens = Vec::with_capacity(rows);
  for _ in 0..rows {
    row_lens.push(if head.is_psb { read_u32(f, path)? as usize } else { read_u16(f, path)? as usize });
  }
  let row_bytes = head.w as usize * bps;
  let mut planes = Vec::with_capacity(head.channels as usize);
  for c in 0..head.channels as usize {
    let mut plane = Vec::with_capacity(row_bytes * head.h as usize);
    for r in 0..head.h as usize {
      let len = row_lens[c * head.h as usize + r];
      let mut packed = vec![0u8; len];
      f.read_exact(&mut packed).map_err(|e| log::warn!("PSB RLE 读取失败（{}）：{e}", path.display())).ok()?;
      plane.extend_from_slice(&unpack_bits(&packed, row_bytes)?);
    }
    planes.push(downsample_plane(&plane, bps));
  }
  Some(planes)
}

/// PackBits 解码：头字节 0..=127 → 后跟 n+1 字面字节；129..=255（i8 负）→ 后一字节重复 1-n 次；128 = NOP
fn unpack_bits(data: &[u8], expected: usize) -> Option<Vec<u8>> {
  let mut out = Vec::with_capacity(expected);
  let mut i = 0;
  while out.len() < expected {
    let n = *data.get(i)? as i8;
    i += 1;
    if n >= 0 {
      let count = n as usize + 1;
      out.extend_from_slice(data.get(i..i + count)?);
      i += count;
    } else if n != -128 {
      let count = (1i16 - n as i16) as usize;
      let b = *data.get(i)?;
      i += 1;
      out.extend(std::iter::repeat(b).take(count));
    }
  }
  out.truncate(expected);
  Some(out)
}

/// 16 位样本取大端高字节降为 8 位；8 位原样
fn downsample_plane(raw: &[u8], bps: usize) -> Vec<u8> {
  if bps == 1 {
    return raw.to_vec();
  }
  raw.chunks_exact(bps).map(|s| s[0]).collect()
}

/// 平面合成 RGBA：RGB（通道 0-2，第 4 通道为 alpha）、灰度（通道 0，第 2 通道为 alpha）、
/// CMYK（存储反相：先 255-v 得油墨量，再 R = 255 - min(255, C+K) 近似——不做色彩管理，仅供缩略图）
fn compose_rgba(planes: &Planes, head: &Head, path: &Path) -> Option<RgbaImage> {
  let px = (head.w * head.h) as usize;
  if planes.iter().any(|p| p.len() != px) {
    log::warn!("PSB 合成图平面尺寸异常：{}", path.display());
    return None;
  }
  let mut rgba = vec![0u8; px * 4];
  match (head.color_mode, head.channels) {
    (3, c) if c >= 3 => {
      for i in 0..px {
        rgba[i * 4] = planes[0][i];
        rgba[i * 4 + 1] = planes[1][i];
        rgba[i * 4 + 2] = planes[2][i];
        rgba[i * 4 + 3] = if c >= 4 { planes[3][i] } else { 255 };
      }
    }
    (1, c) if c >= 1 => {
      for i in 0..px {
        rgba[i * 4] = planes[0][i];
        rgba[i * 4 + 1] = planes[0][i];
        rgba[i * 4 + 2] = planes[0][i];
        rgba[i * 4 + 3] = if c >= 2 { planes[1][i] } else { 255 };
      }
    }
    (4, c) if c >= 4 => {
      for i in 0..px {
        let (cyan, magenta, yellow, key) = (
          255 - planes[0][i],
          255 - planes[1][i],
          255 - planes[2][i],
          255 - planes[3][i],
        );
        rgba[i * 4] = 255u8.saturating_sub(cyan.saturating_add(key));
        rgba[i * 4 + 1] = 255u8.saturating_sub(magenta.saturating_add(key));
        rgba[i * 4 + 2] = 255u8.saturating_sub(yellow.saturating_add(key));
        rgba[i * 4 + 3] = if c >= 5 { planes[4][i] } else { 255 };
      }
    }
    (mode, _) => {
      log::warn!("PSB 颜色模式不支持（{mode}）：{}", path.display());
      return None;
    }
  }
  RgbaImage::from_raw(head.w, head.h, rgba)
}

fn read_u16(f: &mut File, path: &Path) -> Option<u16> {
  let mut b = [0u8; 2];
  f.read_exact(&mut b).map_err(|e| log::warn!("PSB 读取失败（{}）：{e}", path.display())).ok()?;
  Some(u16::from_be_bytes(b))
}

fn read_u32(f: &mut File, path: &Path) -> Option<u32> {
  let mut b = [0u8; 4];
  f.read_exact(&mut b).map_err(|e| log::warn!("PSB 读取失败（{}）：{e}", path.display())).ok()?;
  Some(u32::from_be_bytes(b))
}

fn read_u64(f: &mut File, path: &Path) -> Option<u64> {
  let mut b = [0u8; 8];
  f.read_exact(&mut b).map_err(|e| log::warn!("PSB 读取失败（{}）：{e}", path.display())).ok()?;
  Some(u64::from_be_bytes(b))
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;

  /// 构造最小 PSB：头 + 三个空节 + image data
  #[allow(clippy::too_many_arguments)] // 测试构造器按 PSB 头字段平铺，读比封装 struct 更直接
  fn make_psb(path: &Path, channels: u16, w: u32, h: u32, depth: u16, mode: u16, compression: u16, tail: &[u8]) {
    make_variant(path, b"8BPB", channels, w, h, depth, mode, compression, tail);
  }

  /// 同 make_psb 但自定义签名（8BPS+version2 变体：部分工具链导出，按版本号判 PSB 结构）
  #[allow(clippy::too_many_arguments)] // 测试构造器按 PSB 头字段平铺，读比封装 struct 更直接
  fn make_variant(path: &Path, sig: &[u8; 4], channels: u16, w: u32, h: u32, depth: u16, mode: u16, compression: u16, tail: &[u8]) {
    let mut data = Vec::new();
    data.extend_from_slice(sig);
    data.extend_from_slice(&2u16.to_be_bytes());
    data.extend_from_slice(&[0u8; 6]);
    data.extend_from_slice(&channels.to_be_bytes());
    data.extend_from_slice(&h.to_be_bytes());
    data.extend_from_slice(&w.to_be_bytes());
    data.extend_from_slice(&depth.to_be_bytes());
    data.extend_from_slice(&mode.to_be_bytes());
    data.extend_from_slice(&0u32.to_be_bytes()); // color mode data
    data.extend_from_slice(&0u32.to_be_bytes()); // image resources
    data.extend_from_slice(&0u64.to_be_bytes()); // layer & mask（version=2 → 8 字节）
    data.extend_from_slice(&compression.to_be_bytes());
    data.extend_from_slice(tail);
    fs::write(path, data).unwrap();
  }

  #[test]
  fn decodes_8bps_version2_variant() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("变体.psb");
    // 8BPS 签名 + version 2（实测公司工具链导出文件）：layer 长度仍按 8 字节
    make_variant(&f, b"8BPS", 3, 1, 1, 8, 3, 0, &[9, 8, 7]);
    let img = decode_composite(&f).unwrap();
    assert_eq!(img.get_pixel(0, 0).0, [9, 8, 7, 255]);
  }

  #[test]
  fn decodes_raw_rgb_psb() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("大图.psb");
    // 2×2 RGB：R 平面 [255,0,0,255]，G/B 全 0 → 像素 (255,0,0) / 黑交替
    make_psb(&f, 3, 2, 2, 8, 3, 0, &[255, 0, 0, 255, 0, 0, 0, 0, 0, 0, 0, 0]);
    let img = decode_composite(&f).unwrap();
    assert_eq!((img.width(), img.height()), (2, 2));
    assert_eq!(img.get_pixel(0, 0).0, [255, 0, 0, 255]);
    assert_eq!(img.get_pixel(1, 0).0, [0, 0, 0, 255]);
    assert_eq!(img.get_pixel(1, 1).0, [255, 0, 0, 255]);
  }

  #[test]
  fn decodes_rle_rgb_psb() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("压缩.psb");
    // 2×2 RGB RLE：行长度表 3 通道 × 2 行 = 6 条 × 4 字节；每行 PackBits [1, v1, v2]（2 字面字节）
    let mut tail = Vec::new();
    for _ in 0..6 {
      tail.extend_from_slice(&3u32.to_be_bytes());
    }
    for plane in [[255u8, 0u8, 0u8, 255u8], [10, 20, 30, 40], [1, 2, 3, 4]] {
      for row in 0..2 {
        tail.extend_from_slice(&[1, plane[row * 2], plane[row * 2 + 1]]);
      }
    }
    make_psb(&f, 3, 2, 2, 8, 3, 1, &tail);
    let img = decode_composite(&f).unwrap();
    assert_eq!(img.get_pixel(0, 0).0, [255, 10, 1, 255]);
    assert_eq!(img.get_pixel(1, 1).0, [255, 40, 4, 255]);
  }

  #[test]
  fn decodes_16bit_by_high_byte() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("深.psb");
    // 1×1 RGB 16 位：样本大端 [0xAB, 0x00] → 取高字节 0xAB
    make_psb(&f, 3, 1, 1, 16, 3, 0, &[0xAB, 0x11, 0xCD, 0x22, 0xEF, 0x33]);
    let img = decode_composite(&f).unwrap();
    assert_eq!(img.get_pixel(0, 0).0, [0xAB, 0xCD, 0xEF, 255]);
  }

  #[test]
  fn decodes_grayscale_and_cmyk() {
    let dir = tempfile::tempdir().unwrap();
    // 灰度 1×1：值 128 → (128,128,128)
    let g = dir.path().join("灰.psb");
    make_psb(&g, 1, 1, 1, 8, 1, 0, &[128]);
    assert_eq!(decode_composite(&g).unwrap().get_pixel(0, 0).0, [128, 128, 128, 255]);
    // CMYK 1×1：存 0,0,0,255（= 油墨 C0 M0 Y0 K100%）→ 黑
    let c = dir.path().join("彩.psb");
    make_psb(&c, 4, 1, 1, 8, 4, 0, &[0, 0, 0, 255]);
    assert_eq!(decode_composite(&c).unwrap().get_pixel(0, 0).0, [0, 0, 0, 255]);
  }

  #[test]
  fn rejects_unsupported_and_truncated() {
    let dir = tempfile::tempdir().unwrap();
    // Lab 模式（9）不支持
    let lab = dir.path().join("lab.psb");
    make_psb(&lab, 3, 1, 1, 8, 9, 0, &[0, 0, 0]);
    assert!(decode_composite(&lab).is_none());
    // 截断文件
    let bad = dir.path().join("坏.psb");
    fs::write(&bad, b"8BPB\x00").unwrap();
    assert!(decode_composite(&bad).is_none());
  }

  /// 真实文件手动验证（#[ignore]）：cargo test -- --ignored real_psb_decodes
  /// 样本 = 公司工具链导出的 8BPS+version2 变体（RGBA 879×1016，RLE）
  #[test]
  #[ignore]
  fn real_psb_decodes() {
    let f = std::path::Path::new(r"E:/02_Assets/测试/asstes/AN2607230147-C8JQ7.info/闹钟.psb");
    let img = decode_composite(f).expect("真实 PSB 应解码成功");
    assert_eq!((img.width(), img.height()), (879, 1016));
    // RGBA 素材应有透明像素（alpha 平面真实读出，非恒 255）
    let has_transparent = img.pixels().any(|p| p[3] < 255);
    assert!(has_transparent, "alpha 通道应生效");
  }
}
