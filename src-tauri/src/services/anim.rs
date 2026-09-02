//! 动图（GIF / WEBP 动图）探测与逐帧解码：纯 Rust，无 ffmpeg 依赖。
//! probe：GIF 走自研字节级扫描（2026-07-26：image crate into_frames 全量解码像素数帧，
//! 大 GIF 导入等半天——字节扫描只读块结构不解像素，微秒级返回）；WEBP 走 image-webp 元数据。
//! decode_frames 流式逐帧回调：单趟解码供**帧流 webp 桥（frame_stream）与动图帧条
//! （preview_queue::AnimStrips）**；decode_frame 单帧解码供设封面 / 笔记帧画面抽帧
//! （2026-08-20 解包废除后，三者全部对 origin 运行时解码）。静态 gif/webp（单帧）按静态图片处理。

use std::path::Path;

use image::{AnimationDecoder, RgbaImage};

use crate::core::models::AssetMeta;

/// 动图信息：导入时写 metadata（width/height/duration/frameCount）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimInfo {
  pub width: u32,
  pub height: u32,
  pub frame_count: u32,
  pub duration_secs: f64,
}

/// 探测动图信息；非 gif/webp、静态（单帧）、解析失败均返回 None（"能力探测"语义）
pub fn probe(path: &Path) -> Option<AnimInfo> {
  match ext_of(path).as_str() {
    "gif" => probe_gif(path),
    "webp" => probe_webp(path),
    _ => None,
  }
}

/// 动图资产判定：gif/webp 且已 probe 出多帧（frame_count 有值，导入时 probe 写入）。
/// 帧流路由 / list_sequence_frames 拒绝分支 / 封面帧抽帧分支 / 笔记抽帧分支共用同一口径
pub fn is_anim(meta: &AssetMeta) -> bool {
  meta.frame_count.is_some() && matches!(meta.ext.to_lowercase().as_str(), "gif" | "webp")
}

/// 流式解码全部帧：逐帧回调（帧序号 0 起、RGBA 完整帧、该帧显示时长 ms），回调返回 false 中止。
/// 单趟解码（替代 decode_frame 每帧全量重解码的 O(n²)）。
/// GIF 迭代器内部已完成 disposal/画布合成（image 0.25 GifFrameIterator 直接产完整画面），
/// 与 decode_frame 的 collect_frames 语义一致；单帧解码失败跳过并 warn（序号仍占位），不整体失败。
/// 非 gif/webp / 打开失败返回 None。
pub fn decode_frames(path: &Path, mut on_frame: impl FnMut(u32, RgbaImage, u32) -> bool) -> Option<()> {
  match ext_of(path).as_str() {
    "gif" => stream_gif(path, &mut on_frame),
    "webp" => stream_webp(path, &mut on_frame),
    _ => None,
  }
}

/// 解码第 index 帧（0 起）与其显示时长（ms）；越界 / 非动图 / 失败返回 None。
/// 设封面（thumb_frame）与笔记帧画面（commands/notes）抽帧入口
pub fn decode_frame(path: &Path, index: u32) -> Option<(RgbaImage, u32)> {
  let mut hit = None;
  decode_frames(path, |i, img, delay| {
    if i == index {
      hit = Some((img, delay));
      return false;
    }
    true
  })?;
  hit
}

fn ext_of(path: &Path) -> String {
  path.extension().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default()
}

/// GIF 字节级探测（2026-07-26）：只解析块结构不解像素——帧数 = 图像描述块（0x2C）次数，
/// 时长 = 图形控制扩展（0x21 0xF9）delay（1/100 秒）累加。任何结构畸形（截断 / 未知块）返回 None
fn probe_gif(path: &Path) -> Option<AnimInfo> {
  let data = std::fs::read(path).ok()?;
  scan_gif(&data)
}

