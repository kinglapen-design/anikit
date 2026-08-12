//! 动图小动画播放缓存（2.3 性能优化）：info/{stem}_anim.webp——短边 320 animated webp
//! （尺寸语义同缩略图 THUMB_SHORT_EDGE：短边 320 长边等比，不足不放大），
//! 前端卡片播放替代原尺寸 GIF（get_asset_file_path 的 animPreview 字段下发）。
//! ffmpeg sidecar 转码（命令拼装归 adapters/ffmpeg）；生成与解包共用同一异步管线
//! （unpack_queue：导入挂钩 + get_asset_file_path 懒生成补提交都走它）。
//! 缓存可重建、不进备份（同 thumbnail 口径）；失败返回 None——前端回退原文件播放，
//! 不报错打扰（sidecar 缺失 / 转码失败均为能力降级，非用户错误）。

use std::fs;
use std::path::{Path, PathBuf};

use crate::adapters::ffmpeg;
use crate::core::json_io::read_json;
use crate::core::models::AssetMeta;
use crate::core::paths;
use crate::services::{anim, thumbnail};

/// 缓存是否新鲜：存在且 mtime 不旧于 origin（origin 换过 → 重新生成）
fn is_fresh(dest: &Path, origin: &Path) -> bool {
  if !dest.is_file() {
    return false;
  }
  let (Ok(d), Ok(o)) = (fs::metadata(dest), fs::metadata(origin)) else {
    return false;
  };
  let (Ok(d), Ok(o)) = (d.modified(), o.modified()) else {
    return false;
  };
  d >= o
}

/// 缓存"只判新鲜不生成"版本（命令层快路径）：命中返回绝对路径，未命中 None（生成归后台队列）
pub fn fresh_anim_preview(assets_dir: &Path, asset_id: &str, origin_name: &str) -> Option<String> {
  let dest = paths::anim_preview_file(assets_dir, asset_id, origin_name);
  let origin = paths::origin_file(assets_dir, asset_id, origin_name);
  is_fresh(&dest, &origin).then(|| dest.to_string_lossy().into_owned())
}

