//! 缩略图服务：图片宽高探测 + 缩略图生成 + 格式图标缓存（4.2）。
//! 缩略图存 info 文件夹（{名}_thumbnail.webp），是缓存可重建、不进备份（PRD 06-04）。
//! 格式图标（shell 保底）按格式存 cache/thumbs/{ext}.webp，不进 info（2026-07-22 用户定案）。
//! 序列帧帧条小图按资源存 cache/frames/{asset_id}/{帧名}.webp，不进 info（会话级，打开库清空）。
//! cache 下的会话级目录（tmp / previews / frames）打开库时统一清空（2026-07-25 用户定）；thumbs 格式图标跨会话保留。
//! 探测 / 解码失败返回 None 属"能力探测"语义（不是图片），不算吞错误。

use std::fs;
use std::path::{Path, PathBuf};

use image::DynamicImage;

use crate::adapters::shell_thumb;
use crate::core::library::CACHE_DIR;
use crate::services::thumb_formats;

/// 缩略图短边长度（像素）：短边缩到 320、长边等比（对齐 Eagle 同级策略；
/// 768 长边对千像素级原图过剩，体积不合算）
pub const THUMB_SHORT_EDGE: u32 = 320;

/// 帧条小图短边长度（像素）：预览窗口序列帧播放条用，预生成存 cache/frames 不进 info
pub const STRIP_SHORT_EDGE: u32 = 128;

/// 全尺寸预览缓存长边上限（像素）：psd/psb 等 webview 直渲不了的格式，
/// 预览窗口"渐进加载"的高清层（Eagle 式：先秒显 info 缩略图，后台解析真尺寸替换，2026-07-25 用户定）
pub const FULL_PREVIEW_LONG_EDGE: u32 = 4096;

/// 有损 webp 质量（libwebp q80：目视接近无损，体积约为无损编码 1/5）
const WEBP_QUALITY: f32 = 80.0;

/// 探测图片宽高；非图片 / 探测失败返回 None（PSD/SVG 走 thumb_formats 补充探测）
pub fn probe(path: &Path) -> Option<(u32, u32)> {
  image::image_dimensions(path).ok().or_else(|| thumb_formats::probe(path))
}

/// 生成缩略图：短边缩到 320、长边等比（原图短边不足 320 不放大），
/// 优先存有损 webp，失败退回 PNG。gif 取首帧（image::open 行为即首帧）。
/// 返回缩略图文件名；非图片 / 解码失败返回 None。
pub fn generate(info_dir: &Path, origin: &Path, stem: &str) -> Option<String> {
  let img = load_origin(origin)?;
  let thumb = shrink_to_thumb(img);

  let webp_name = format!("{stem}_thumbnail.webp");
  if save_webp(&thumb, &info_dir.join(&webp_name)).is_ok() {
    return Some(webp_name);
  }
  log::warn!("webp 缩略图编码失败，退回 PNG：{}", origin.display());
  let png_name = format!("{stem}_thumbnail.png");
  if thumb.save(info_dir.join(&png_name)).is_ok() {
    return Some(png_name);
  }
  None
}

/// 原图解码：image crate（jpg/png/gif/webp/bmp）→ PSD/SVG 解码补充（thumb_formats，4.2）
fn load_origin(origin: &Path) -> Option<DynamicImage> {
  if let Ok(img) = image::open(origin) {
    return Some(img);
  }
  thumb_formats::decode(origin).map(DynamicImage::ImageRgba8)
}

/// 格式图标缓存路径（cache/thumbs/{ext}.webp；无扩展名归 "file" 键）。
/// 只组路径不判存在：供"只命中已缓存"的快照场景（收件箱实况快层）与落盘后回填复用
pub fn format_icon_path(root: &Path, ext: &str) -> PathBuf {
  let key = if ext.is_empty() { "file" } else { ext };
  root.join(CACHE_DIR).join("thumbs").join(format!("{key}.webp"))
}

