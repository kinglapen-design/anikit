//! Photoshop 适配器：PSD = psd crate 解文件内嵌合成图；PSB = 自研轻量合成图解析
//!（`services::psb`，psd crate 不支持 PSB）。宽高走轻量固定头解析，不整文件解码。
//! .psd 失败兜底走 psb.rs：存在 PSD 扩展名 + PSB 结构（version=2）的变体文件。

use std::fs;
use std::io::Read;
use std::path::Path;

use image::RgbaImage;

use super::FormatAdapter;
use crate::services::psb;

pub const ADAPTER: FormatAdapter = FormatAdapter { exts: &["psd", "psb"], probe: probe_photoshop, decode: decode_photoshop };

/// PSD / PSB 固定头（大端，两者布局一致）：签名(0-3，8BPS / 8BPB) + 版本(4-5) + 保留(6-11)
/// + 通道数(12-13) + 高(14-17) + 宽(18-21)
fn probe_photoshop(path: &Path) -> Option<(u32, u32)> {
  let mut head = [0u8; 22];
  fs::File::open(path).ok()?.read_exact(&mut head).ok()?;
  if &head[0..4] != b"8BPS" && &head[0..4] != b"8BPB" {
    return None;
  }
  let h = u32::from_be_bytes(head[14..18].try_into().ok()?);
  let w = u32::from_be_bytes(head[18..22].try_into().ok()?);
  (w > 0 && h > 0).then_some((w, h))
}

fn decode_photoshop(path: &Path) -> Option<RgbaImage> {
  decode_psd(path).or_else(|| psb::decode_composite(path))
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

#[cfg(test)]
mod tests {
  use super::*;

  /// 构造 PSD 头（签名 + 版本 + 保留 + 通道 + 高 + 宽），尾部补零
  fn make_fake_header(path: &Path, sig: &[u8; 4], w: u32, h: u32) {
    let mut data = vec![0u8; 64];
    data[0..4].copy_from_slice(sig);
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
    make_fake_header(&f, b"8BPS", 200, 100);
    assert_eq!(probe_photoshop(&f), Some((200, 100)));

    // PSB（8BPB）同布局同样可读
    let b = dir.path().join("大画.psb");
    make_fake_header(&b, b"8BPB", 4000, 3000);
    assert_eq!(probe_photoshop(&b), Some((4000, 3000)));

    // 签名不对不算
    let bad = dir.path().join("坏.psd");
    fs::write(&bad, b"not a psd at all, just some bytes").unwrap();
    assert_eq!(probe_photoshop(&bad), None);
  }

  #[test]
  fn decode_psd_returns_none_for_fake_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("假.psd");
    fs::write(&f, b"fake psd bytes").unwrap();
    assert!(decode_photoshop(&f).is_none());
  }
}