/// 生成小动画缓存（幂等）：已新鲜直接返回；否则 ffmpeg 转码同目录临时文件 + rename 落盘
/// （rename 覆盖旧缓存，防中途失败留半文件）。非动图 / metadata 缺失 / 转码失败返回 None。
pub fn ensure_anim_preview(assets_dir: &Path, asset_id: &str) -> Option<PathBuf> {
  let info_dir = paths::info_dir(assets_dir, asset_id);
  let meta: AssetMeta = read_json(&info_dir.join("metadata.json")).ok()?;
  if !anim::is_anim(&meta) {
    return None;
  }
  let dest = paths::anim_preview_file(assets_dir, asset_id, &meta.origin_name);
  let origin = info_dir.join(&meta.origin_name);
  if is_fresh(&dest, &origin) {
    return Some(dest);
  }
  let tmp = dest.with_extension("tmp");
  if !ffmpeg::transcode_anim_preview(&origin, &tmp, thumbnail::THUMB_SHORT_EDGE) {
    let _ = fs::remove_file(&tmp);
    return None;
  }
  if let Err(e) = fs::rename(&tmp, &dest) {
    log::warn!("小动画缓存落盘失败（{asset_id}）：{e}");
    let _ = fs::remove_file(&tmp);
    return None;
  }
  log::info!("小动画播放缓存已生成：{asset_id}");
  Some(dest)
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::BufReader;

  use crate::core::library;
  use crate::services::import::{self, PendingFile, Source};

  struct Harness {
    _dir: tempfile::TempDir,
    lib: crate::core::library::Library,
  }

  fn make_lib() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    Harness { _dir: dir, lib }
  }

  /// 造 3 帧测试 gif（红/绿/蓝各 200ms，尺寸 w×h）
  fn make_gif(path: &Path, w: u32, h: u32) {
    let colors = [[220, 40, 40, 255], [40, 200, 40, 255], [40, 40, 220, 255]];
    let file = fs::File::create(path).unwrap();
    let mut enc = image::codecs::gif::GifEncoder::new(file);
    enc.set_repeat(image::codecs::gif::Repeat::Infinite).unwrap();
    let frames: Vec<image::Frame> = colors
      .iter()
      .map(|c| {
        image::Frame::from_parts(
          image::RgbaImage::from_pixel(w, h, image::Rgba(*c)),
          0,
          0,
          image::Delay::from_numer_denom_ms(200, 1),
        )
      })
      .collect();
    enc.encode_frames(frames).unwrap();
  }

  /// gif 入库（build_asset_with_id 跑 probe 写 frameCount），返回 origin_name
  fn add_anim(hx: &Harness, name: &str, asset_id: &str, w: u32, h: u32) -> String {
    let src = hx._dir.path().join(name);
    make_gif(&src, w, h);
    let pf = PendingFile { path: src, chain: Vec::new() };
    let meta = import::build_asset_with_id(&hx.lib.assets_dir, &pf, "sha256:t", asset_id, Source::Copy, None).unwrap();
    assert!(anim::is_anim(&meta), "{name} 应判定为动图资产");
    meta.origin_name
  }

  #[test]
  fn ensure_generates_scaled_anim_webp() {
    // 转码依赖 ffmpeg sidecar：未下载二进制的机器上跳过（二进制不入 git）
    if !ffmpeg::ffmpeg_available() {
      eprintln!("ffmpeg sidecar 缺失，跳过小动画转码验证");
      return;
    }
    let h = make_lib();
    // 400×340（短边 340 > 320 → 缩到 376×320；400×300 短边仅 300 属不放大档，覆盖不了缩放）
    let origin_name = add_anim(&h, "横图.gif", "AN-GIF", 400, 340);

    let p = ensure_anim_preview(&h.lib.assets_dir, "AN-GIF").expect("转码应成功");
    assert_eq!(p, paths::anim_preview_file(&h.lib.assets_dir, "AN-GIF", &origin_name));
    assert_eq!(p.file_name().unwrap().to_string_lossy(), "横图_anim.webp");
    assert!(!p.with_extension("tmp").exists(), "临时文件应已 rename");

    // image-webp 验证产物：3 帧、376×320（w=-2 等比取偶）、逐帧 delay 保留（总时长 ≈600ms）
    let dec = image_webp::WebPDecoder::new(BufReader::new(fs::File::open(&p).unwrap())).unwrap();
    assert!(dec.is_animated());
    assert_eq!(dec.num_frames(), 3, "帧数应保留");
    assert_eq!(dec.dimensions(), (376, 320), "短边缩到 320、长边等比");
    let dur = dec.loop_duration();
    assert!((dur as i64 - 600).abs() <= 100, "总时长应≈600ms（fps_mode passthrough 保留逐帧 delay）：{dur}");

    // 幂等：已新鲜直接返回，不重复转码（mtime 不变）
    let mtime = fs::metadata(&p).unwrap().modified().unwrap();
    assert_eq!(ensure_anim_preview(&h.lib.assets_dir, "AN-GIF").as_deref(), Some(p.as_path()));
    assert_eq!(fs::metadata(&p).unwrap().modified().unwrap(), mtime);
    // 命令层快路径同路径命中
    assert_eq!(
      fresh_anim_preview(&h.lib.assets_dir, "AN-GIF", &origin_name).as_deref(),
      Some(p.to_string_lossy().as_ref())
    );
  }

  #[test]
  fn ensure_does_not_upscale_small_anim() {
    if !ffmpeg::ffmpeg_available() {
      eprintln!("ffmpeg sidecar 缺失，跳过小动画转码验证");
      return;
    }
    let h = make_lib();
    add_anim(&h, "小图.gif", "AN-SMALL", 64, 48); // 短边 48 < 320 不放大
    let p = ensure_anim_preview(&h.lib.assets_dir, "AN-SMALL").expect("转码应成功");
    let dec = image_webp::WebPDecoder::new(BufReader::new(fs::File::open(&p).unwrap())).unwrap();
    assert_eq!(dec.dimensions(), (64, 48), "短边不足 320 保持原尺寸");
  }

  #[test]
  fn ensure_returns_none_for_non_anim() {
    let h = make_lib();
    // 静态 png 入库
    let src = h._dir.path().join("静态.png");
    image::RgbaImage::from_pixel(64, 48, image::Rgba([1, 2, 3, 255])).save(&src).unwrap();
    let pf = PendingFile { path: src, chain: Vec::new() };
    import::build_asset_with_id(&h.lib.assets_dir, &pf, "sha256:t", "AN-PNG", Source::Copy, None).unwrap();
    assert_eq!(ensure_anim_preview(&h.lib.assets_dir, "AN-PNG"), None, "非动图不生成");
    // 不存在的资产（无 metadata）也 None，不 panic
    assert_eq!(ensure_anim_preview(&h.lib.assets_dir, "AN-不存在"), None);
    assert_eq!(fresh_anim_preview(&h.lib.assets_dir, "AN-PNG", "静态.png"), None, "未生成时快路径 None");
  }
}