/// GIF 块结构扫描：Header(6B) + 逻辑屏幕描述(7B，拿宽高) + [全局色表] + 块循环
/// （0x21 扩展块 / 0x2C 图像描述块 / 0x3B 结束）
fn scan_gif(d: &[u8]) -> Option<AnimInfo> {
  if d.len() < 13 || !matches!(&d[..6], b"GIF87a" | b"GIF89a") {
    return None;
  }
  let width = u16::from_le_bytes([d[6], d[7]]) as u32;
  let height = u16::from_le_bytes([d[8], d[9]]) as u32;
  let mut pos = 13 + color_table_len(d[10]); // 全局色表（无则 0）
  let mut frame_count = 0u32;
  let mut ms = 0u64;
  while pos < d.len() {
    match d[pos] {
      0x3B => break, // 结束块
      0x21 => {
        // 扩展块：图形控制扩展（0xF9）拿 delay，其余（应用 / 注释 / 纯文本）直接跳子块链
        let label = *d.get(pos + 1)?;
        if label == 0xF9 && d.get(pos + 2) == Some(&4) {
          // [0xF9][块长=4][packed + delay(2B LE, 1/100s) + 透明色索引][0]
          ms += u16::from_le_bytes([*d.get(pos + 4)?, *d.get(pos + 5)?]) as u64 * 10;
        }
        pos = skip_sub_blocks(d, pos + 2)?;
      }
      0x2C => {
        // 图像描述块（一帧）：10B 描述 + [局部色表] + LZW 最小码长 1B + 子块链
        frame_count += 1;
        let packed = *d.get(pos + 9)?;
        pos += 10 + color_table_len(packed);
        pos = skip_sub_blocks(d, pos + 1)?; // +1 = LZW 最小码长
      }
      _ => return None, // 未知块 = 畸形
    }
  }
  if frame_count < 2 {
    return None;
  }
  Some(AnimInfo { width, height, frame_count, duration_secs: ms as f64 / 1000.0 })
}

/// 色表字节数：packed 最高位为 1 时存在，大小 = 3 * 2^(低 3 位 + 1)
fn color_table_len(packed: u8) -> usize {
  if packed & 0x80 == 0 { 0 } else { 3 * (1usize << ((packed & 0x07) + 1)) }
}

/// 跳过子块链：循环读 [size][size 字节] 直到 size=0，返回链后位置；越界（畸形）返回 None
fn skip_sub_blocks(d: &[u8], mut pos: usize) -> Option<usize> {
  loop {
    let size = *d.get(pos)? as usize;
    pos += 1;
    if size == 0 {
      return Some(pos);
    }
    pos = pos.checked_add(size)?;
    if pos > d.len() {
      return None;
    }
  }
}

/// GIF 流式解码：into_frames 逐帧迭代（单趟）。image 0.25 的 GifFrameIterator 内部按
/// disposal 合成完整画布帧（left/top 恒 0），逐帧即完整画面，与 collect_frames 输出一致
fn stream_gif(path: &Path, on_frame: &mut impl FnMut(u32, RgbaImage, u32) -> bool) -> Option<()> {
  let decoder = image::codecs::gif::GifDecoder::new(std::io::BufReader::new(std::fs::File::open(path).ok()?)).ok()?;
  for (i, frame) in decoder.into_frames().enumerate() {
    let i = i as u32; // 失败帧也占号：帧名与 frameCount 预生成占位清单一一对应，坏帧维持 null 兜底
    let frame = match frame {
      Ok(f) => f,
      Err(e) => {
        log::warn!("GIF 第 {i} 帧解码失败，跳过（{}）：{e}", path.display());
        continue;
      }
    };
    let delay = delay_ms(&frame);
    if !on_frame(i, frame.into_buffer(), delay) {
      return Some(());
    }
  }
  Some(())
}

fn delay_ms(frame: &image::Frame) -> u32 {
  let (n, d) = frame.delay().numer_denom_ms();
  n / d.max(1)
}

fn probe_webp(path: &Path) -> Option<AnimInfo> {
  let decoder = image_webp::WebPDecoder::new(std::io::BufReader::new(std::fs::File::open(path).ok()?)).ok()?;
  if !decoder.is_animated() {
    return None;
  }
  let (width, height) = decoder.dimensions();
  Some(AnimInfo {
    width,
    height,
    frame_count: decoder.num_frames(),
    duration_secs: decoder.loop_duration() as f64 / 1000.0,
  })
}

/// WEBP 动图流式解码：read_frame 逐帧读（单趟），libwebp 动画解码器输出完整画布帧
fn stream_webp(path: &Path, on_frame: &mut impl FnMut(u32, RgbaImage, u32) -> bool) -> Option<()> {
  let mut decoder = image_webp::WebPDecoder::new(std::io::BufReader::new(std::fs::File::open(path).ok()?)).ok()?;
  if !decoder.is_animated() {
    return None;
  }
  let (w, h) = decoder.dimensions();
  let bpp = if decoder.has_alpha() { 4usize } else { 3 };
  let mut buf = vec![0u8; decoder.output_buffer_size()?];
  let pixels = w as usize * h as usize * bpp;
  for index in 0..decoder.num_frames() {
    let delay = match decoder.read_frame(&mut buf) {
      Ok(d) => d,
      Err(e) => {
        log::warn!("WEBP 第 {index} 帧解码失败，跳过（{}）：{e}", path.display());
        continue;
      }
    };
    let img = if bpp == 4 {
      RgbaImage::from_raw(w, h, buf[..pixels].to_vec())?
    } else {
      image::DynamicImage::ImageRgb8(image::RgbImage::from_raw(w, h, buf[..pixels].to_vec())?).to_rgba8()
    };
    if !on_frame(index, img, delay) {
      return Some(());
    }
  }
  Some(())
}

