//! ffmpeg-next 解码适配（阶段四·自研解码器）：GIF / 视频 → 逐帧 RGBA + 帧时长。
//! 动图 webp 不归此（BtbN lgpl 构建无 libwebp decoder，踩坑 039）——走 services/anim（image-webp）。
//! 链路口径与 spike/decode-proto 一致（已实测：GIF 50 路 3110fps / H264 50 路 7740fps @320p）。
//! （5.3 视频/音频引擎已随视频播放定线 <video> 直读删除，2026-09-07——方案与代码存档见 git 历史）

use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};

use ffmpeg_next as ffmpeg;

/// 一帧解码产物（RGBA 紧凑字节，短边等比缩放后）
pub struct DecodedFrame {
  pub w: u32,
  pub h: u32,
  pub delay_ms: u32, // 本帧展示时长（动图逐帧 delay）
  /// 本帧时刻（ms，趟内累计 delay——动图循环重开归零）
  pub pts_ms: f64,
  pub rgba: Vec<u8>,
}

/// 单趟解码：逐帧回调，返回 false 中止；解码/打开失败返回 Err。
/// 多线程并发安全（各线程独立 demuxer/decoder；spike bench 50 线程实证）。
/// short_edge = 目标短边（真短边口径，thumbnail::shrink_to_edge 同语义；不足不放大）——
/// 每帧读取，调用方可运行期改值热更新输出尺寸（帧流卡片尺寸跟随，2026-08-20 用户定）。
pub fn decode_once(
  path: &Path,
  short_edge: &AtomicU32,
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
  let mut cum_pts = 0.0f64; // 趟内累计 delay（pts_ms）

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
      &mut cum_pts,
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
    &mut cum_pts,
    on_frame,
  );
  Ok(())
}

/// 帧缩放到目标短边（真短边等比、不足不放大，thumbnail::shrink_to_edge 同口径）→ 紧凑 RGBA。
/// 目标尺寸变化时重建 scaler 与输出帧（旧 buffer 尺寸不符会让 run 报错丢帧）；单帧失败返回 None 跳过。
#[allow(clippy::too_many_arguments)]
fn scale_frame(
  scaler: &mut Option<ffmpeg::software::scaling::Context>,
  dims: &mut (u32, u32),
  frame: &ffmpeg::frame::Video,
  rgb: &mut ffmpeg::frame::Video,
  short_edge: &AtomicU32,
) -> Option<(u32, u32, Vec<u8>)> {
  let edge = short_edge.load(Ordering::Relaxed);
  let (sw, sh) = (frame.width(), frame.height());
  let nd = if sw.min(sh) <= edge {
    (sw, sh)
  } else if sw <= sh {
    (edge, (sh as u64 * edge as u64 / sw as u64) as u32)
  } else {
    ((sw as u64 * edge as u64 / sh as u64) as u32, edge)
  };
  let nd = (nd.0.max(2), nd.1.max(2));
  if *dims != nd || scaler.is_none() {
    *dims = nd;
    *rgb = ffmpeg::frame::Video::empty();
    *scaler = Some(
      ffmpeg::software::scaling::Context::get(
        frame.format(),
        sw,
        sh,
        ffmpeg::format::Pixel::RGBA,
        nd.0,
        nd.1,
        ffmpeg::software::scaling::Flags::BILINEAR,
      )
      .expect("创建 scaler 失败"),
    );
  }
  if scaler.as_mut().unwrap().run(frame, rgb).is_err() {
    return None;
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
  Some((w, h, rgba))
}

#[allow(clippy::too_many_arguments)]
fn drain(
  dec: &mut ffmpeg::decoder::Video,
  scaler: &mut Option<ffmpeg::software::scaling::Context>,
  dims: &mut (u32, u32),
  frame: &mut ffmpeg::frame::Video,
  rgb: &mut ffmpeg::frame::Video,
  short_edge: &AtomicU32,
  tb: (f64, f64),
  cum_pts: &mut f64,
  on_frame: &mut impl FnMut(DecodedFrame) -> bool,
) -> bool {
  while dec.receive_frame(frame).is_ok() {
    let Some((w, h, rgba)) = scale_frame(scaler, dims, frame, rgb, short_edge) else {
      continue; // 单帧缩放失败跳过，不整体失败
    };
    let delay_ms = (frame.packet().duration as f64 * tb.0 / tb.1 * 1000.0).max(0.0) as u32;
    let pts_ms = *cum_pts;
    *cum_pts += delay_ms as f64;
    if !on_frame(DecodedFrame { w, h, delay_ms, pts_ms, rgba }) {
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
    decode_once(&path, &AtomicU32::new(32), &mut |f| {
      frames.push((f.w, f.h, f.delay_ms, f.rgba.len()));
      true
    })
    .unwrap();
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].0, 42); // 真短边缩放：64x48 → 42x32（64*32/48=42）
    assert_eq!(frames[0].1, 32);
    assert_eq!(frames[0].2, 100); // delay 100ms
    assert_eq!(frames[0].3, 42 * 32 * 4);
    // 中止：第一帧返回 false 应只收 1 帧
    let mut n = 0;
    decode_once(&path, &AtomicU32::new(32), &mut |_f| {
      n += 1;
      false
    })
    .unwrap();
    assert_eq!(n, 1);
  }

  /// 不足不放大：源 64x48 目标 320 → 原尺寸输出；运行期改目标短边 → 下一帧即按新尺寸
  #[test]
  fn no_upscale_and_live_resize() {
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

    // 不足不放大
    let mut dims = Vec::new();
    decode_once(&path, &AtomicU32::new(320), &mut |f| {
      dims.push((f.w, f.h));
      true
    })
    .unwrap();
    assert_eq!(dims, vec![(64, 48), (64, 48)]);

    // 热更新：首帧回调改 320 → 16，第二帧按新尺寸（64x48 → 21x16）
    let edge = AtomicU32::new(320);
    let mut dims = Vec::new();
    decode_once(&path, &edge, &mut |f| {
      dims.push((f.w, f.h));
      edge.store(16, Ordering::Relaxed);
      true
    })
    .unwrap();
    assert_eq!(dims, vec![(64, 48), (21, 16)]);
  }

  /// pts 累计（gif 桥）：2 帧 100ms → pts 0 / 100
  #[test]
  fn gif_pts_accumulates() {
    use image::codecs::gif::{GifEncoder, Repeat};
    use image::{Frame as GifFrame, ImageBuffer, Rgba, RgbaImage};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.gif");
    let mut out = std::fs::File::create(&path).unwrap();
    let mut enc = GifEncoder::new(&mut out);
    enc.set_repeat(Repeat::Infinite).unwrap();
    for i in 0..2u8 {
      let img: RgbaImage = ImageBuffer::from_fn(64, 48, |x, _y| Rgba([i * 100, x as u8, 0, 255]));
      let f = GifFrame::from_parts(img, 0, 0, image::Delay::from_numer_denom_ms(100, 1));
      enc.encode_frame(f).unwrap();
    }
    drop(enc);
    drop(out);
    let mut pts = Vec::new();
    decode_once(&path, &AtomicU32::new(32), &mut |f| {
      pts.push(f.pts_ms);
      true
    })
    .unwrap();
    assert_eq!(pts, vec![0.0, 100.0]);
  }
}
