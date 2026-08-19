//! ffmpeg-next 解码适配（阶段四·自研解码器）：GIF / 视频 → 逐帧 RGBA + 帧时长。
//! 动图 webp 不归此（BtbN lgpl 构建无 libwebp decoder，踩坑 039）——走 services/anim（image-webp）。
//! 链路口径与 spike/decode-proto 一致（已实测：GIF 50 路 3110fps / H264 50 路 7740fps @320p）。

use std::path::Path;

use ffmpeg_next as ffmpeg;

/// 一帧解码产物（RGBA 紧凑字节，短边等比缩放后）
pub struct DecodedFrame {
  pub w: u32,
  pub h: u32,
  pub delay_ms: u32, // 本帧展示时长（动图逐帧 delay；视频 = 帧距）
  pub rgba: Vec<u8>,
}

/// 单趟解码：逐帧回调，返回 false 中止；解码/打开失败返回 Err。
/// 多线程并发安全（各线程独立 demuxer/decoder；spike bench 50 线程实证）。
pub fn decode_once(
  path: &Path,
  short_edge: u32,
  on_frame: &mut impl FnMut(DecodedFrame) -> bool,
) -> Result<(), String> {
  ffmpeg::init().map_err(|e| format!("ffmpeg init 失败: {e:?}"))?;
  let mut ictx = ffmpeg::format::input(path).map_err(|e| format!("打开失败: {e:?}"))?;
  let stream = ictx
    .streams()
    .best(ffmpeg::media::Type::Video)
    .ok_or_else(|| "无视频流".to_string())?;
  let si = stream.index();
  let tb_num = stream.time_base().numerator() as f64;
  let tb_den = stream.time_base().denominator() as f64;
  let ctx = ffmpeg::codec::context::Context::from_parameters(stream.parameters())
    .map_err(|e| format!("解码上下文失败: {e:?}"))?;
  let mut dec = ctx.decoder().video().map_err(|e| format!("解码器失败: {e:?}"))?;

  let mut scaler: Option<ffmpeg::software::scaling::Context> = None;
  let mut dims = (0u32, 0u32);
  let mut frame = ffmpeg::frame::Video::empty();
  let mut rgb = ffmpeg::frame::Video::empty();

  for (s, packet) in ictx.packets() {
    if s.index() != si {
      continue;
    }
    dec.send_packet(&packet).map_err(|e| format!("送包失败: {e:?}"))?;
    if !drain(
      &mut dec,
      &mut scaler,
      &mut dims,
      &mut frame,
      &mut rgb,
      short_edge,
      (tb_num, tb_den),
      on_frame,
    ) {
      return Ok(());
    }
  }
  dec.send_eof().map_err(|e| format!("EOF 失败: {e:?}"))?;
  drain(
    &mut dec,
    &mut scaler,
    &mut dims,
    &mut frame,
    &mut rgb,
    short_edge,
    (tb_num, tb_den),
    on_frame,
  );
  Ok(())
}

#[allow(clippy::too_many_arguments)]
fn drain(
  dec: &mut ffmpeg::decoder::Video,
  scaler: &mut Option<ffmpeg::software::scaling::Context>,
  dims: &mut (u32, u32),
  frame: &mut ffmpeg::frame::Video,
  rgb: &mut ffmpeg::frame::Video,
  short_edge: u32,
  tb: (f64, f64),
  on_frame: &mut impl FnMut(DecodedFrame) -> bool,
) -> bool {
  while dec.receive_frame(frame).is_ok() {
    if scaler.is_none() {
      let (mut w, mut h) = (frame.width(), frame.height());
      if w >= h {
        h = (h * short_edge / w) & !1;
        w = short_edge;
      } else {
        w = (w * short_edge / h) & !1;
        h = short_edge;
      }
      *dims = (w.max(2), h.max(2));
      *scaler = Some(
        ffmpeg::software::scaling::Context::get(
          frame.format(),
          frame.width(),
          frame.height(),
          ffmpeg::format::Pixel::RGBA,
          dims.0,
          dims.1,
          ffmpeg::software::scaling::Flags::BILINEAR,
        )
        .expect("创建 scaler 失败"),
      );
    }
    if scaler.as_mut().unwrap().run(frame, rgb).is_err() {
      return true; // 单帧缩放失败跳过，不整体失败
    }
    // linesize 可能大于 w*4（对齐），逐行拷出紧凑 RGBA
    let (w, h) = *dims;
    let row = (w * 4) as usize;
    let stride = rgb.stride(0);
    let data = rgb.data(0);
    let mut rgba = Vec::with_capacity(row * h as usize);
    for y in 0..h as usize {
      rgba.extend_from_slice(&data[y * stride..y * stride + row]);
    }
    let delay_ms = (frame.packet().duration as f64 * tb.0 / tb.1 * 1000.0).max(0.0) as u32;
    if !on_frame(DecodedFrame { w, h, delay_ms, rgba }) {
      return false;
    }
  }
  true
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 造一个 2 帧 GIF（image crate 编码），解码应出 2 帧 RGBA、短边缩放正确
  #[test]
  fn decode_gif_frames() {
    use image::codecs::gif::{GifEncoder, Repeat};
    use image::{Frame as GifFrame, ImageBuffer, Rgba, RgbaImage};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.gif");
    let mut out = std::fs::File::create(&path).unwrap();
    let mut enc = GifEncoder::new(&mut out);
    enc.set_repeat(Repeat::Infinite).unwrap();
    for i in 0..2u8 {
      let img: RgbaImage =
        ImageBuffer::from_fn(64, 48, |x, _y| Rgba([i * 100, x as u8, 0, 255]));
      let f = GifFrame::from_parts(img, 0, 0, image::Delay::from_numer_denom_ms(100, 1));
      enc.encode_frame(f).unwrap();
    }
    drop(enc);
    drop(out);

    let mut frames = Vec::new();
    decode_once(&path, 32, &mut |f| {
      frames.push((f.w, f.h, f.delay_ms, f.rgba.len()));
      true
    })
    .unwrap();
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].0, 32); // 短边缩放：64x48 → 32x24
    assert_eq!(frames[0].1, 24);
    assert_eq!(frames[0].2, 100); // delay 100ms
    assert_eq!(frames[0].3, 32 * 24 * 4);
    // 中止：第一帧返回 false 应只收 1 帧
    let mut n = 0;
    decode_once(&path, 32, &mut |_f| {
      n += 1;
      false
    })
    .unwrap();
    assert_eq!(n, 1);
  }
}