// —— WebP 动画组装（纯 Rust 全帧组包，2026-09-01 CEP 导出落地 / 2026-09-02 收编本域共用）——
// 背景（实机踩坑三连）：① ffmpeg libwebp_anim 合成输出差分帧（帧矩形裁剪 + blend/dispose
// 混合档），image-webp 的画布合成对其处理缺陷 → 播放重影；② 组包 flags 写 0（alpha 混合），
// 透明区混合 = 保留画布旧内容 → 移动残影；③ **ffmpeg libwebp 差分编码产物本身带残影**
// （Pillow 规范合成验证，2026-09-02 八轮——小动画 ffmpeg 转码链因此整体退役换本组包）。
// 编码端根治终版：逐帧 webp 单帧编码 + 手写 RIFF 组包——每帧**全画布矩形 + flags bit1=1
// （do not blend，源像素含透明区直接替换画布）**，每帧画布即该帧完整内容，任何解码端都不可能叠影；
// 同时摘掉对 ffmpeg sidecar 的依赖。

/// WebP 动画帧（解码后 RGBA + 帧时长 ms）
#[derive(Clone)]
pub struct AnimFrame {
  pub rgba: RgbaImage,
  pub delay_ms: u32,
}

/// 从单帧 webp 字节（完整 RIFF 文件）提取图像 chunk 序列（ALPH（有损 alpha 伴随块，**必须保留**——
/// 2026-09-01 黑底回归：初版只提 VP8 丢 ALPH 致透明丢）+ VP8/VP8L，按序拼接；VP8X/ICCP 等元数据跳过）
fn extract_image_chunks(webp_file: &[u8]) -> Option<Vec<u8>> {
  if webp_file.len() < 16 || &webp_file[0..4] != b"RIFF" || &webp_file[8..12] != b"WEBP" {
    return None;
  }
  let mut out = Vec::new();
  let mut has_image = false;
  let mut i = 12;
  while i + 8 <= webp_file.len() {
    let fourcc = &webp_file[i..i + 4];
    let size = u32::from_le_bytes(webp_file[i + 4..i + 8].try_into().ok()?) as usize;
    let end = i + 8 + size + (size & 1); // chunk 偶对齐 pad
    if end > webp_file.len() {
      return None;
    }
    if fourcc == b"ALPH" || fourcc == b"VP8 " || fourcc == b"VP8L" {
      out.extend_from_slice(&webp_file[i..end]);
      has_image = true;
    }
    i = end;
  }
  has_image.then_some(out)
}

fn push_u24le(out: &mut Vec<u8>, v: u32) {
  out.extend_from_slice(&v.to_le_bytes()[0..3]);
}

fn push_chunk(out: &mut Vec<u8>, fourcc: &[u8; 4], data: &[u8]) {
  out.extend_from_slice(fourcc);
  out.extend_from_slice(&(data.len() as u32).to_le_bytes());
  out.extend_from_slice(data);
  if data.len() % 2 == 1 {
    out.push(0); // RIFF chunk 偶对齐
  }
}