/// 格式图标缓存（PRD 04-03：系统缩略图按格式缓存一份，cache/thumbs/{ext}.webp，**不进 info**——
/// 避免与后期补的伴生缩略图并存污染 info，2026-07-22 用户定案）。已存在直接 true；
/// 否则经 shell 适配器取系统图标落盘。返回该格式图标是否可用。
pub fn ensure_format_icon(root: &Path, path: &Path, ext: &str) -> bool {
  let key = if ext.is_empty() { "file" } else { ext };
  let dest = format_icon_path(root, ext);
  if dest.is_file() {
    return true;
  }
  let Some(img) = shell_thumb::format_icon(path, shell_thumb::FORMAT_ICON_EDGE) else {
    return false;
  };
  let Some(parent) = dest.parent() else {
    return false;
  };
  if let Err(e) = fs::create_dir_all(parent) {
    log::error!("格式图标缓存目录创建失败：{e}");
    return false;
  }
  if let Err(e) = save_webp(&DynamicImage::ImageRgba8(img), &dest) {
    log::error!("格式图标缓存落盘失败（{key}）：{e}");
    return false;
  }
  log::info!("格式图标已缓存：{key}");
  true
}

/// 冲突弹窗"新的文件"预览缓存（cache/tmp/{key}.webp）：webview 直渲不了的格式（PSD/PSB/SVG 等）
/// 经解码管线生成 320 短边 webp，成功返回绝对路径；非图片 / 解码失败 None（前端按扩展名占位）。
/// 不复用已有侧缩略图：将来支持剪贴板自定义缩略图后，hash 相同缩略图也可能不同（2026-07-23 用户定）。
/// 按 key 幂等（已存在直接返回），不主动清理——cache 属可重建缓存（PRD 06）
pub fn generate_preview(root: &Path, origin: &Path, key: &str) -> Option<String> {
  let name = format!("{key}.webp");
  let dest = root.join(CACHE_DIR).join("tmp").join(&name);
  if dest.is_file() {
    return Some(dest.to_string_lossy().into_owned());
  }
  let img = load_origin(origin)?;
  let thumb = shrink_to_thumb(img);
  let parent = dest.parent()?;
  if let Err(e) = fs::create_dir_all(parent) {
    log::error!("预览缓存目录创建失败：{e}");
    return None;
  }
  if let Err(e) = save_webp(&thumb, &dest) {
    log::error!("预览缓存落盘失败（{}）：{e}", origin.display());
    return None;
  }
  Some(dest.to_string_lossy().into_owned())
}

/// 全尺寸预览缓存目标路径（cache/previews/{asset_id}_{size}_{mtime}.webp，按源文件 大小+修改时间 散列）；
/// 源文件元数据读取失败返回 None
fn full_preview_dest(root: &Path, origin: &Path, asset_id: &str) -> Option<PathBuf> {
  let meta = fs::metadata(origin).ok()?;
  let mtime = meta.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
  let key = format!("{asset_id}_{}_{}", meta.len(), mtime);
  Some(root.join(CACHE_DIR).join("previews").join(format!("{key}.webp")))
}

/// 全尺寸预览缓存"只判存在不解码"版本：缓存命中返回路径，未命中 / 元数据读取失败返回 None。
/// 命令层用它决定直给路径还是进预览解析队列——generate_full_preview 未命中会同步解码（秒级），
/// 命令层直接调会阻塞 tokio 线程池（2026-07-25 预览解析队列改造）
pub fn full_preview_path(root: &Path, origin: &Path, asset_id: &str) -> Option<String> {
  let dest = full_preview_dest(root, origin, asset_id)?;
  dest.is_file().then(|| dest.to_string_lossy().into_owned())
}

/// 全尺寸预览缓存（cache/previews/{asset_id}_{size}_{mtime}.webp）：psd/psb 等 webview 直渲不了的
/// 格式，解码后长边 ≤ 4096 存 webp，作预览窗口渐进加载的高清层。**会话级缓存**（打开库时清空，
/// 2026-07-25 用户定：点开过的素材缓存不留到下一会话，防容量膨胀），按源文件 大小+修改时间 失效
/// （文件名散列，同资源旧缓存顺手清理）。解码失败返回 None。
pub fn generate_full_preview(root: &Path, origin: &Path, asset_id: &str) -> Option<String> {
  let dest = full_preview_dest(root, origin, asset_id)?;
  if dest.is_file() {
    return Some(dest.to_string_lossy().into_owned());
  }
  let img = load_origin(origin)?;
  let img = shrink_long_edge(img, FULL_PREVIEW_LONG_EDGE);
  let dir = dest.parent()?;
  if let Err(e) = fs::create_dir_all(dir) {
    log::error!("全尺寸预览缓存目录创建失败：{e}");
    return None;
  }
  if let Err(e) = save_webp(&img, &dest) {
    log::error!("全尺寸预览缓存落盘失败（{}）：{e}", origin.display());
    return None;
  }
  // 同资源旧缓存（源文件已变化）顺手清理
  let prefix = format!("{asset_id}_");
  if let Ok(entries) = fs::read_dir(dir) {
    for e in entries.flatten() {
      if e.file_name().to_string_lossy().starts_with(&prefix) && e.path() != dest {
        let _ = fs::remove_file(e.path());
      }
    }
  }
  Some(dest.to_string_lossy().into_owned())
}

