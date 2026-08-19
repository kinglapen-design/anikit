//! 阶段四 spike：ffmpeg-next 解码探针。
//! 用法：
//!   decode-proto <媒体文件>                   单路解码摸底（原尺寸 RGBA）
//!   decode-proto --list                       探关键 decoder 注册情况
//!   decode-proto --bench <文件> <路数> <秒数> N 路并发循环解码吞吐（缩放短边 320，卡片实况）

use ffmpeg_next as ffmpeg;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// 卡片实况：解码输出缩放到短边 320（与主仓 THUMB_SHORT_EDGE / anim_preview 口径一致）
const CARD_SHORT_EDGE: u32 = 320;

#[derive(Default)]
struct Stats {
    frames: u64,
    t_scale: Duration,
    first_pts: Vec<(i64, i64)>, // (pts, packet duration) 前五帧，看动图逐帧 delay 基准
}

fn drain(
    dec: &mut ffmpeg::decoder::Video,
    scaler: &mut Option<ffmpeg::software::scaling::Context>,
    st: &mut Stats,
    short_edge: Option<u32>,
) {
    let mut frame = ffmpeg::frame::Video::empty();
    let mut rgb = ffmpeg::frame::Video::empty();
    while dec.receive_frame(&mut frame).is_ok() {
        st.frames += 1;
        if st.first_pts.len() < 5 {
            st.first_pts
                .push((frame.pts().unwrap_or(-1), frame.packet().duration));
        }
        if scaler.is_none() {
            let (mut w, mut h) = (frame.width(), frame.height());
            if let Some(edge) = short_edge {
                // 短边等比缩到 edge，长边取偶（scaler 友好）
                if w >= h {
                    h = (h * edge / w) & !1;
                    w = edge;
                } else {
                    w = (w * edge / h) & !1;
                    h = edge;
                }
            }
            *scaler = Some(
                ffmpeg::software::scaling::Context::get(
                    frame.format(),
                    frame.width(),
                    frame.height(),
                    ffmpeg::format::Pixel::RGBA,
                    w.max(2),
                    h.max(2),
                    ffmpeg::software::scaling::Flags::BILINEAR,
                )
                .expect("创建 scaler 失败"),
            );
        }
        let t = Instant::now();
        scaler.as_mut().unwrap().run(&frame, &mut rgb).expect("缩放失败");
        st.t_scale += t.elapsed();
        std::hint::black_box(rgb.data(0)[0]); // 确保真写出不被优化掉
    }
}

/// 打开文件完整解码一遍（demuxer → decoder → 可选缩放 RGBA），返回统计。
fn decode_once(path: &str, short_edge: Option<u32>) -> Result<Stats, ffmpeg::Error> {
    let mut ictx = ffmpeg::format::input(path)?;
    let stream = ictx
        .streams()
        .best(ffmpeg::media::Type::Video)
        .expect("无视频流");
    let si = stream.index();

    let ctx = ffmpeg::codec::context::Context::from_parameters(stream.parameters())?;
    let mut dec = ctx.decoder().video()?;

    let mut scaler = None;
    let mut st = Stats::default();
    for (s, packet) in ictx.packets() {
        if s.index() != si {
            continue;
        }
        dec.send_packet(&packet)?;
        drain(&mut dec, &mut scaler, &mut st, short_edge);
    }
    dec.send_eof()?;
    drain(&mut dec, &mut scaler, &mut st, short_edge);
    Ok(st)
}

fn report(st: &Stats, total: Duration) {
    println!(
        "结果: {} 帧  总耗时 {:?}（解码+收包 {:?}，RGBA 转换 {:?}）  折合 {:.1} 帧/秒",
        st.frames,
        total,
        total - st.t_scale,
        st.t_scale,
        st.frames as f64 / total.as_secs_f64()
    );
    println!("前 5 帧 (pts, duration): {:?}", st.first_pts);
}

fn bench(path: &str, lanes: usize, secs: u64) {
    let total = Arc::new(AtomicU64::new(0));
    let deadline = Instant::now() + Duration::from_secs(secs);
    let mut handles = Vec::new();
    for _ in 0..lanes {
        let path = path.to_string();
        let total = Arc::clone(&total);
        handles.push(std::thread::spawn(move || {
            let mut local = 0u64;
            while Instant::now() < deadline {
                match decode_once(&path, Some(CARD_SHORT_EDGE)) {
                    Ok(st) => local += st.frames,
                    Err(e) => eprintln!("解码错误: {e:?}"),
                }
            }
            total.fetch_add(local, Ordering::Relaxed);
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    let frames = total.load(Ordering::Relaxed);
    let fps = frames as f64 / secs as f64;
    println!(
        "bench: {lanes} 路 × {secs}s  总帧数 {frames}  总吞吐 {fps:.0} 帧/秒  每路 {:.1} fps",
        fps / lanes as f64
    );
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).expect("用法: decode-proto <媒体文件> | --list | --bench <文件> <路数> <秒数>");
    ffmpeg::init()?;

    if path == "--list" {
        for name in ["libwebp_anim", "webp_anim", "libwebp", "webp", "gif"] {
            let found = ffmpeg::codec::decoder::find_by_name(name).map(|c| c.name().to_string());
            println!("{name}: {found:?}");
        }
        return Ok(());
    }

    if path == "--bench" {
        let file = args.get(2).expect("--bench 缺文件");
        let lanes: usize = args.get(3).and_then(|s| s.parse().ok()).expect("--bench 缺路数");
        let secs: u64 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(5);
        bench(file, lanes, secs);
        return Ok(());
    }

    if path.to_ascii_lowercase().ends_with(".webp") {
        // 本构建（BtbN lgpl）libwebp 只开 encoder，无 libwebp_anim decoder；
        // 原生 webp decoder 不认 ANIM/ANMF → 动图 webp 不归 ffmpeg 管线，
        // 走主线既有 image-webp 纯 Rust 解码（services/anim.rs 生产在跑）。
        println!("结论: 动图 webp 不进 ffmpeg 管线（lgpl 构建无 libwebp decoder），归 image-webp");
        return Ok(());
    }

    let t_all = Instant::now();
    let st = decode_once(path, None)?;
    report(&st, t_all.elapsed());
    Ok(())
}