/// 组 WebP 动画（全帧矩形 + blend=0 + loop 0 无限循环）：帧尺寸必须一致（调用方先缩放）。
/// 逐帧编码 rayon 并行（帧间独立无共享态，保序 collect 组包）；libwebp method 2
/// （动画缩略图不追极致压缩比，体积由调用方降级链兜底）。
pub fn encode_webp_anim(frames: &[AnimFrame], quality: f32) -> Option<Vec<u8>> {
  let first = frames.first()?;
  let (w, h) = (first.rgba.width(), first.rgba.height());
  // 并行逐帧编码（保序：par_iter collect 回 Vec 保持原序）
  use rayon::prelude::*;
  let chunks: Vec<Vec<u8>> = frames
    .par_iter()
    .map(|f| {
      let mut config = webp::WebPConfig::new().ok()?;
      config.quality = quality;
      config.method = 2; // 0=最快…4=默认…6=最慢最小；动画缩略图取快档
      let mem = webp::Encoder::from_rgba(f.rgba.as_raw(), w, h).encode_advanced(&config).ok()?;
      extract_image_chunks(&mem)
    })
    .collect::<Option<Vec<_>>>()?;
  let mut body: Vec<u8> = Vec::new();
  // VP8X：动画位 0x02 + alpha 位 0x10（恒带 alpha 通道编码，白底帧 alpha 全满也无妨）
  let mut vp8x: Vec<u8> = vec![0x12, 0, 0, 0];
  push_u24le(&mut vp8x, w - 1);
  push_u24le(&mut vp8x, h - 1);
  push_chunk(&mut body, b"VP8X", &vp8x);
  // ANIM：背景色 BGRA(白,不透明) + loop=0 无限
  push_chunk(&mut body, b"ANIM", &[0xff, 0xff, 0xff, 0xff, 0, 0]);
  for (f, chunk) in frames.iter().zip(chunks) {
    let mut anmf: Vec<u8> = Vec::with_capacity(16 + chunk.len());
    push_u24le(&mut anmf, 0); // x
    push_u24le(&mut anmf, 0); // y
    push_u24le(&mut anmf, w - 1);
    push_u24le(&mut anmf, h - 1);
    push_u24le(&mut anmf, f.delay_ms);
    anmf.push(0b10); // flags：bit1=1 do not blend（源像素**含透明区**直接替换画布——
    // 全帧矩形 + 源覆盖 = 每帧画布即该帧完整内容，任何解码端不可能叠影。
    // 注意 bit1=0（alpha 混合）才是坑：透明区混合 = 保留画布旧内容 = 残影，2026-09-01 实机钉）
    anmf.extend_from_slice(&chunk);
    push_chunk(&mut body, b"ANMF", &anmf);
  }
  let mut out: Vec<u8> = Vec::with_capacity(body.len() + 12);
  out.extend_from_slice(b"RIFF");
  out.extend_from_slice(&(body.len() as u32 + 4).to_le_bytes());
  out.extend_from_slice(b"WEBP");
  out.extend_from_slice(&body);
  Some(out)
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::path::PathBuf;

  /// 造 3 帧测试 gif（红/绿/蓝各 200ms）
  fn make_gif(path: &Path, frames: u32) {
    let colors = [[220, 40, 40, 255], [40, 200, 40, 255], [40, 40, 220, 255]];
    let file = std::fs::File::create(path).unwrap();
    let mut enc = image::codecs::gif::GifEncoder::new(file);
    enc.set_repeat(image::codecs::gif::Repeat::Infinite).unwrap();
    let frames: Vec<image::Frame> = colors
      .iter()
      .take(frames as usize)
      .map(|c| image::Frame::from_parts(RgbaImage::from_pixel(64, 48, image::Rgba(*c)), 0, 0, image::Delay::from_numer_denom_ms(200, 1)))
      .collect();
    enc.encode_frames(frames).unwrap();
  }

  #[test]
  fn probe_gif_reads_frames_and_duration() {
    let dir = tempfile::tempdir().unwrap();
    let gif = dir.path().join("动图.gif");
    make_gif(&gif, 3);
    let info = probe(&gif).unwrap();
    assert_eq!((info.width, info.height), (64, 48));
    assert_eq!(info.frame_count, 3);
    assert!((info.duration_secs - 0.6).abs() < 0.01);
  }

  #[test]
  fn probe_returns_none_for_static_gif_and_non_anim() {
    let dir = tempfile::tempdir().unwrap();
    let gif = dir.path().join("静态.gif");
    make_gif(&gif, 1);
    assert_eq!(probe(&gif), None, "单帧 gif 按静态图片处理");

    let txt = dir.path().join("文档.txt");
    std::fs::write(&txt, "hello").unwrap();
    assert_eq!(probe(&txt), None);
  }

  /// 旧解码路径参照（image crate 全量解码数帧）：交叉校验字节级扫描的 frameCount / duration
  fn decode_path_reference(path: &Path) -> (u32, u64) {
    let decoder =
      image::codecs::gif::GifDecoder::new(std::io::BufReader::new(std::fs::File::open(path).unwrap())).unwrap();
    let mut count = 0u32;
    let mut ms = 0u64;
    for frame in decoder.into_frames() {
      let frame = frame.unwrap();
      count += 1;
      ms += delay_ms(&frame) as u64;
    }
    (count, ms)
  }

  /// 字节级 probe_gif 与 image crate 全量解码结果交叉校验（样本 + 真实文件）
  #[test]
  fn probe_gif_byte_scan_matches_image_crate_decode() {
    let dir = tempfile::tempdir().unwrap();
    let gif = dir.path().join("动图.gif");
    make_gif(&gif, 3);
    for path in [gif, sample("anim3.gif")] {
      let info = probe_gif(&path).unwrap();
      let (count, ms) = decode_path_reference(&path);
      assert_eq!(info.frame_count, count, "{} 帧数一致", path.display());
      assert_eq!(info.duration_secs, ms as f64 / 1000.0, "{} 时长一致", path.display());
    }
  }

  #[test]
  fn probe_gif_truncated_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let gif = dir.path().join("动图.gif");
    make_gif(&gif, 3);
    let full = std::fs::read(&gif).unwrap();
    let truncated = dir.path().join("截断.gif");
    std::fs::write(&truncated, &full[..full.len() / 2]).unwrap();
    assert_eq!(probe(&truncated), None, "截断畸形文件返回 None（能力探测语义）");
  }

  #[test]
  fn decode_gif_frame_returns_frame_and_delay() {
    let dir = tempfile::tempdir().unwrap();
    let gif = dir.path().join("动图.gif");
    make_gif(&gif, 3);
    let (img, delay) = decode_frame(&gif, 1).unwrap();
    assert_eq!(delay, 200);
    assert_eq!((img.width(), img.height()), (64, 48));
    let p = img.get_pixel(10, 10);
    assert!(p[1] > 150 && p[0] < 100, "第 2 帧应为绿色");
    assert!(decode_frame(&gif, 3).is_none(), "越界返回 None");
  }

  /// 测试样本：tools/test-assets/anim3.webp（tools/gen-anim-samples.py 生成，3 帧各 200ms）
  fn sample(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/test-assets").join(name)
  }

  #[test]
  fn probe_webp_anim() {
    let info = probe(&sample("anim3.webp")).expect("anim3.webp 应判定为动图");
    assert_eq!((info.width, info.height), (64, 48));
    assert_eq!(info.frame_count, 3);
    assert!((info.duration_secs - 0.6).abs() < 0.01);
  }

  #[test]
  fn probe_webp_still_returns_none() {
    assert_eq!(probe(&sample("still.webp")), None, "静态 webp 按静态图片处理");
  }

  #[test]
  fn decode_webp_frame_returns_frame_and_delay() {
    let (img, delay) = decode_frame(&sample("anim3.webp"), 2).expect("第 3 帧应可解码");
    assert_eq!(delay, 200);
    let p = img.get_pixel(10, 10);
    assert!(p[2] > 150 && p[0] < 120, "第 3 帧应为蓝色（有损编码容差）");
  }

  #[test]
  fn decode_frames_streams_gif_in_order_with_delays() {
    let dir = tempfile::tempdir().unwrap();
    let gif = dir.path().join("动图.gif");
    make_gif(&gif, 3);
    let mut seen: Vec<(u32, u32, [u8; 4])> = Vec::new();
    let done = decode_frames(&gif, |i, img, delay| {
      seen.push((i, delay, img.get_pixel(10, 10).0));
      true
    });
    assert!(done.is_some());
    assert_eq!(seen.len(), 3, "3 帧应逐帧回调");
    assert_eq!(seen.iter().map(|f| f.0).collect::<Vec<_>>(), [0, 1, 2], "帧序号按序");
    assert!(seen.iter().all(|f| f.1 == 200), "每帧 delay 200ms");
    assert!(seen[0].2[0] > 150 && seen[1].2[1] > 150 && seen[2].2[2] > 150, "红/绿/蓝顺序");
  }

  #[test]
  fn decode_frames_aborts_when_callback_returns_false() {
    let dir = tempfile::tempdir().unwrap();
    let gif = dir.path().join("动图.gif");
    make_gif(&gif, 3);
    let mut count = 0;
    let done = decode_frames(&gif, |_, _, _| {
      count += 1;
      false // 首帧即中止（解码方主动停解码的路径）
    });
    assert!(done.is_some());
    assert_eq!(count, 1, "中止后不应继续解码后续帧");
  }

  #[test]
  fn decode_frames_streams_real_samples() {
    for name in ["anim3.gif", "anim3.webp"] {
      let mut seen: Vec<(u32, u32)> = Vec::new();
      decode_frames(&sample(name), |i, _img, delay| {
        seen.push((i, delay));
        true
      })
      .unwrap_or_else(|| panic!("{name} 应流式解码成功"));
      assert_eq!(seen, [(0, 200), (1, 200), (2, 200)], "{name} 3 帧各 200ms");
    }
  }

  #[test]
  fn decode_frames_rejects_non_anim() {
    let txt = Path::new("文档.txt");
    assert!(decode_frames(txt, |_, _, _| true).is_none());
    assert!(decode_frames(&sample("still.webp"), |_, _, _| true).is_none(), "静态 webp 不是动图");
  }
}