/// 帧条缓存目标路径（cache/frames/{asset_id}/{帧名}.webp）
fn frame_strip_dest(root: &Path, asset_id: &str, frame_name: &str) -> PathBuf {
  root.join(CACHE_DIR).join("frames").join(asset_id).join(format!("{frame_name}.webp"))
}

/// 帧条缓存"只判存在不生成"版本：已生成返回路径，未生成返回 None。
/// 命令层用它填 stripPath——未缓存帧进预览解析队列后台生成（2026-07-25 预览解析队列改造）
pub fn frame_strip_path(root: &Path, asset_id: &str, frame_name: &str) -> Option<String> {
  let dest = frame_strip_dest(root, asset_id, frame_name);
  dest.is_file().then(|| dest.to_string_lossy().into_owned())
}

/// 序列帧帧条小图（cache/frames/{asset_id}/{帧名}.webp，短边 128 等比不放大）：
/// 预览窗口播放条逐帧预生成——存 cache 不进 info，会话级（打开库时清空，2026-07-25 用户定）。
/// 已存在直接返回路径；非图片 / 解码失败返回 None。
pub fn generate_frame_strip(root: &Path, origin: &Path, asset_id: &str, frame_name: &str) -> Option<String> {
  let dest = frame_strip_dest(root, asset_id, frame_name);
  if dest.is_file() {
    return Some(dest.to_string_lossy().into_owned());
  }
  let img = load_origin(origin)?;
  let strip = shrink_to_edge(img, STRIP_SHORT_EDGE);
  let parent = dest.parent()?;
  if let Err(e) = fs::create_dir_all(parent) {
    log::error!("帧条缓存目录创建失败：{e}");
    return None;
  }
  if let Err(e) = save_webp(&strip, &dest) {
    log::error!("帧条缓存落盘失败（{}）：{e}", origin.display());
    return None;
  }
  Some(dest.to_string_lossy().into_owned())
}

/// 短边缩到 THUMB_SHORT_EDGE、长边等比（整数除法 floor，短边恰好 320）；原图短边不足不放大
fn shrink_to_thumb(img: DynamicImage) -> DynamicImage {
  shrink_to_edge(img, THUMB_SHORT_EDGE)
}

/// 短边缩到 short_edge、长边等比（整数除法 floor）；原图短边不足不放大
fn shrink_to_edge(img: DynamicImage, short_edge: u32) -> DynamicImage {
  let (w, h) = (img.width(), img.height());
  if w.min(h) <= short_edge {
    return img;
  }
  let (nw, nh) = if w <= h {
    (short_edge, (h as u64 * short_edge as u64 / w as u64) as u32)
  } else {
    ((w as u64 * short_edge as u64 / h as u64) as u32, short_edge)
  };
  // imageops::resize 输出严格 nw×nh（thumbnail 的 fit-within 是长边语义，不合短边策略）
  let rgba = img.to_rgba8();
  DynamicImage::ImageRgba8(image::imageops::resize(&rgba, nw, nh, image::imageops::FilterType::Triangle))
}

/// 长边缩到 long_edge、短边等比；原图长边不足不放大
fn shrink_long_edge(img: DynamicImage, long_edge: u32) -> DynamicImage {
  let (w, h) = (img.width(), img.height());
  if w.max(h) <= long_edge {
    return img;
  }
  let (nw, nh) = if w >= h {
    (long_edge, (h as u64 * long_edge as u64 / w as u64) as u32)
  } else {
    ((w as u64 * long_edge as u64 / h as u64) as u32, long_edge)
  };
  let rgba = img.to_rgba8();
  DynamicImage::ImageRgba8(image::imageops::resize(&rgba, nw, nh, image::imageops::FilterType::Triangle))
}

