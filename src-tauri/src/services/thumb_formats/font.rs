//! 字体适配器（5.6）：ttf / otf / ttc 缩略图 = 样张渲染（services/font，fontdue 光栅化）。
//! probe 返回样张画布尺寸——字体无内容固有尺寸，width/height 元数据记样张尺寸
//!（与音频波形记 WAVEFORM 尺寸同手法，2026-07-26 先例）；解析失败 None 由 thumbnail 回退格式图标。

use std::path::Path;

use image::RgbaImage;

use super::FormatAdapter;
use crate::services::font;

pub const ADAPTER: FormatAdapter = FormatAdapter { exts: &["ttf", "otf", "ttc"], probe: probe_font, decode: decode_font };

fn probe_font(path: &Path) -> Option<(u32, u32)> {
  font::probe_font(path).map(|_| (font::SPECIMEN_W, font::SPECIMEN_H))
}

fn decode_font(path: &Path) -> Option<RgbaImage> {
  font::render_specimen(path, font::SPECIMEN_W, font::SPECIMEN_H)
}
