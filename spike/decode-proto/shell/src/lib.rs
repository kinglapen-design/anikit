pub mod decode {
    use ffmpeg_next as ffmpeg;

    pub struct RgbaFrame {
        pub w: u32,
        pub h: u32,
        pub delay_ms: u32, // 本帧展示时长（动图逐帧 delay；视频 = 帧距）
        pub rgba: Vec<u8>,
    }

    /// 循环解码文件，逐帧回调（RGBA，短边等比缩放 short_edge）。永不返回（出错打印后重开）。
    pub fn decode_loop(path: &str, short_edge: u32, mut on_frame: impl FnMut(RgbaFrame)) {
        loop {
            if let Err(e) = decode_once(path, short_edge, &mut on_frame) {
                eprintln!("解码一轮失败 {e:?}，重开");
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }

    fn decode_once(
        path: &str,
        short_edge: u32,
        on_frame: &mut impl FnMut(RgbaFrame),
    ) -> Result<(), ffmpeg::Error> {
        let mut ictx = ffmpeg::format::input(path)?;
        let stream = ictx
            .streams()
            .best(ffmpeg::media::Type::Video)
            .expect("无视频流");
        let si = stream.index();
        let tb_num = stream.time_base().numerator() as f64;
        let tb_den = stream.time_base().denominator() as f64;
        let ctx = ffmpeg::codec::context::Context::from_parameters(stream.parameters())?;
        let mut dec = ctx.decoder().video()?;
        let mut scaler: Option<ffmpeg::software::scaling::Context> = None;
        let mut dims = (0u32, 0u32);
        let mut frame = ffmpeg::frame::Video::empty();
        let mut rgb = ffmpeg::frame::Video::empty();

        for (s, packet) in ictx.packets() {
            if s.index() != si {
                continue;
            }
            dec.send_packet(&packet)?;
            drain_frames(&mut dec, &mut scaler, &mut dims, &mut frame, &mut rgb, short_edge, (tb_num, tb_den), on_frame);
        }
        dec.send_eof()?;
        drain_frames(&mut dec, &mut scaler, &mut dims, &mut frame, &mut rgb, short_edge, (tb_num, tb_den), on_frame);
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn drain_frames(
        dec: &mut ffmpeg::decoder::Video,
        scaler: &mut Option<ffmpeg::software::scaling::Context>,
        dims: &mut (u32, u32),
        frame: &mut ffmpeg::frame::Video,
        rgb: &mut ffmpeg::frame::Video,
        short_edge: u32,
        tb: (f64, f64),
        on_frame: &mut impl FnMut(RgbaFrame),
    ) {
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
            scaler.as_mut().unwrap().run(frame, rgb).expect("缩放失败");
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
            on_frame(RgbaFrame { w, h, delay_ms, rgba });
        }
    }
}