/// 有损 webp 编码落盘（libwebp 官方绑定；image crate 的 WebPEncoder 只支持无损故不用）
fn save_webp(img: &DynamicImage, path: &Path) -> std::io::Result<()> {
  let rgba = img.to_rgba8();
  let mem = webp::Encoder::from_rgba(rgba.as_raw(), rgba.width(), rgba.height()).encode(WEBP_QUALITY);
  if mem.is_empty() {
    // libwebp 编码失败返回空缓冲，交上层退 PNG
    return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "libwebp 编码失败"));
  }
  fs::write(path, &*mem)
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 造一张纯色测试图
  fn make_png(path: &Path, w: u32, h: u32) {
    let img = image::RgbaImage::from_pixel(w, h, image::Rgba([200, 30, 30, 255]));
    img.save(path).unwrap();
  }

  #[test]
  fn probe_reads_image_dimensions() {
    let dir = tempfile::tempdir().unwrap();
    let png = dir.path().join("a.png");
    make_png(&png, 640, 480);
    assert_eq!(probe(&png), Some((640, 480)));

    // 非图片返回 None
    let txt = dir.path().join("a.txt");
    std::fs::write(&txt, "not an image").unwrap();
    assert_eq!(probe(&txt), None);
  }

  #[test]
  fn generate_shrinks_to_short_edge() {
    let dir = tempfile::tempdir().unwrap();

    // 横向 800x600 → 426x320（高缩到 320、宽等比）
    let land = dir.path().join("land.png");
    make_png(&land, 800, 600);
    let name = generate(dir.path(), &land, "land").unwrap();
    let out = dir.path().join(&name);
    assert_eq!(name, "land_thumbnail.webp");
    assert!(out.is_file());
    // 产物可再解码为 webp
    assert_eq!(image::image_dimensions(&out).unwrap(), (426, 320));
    // 有损压缩体积量级：纯色图应只有几 KB（无损 webp 同图为数十 KB 起）
    assert!(fs::metadata(&out).unwrap().len() < 50 * 1024, "缩略图体积异常");

    // 竖向 600x800 → 320x426（宽缩到 320、高等比）
    let port = dir.path().join("port.png");
    make_png(&port, 600, 800);
    let name = generate(dir.path(), &port, "port").unwrap();
    assert_eq!(image::image_dimensions(dir.path().join(&name)).unwrap(), (320, 426));

    // 方形 500x500 → 320x320
    let square = dir.path().join("square.png");
    make_png(&square, 500, 500);
    let name = generate(dir.path(), &square, "square").unwrap();
    assert_eq!(image::image_dimensions(dir.path().join(&name)).unwrap(), (320, 320));
  }

  #[test]
  fn generate_does_not_upscale_small_image() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("small.png");
    make_png(&src, 100, 50);

    let name = generate(dir.path(), &src, "small").unwrap();
    let (w, h) = image::image_dimensions(dir.path().join(&name)).unwrap();
    assert_eq!((w, h), (100, 50));
  }

  #[test]
  fn generate_returns_none_for_non_image() {
    let dir = tempfile::tempdir().unwrap();
    let txt = dir.path().join("工程.fla");
    std::fs::write(&txt, "fake fla bytes").unwrap();
    assert_eq!(generate(dir.path(), &txt, "工程"), None);
  }

  #[test]
  fn generate_full_preview_caches_by_size_and_mtime() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let origin = root.join("a.png");
    make_png(&origin, 640, 480);

    // 首次生成：640×480（长边不足 4096 保持原尺寸）
    let p1 = generate_full_preview(root, &origin, "A1").unwrap();
    assert!(p1.contains("cache") && p1.contains("previews"));
    assert_eq!(image::image_dimensions(&p1).unwrap(), (640, 480));

    // 二次调用幂等：路径相同、文件不变
    let mtime = fs::metadata(&p1).unwrap().modified().unwrap();
    let p2 = generate_full_preview(root, &origin, "A1").unwrap();
    assert_eq!(p1, p2);
    assert_eq!(fs::metadata(&p2).unwrap().modified().unwrap(), mtime);

    // 源文件变化（大小不同 → 散列变）→ 新缓存，旧缓存被清理
    make_png(&origin, 800, 600);
    let p3 = generate_full_preview(root, &origin, "A1").unwrap();
    assert_ne!(p1, p3);
    assert!(!Path::new(&p1).exists(), "旧缓存应被清理");
  }

  #[test]
  fn full_preview_path_only_checks_existence() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let origin = root.join("a.png");
    make_png(&origin, 640, 480);

    // 未生成：None（不解码、不落盘）
    assert_eq!(full_preview_path(root, &origin, "A1"), None);
    assert!(!root.join("cache").exists(), "只判存在不应创建任何目录");

    // 生成后命中，与 generate_full_preview 同 key 同路径
    let p = generate_full_preview(root, &origin, "A1").unwrap();
    assert_eq!(full_preview_path(root, &origin, "A1").as_deref(), Some(p.as_str()));

    // 源文件不存在：None
    assert_eq!(full_preview_path(root, &root.join("不存在.png"), "A1"), None);
  }

  #[test]
  fn frame_strip_path_only_checks_existence() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let frame = root.join("f1.png");
    make_png(&frame, 640, 480);

    assert_eq!(frame_strip_path(root, "A1", "f1.png"), None);
    let p = generate_frame_strip(root, &frame, "A1", "f1.png").unwrap();
    assert_eq!(frame_strip_path(root, "A1", "f1.png").as_deref(), Some(p.as_str()));
    assert_eq!(frame_strip_path(root, "A1", "f2.png"), None);
  }

  #[test]
  fn shrink_long_edge_caps_at_limit() {
    let dir = tempfile::tempdir().unwrap();
    let big = dir.path().join("big.png");
    make_png(&big, 8000, 4000);
    let img = image::open(&big).unwrap();
    let out = shrink_long_edge(img, FULL_PREVIEW_LONG_EDGE);
    assert_eq!((out.width(), out.height()), (4096, 2048));
    // 不足上限不缩放
    let small = image::open(dir.path().join("big.png")).unwrap();
    let out2 = shrink_long_edge(small, 9000);
    assert_eq!((out2.width(), out2.height()), (8000, 4000));
  }

  /// Eagle 缩略图参数实验（#[ignore]）：cargo test -- --ignored webp_param_probe --nocapture
  /// 样本 = assets/image/寒冰射手_thumbnail.png（Eagle 出品，实为 webp：VP8X+ICC+有损 alpha）
  #[test]
  #[ignore]
  fn webp_param_probe() {
    let data = std::fs::read("../assets/image/寒冰射手_thumbnail.png").unwrap();
    let img = image::load_from_memory(&data).unwrap();
    let rgba = img.to_rgba8();
    println!("解码 {}x{}，Eagle 样本 13894B（ICC 456 + ALPH 5096 + VP8 8288）", rgba.width(), rgba.height());
    for (q, aq, m) in [(80.0, 100, 4), (80.0, 80, 4), (75.0, 80, 4), (70.0, 80, 4), (60.0, 80, 4), (80.0, 80, 6)] {
      let mut config = webp::WebPConfig::new().unwrap();
      config.quality = q;
      config.alpha_quality = aq;
      config.method = m;
      let mem = webp::Encoder::from_rgba(rgba.as_raw(), rgba.width(), rgba.height()).encode_advanced(&config).unwrap();
      println!("q{q} alpha{aq} m{m}: {}B", mem.len());
    }
  }

  #[cfg(windows)]
  #[test]
  fn ensure_format_icon_caches_once_per_ext() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let fla = root.join("工程.fla");
    std::fs::write(&fla, "fake fla bytes").unwrap();

    assert!(ensure_format_icon(root, &fla, "fla"));
    let icon = root.join("cache/thumbs/fla.webp");
    assert!(icon.is_file());
    // 二次调用幂等：文件不重复生成
    let mtime = fs::metadata(&icon).unwrap().modified().unwrap();
    assert!(ensure_format_icon(root, &fla, "fla"));
    assert_eq!(fs::metadata(&icon).unwrap().modified().unwrap(), mtime);
    // 无扩展名归 "file" 键
    let noext = root.join("无扩展名");
    std::fs::write(&noext, "bytes").unwrap();
    assert!(ensure_format_icon(root, &noext, ""));
    assert!(root.join("cache/thumbs/file.webp").is_file());
  }
}

