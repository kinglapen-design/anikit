//! 动图小动画播放缓存（2.3 性能优化；2026-09-02 恢复）：info/{stem}_anim.webp——短边 320
//! animated webp（尺寸语义同缩略图 THUMB_SHORT_EDGE：短边 320 长边等比，不足不放大），
//! 前端卡片播放替代原尺寸动图（get_asset_file_path 的 animPreview 字段下发）。
//! **纯 Rust 转码**（2026-09-02 八轮，ffmpeg sidecar 转码链退役）：anim::decode_frames 完整画布帧
//! → thumbnail::shrink_to_edge 缩放 → anim::encode_webp_anim 全帧组包（blend=0 全画布矩形）。
//! ffmpeg libwebp 差分编码产物本身带残影（矩形裁剪/blend 组合错误，Pillow 规范合成与 Chromium
//! 播放同现——CEP 伴生 webp 大透明区动图必发）；自研编解自洽，任何解码端不叠影，且摘掉 sidecar 依赖。
//! 生成走专用队列（anim_preview_queue：导入挂钩 + get_asset_file_path 懒生成补提交都走它）。
//! 缓存可重建、不进备份（同 thumbnail 口径）；失败返回 None——前端回退原文件播放，不报错打扰。

use std::fs;
use std::path::{Path, PathBuf};

use crate::core::json_io::read_json;
use crate::core::models::AssetMeta;
use crate::core::paths;
use crate::services::{anim, thumbnail};

/// 缓存是否新鲜：存在 + mtime 不旧于 origin（origin 换过 → 重新生成）+ 全帧组包结构
/// （旧 ffmpeg 差分产物一律判陈旧重生成——无版本戳可判时的结构特征口径，见 is_full_frame_pack）
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
  d >= o && is_full_frame_pack(dest)
}

/// 全帧组包判定（2026-09-02 八轮）：逐 ANMF chunk 校验 x=0,y=0、矩形=全画布、flags bit1=1
/// （blend=0 源覆盖）。ffmpeg libwebp 差分产物（矩形裁剪 / blend=1）与残影同构，判 false 重生成；
/// 读失败 / 非 RIFF / 无 ANMF 同样 false（ensure 重生成落盘覆盖是幂等修复方向）。
fn is_full_frame_pack(path: &Path) -> bool {
  let Ok(data) = fs::read(path) else {
    return false;
  };
  if data.len() < 16 || &data[0..4] != b"RIFF" || &data[8..12] != b"WEBP" {
    return false;
  }
  let u24 = |d: &[u8], o: usize| u32::from_le_bytes([d[o], d[o + 1], d[o + 2], 0]);
  let mut canvas: Option<(u32, u32)> = None;
  let mut frames = 0u32;
  let mut i = 12;
  while i + 8 <= data.len() {
    let fourcc = &data[i..i + 4];
    let size = u32::from_le_bytes(data[i + 4..i + 8].try_into().unwrap()) as usize;
    let end = i + 8 + size + (size & 1); // chunk 偶对齐 pad
    if end > data.len() {
      return false;
    }
    let d = &data[i + 8..end];
    match fourcc {
      // VP8X：[flags:1][reserved:3][canvas w-1:3][h-1:3]
      b"VP8X" => {
        if size < 10 {
          return false;
        }
        canvas = Some((u24(d, 4) + 1, u24(d, 7) + 1));
      }
      // ANMF：[x:3][y:3][w-1:3][h-1:3][duration:3][flags:1]（x/y 存储值 = 实际坐标÷2，本组包恒 0）
      b"ANMF" => {
        let Some((cw, ch)) = canvas else {
          return false;
        };
        if size < 16 || u24(d, 0) != 0 || u24(d, 3) != 0 || u24(d, 6) + 1 != cw || u24(d, 9) + 1 != ch {
          return false;
        }
        if d[15] & 0b10 == 0 {
          return false; // flags bit1 = do not blend（源覆盖）
        }
        frames += 1;
      }
      _ => {}
    }
    i = end;
  }
  frames > 0
}

/// 缓存"只判新鲜不生成"版本（命令层快路径）：命中返回绝对路径，未命中 None（生成归后台队列）
pub fn fresh_anim_preview(assets_dir: &Path, asset_id: &str, origin_name: &str) -> Option<String> {
  let dest = paths::anim_preview_file(assets_dir, asset_id, origin_name);
  let origin = paths::origin_file(assets_dir, asset_id, origin_name);
  is_fresh(&dest, &origin).then(|| dest.to_string_lossy().into_owned())
}

/// 生成小动画缓存（幂等）：已新鲜直接返回；否则纯 Rust 转码（解码 → 缩放 → 全帧组包），
/// 同目录临时文件 + rename 落盘（rename 覆盖旧缓存，防中途失败留半文件）。
/// 非动图 / metadata 缺失 / 解码或编码失败返回 None。
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
  // 逐帧解码（完整画布帧）→ 短边 320 缩放后收集；组包要求帧尺寸一致（decode 输出恒为画布尺寸）
  let mut frames: Vec<anim::AnimFrame> = Vec::new();
  anim::decode_frames(&origin, |_, img, delay| {
    let scaled = thumbnail::shrink_to_edge(image::DynamicImage::ImageRgba8(img), thumbnail::THUMB_SHORT_EDGE);
    frames.push(anim::AnimFrame { rgba: scaled.to_rgba8(), delay_ms: delay });
    true
  })?;
  let bytes = anim::encode_webp_anim(&frames, 80.0)?;
  let tmp = dest.with_extension("tmp");
  if let Err(e) = fs::write(&tmp, &bytes) {
    log::warn!("小动画缓存写盘失败（{asset_id}）：{e}");
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
    let h = make_lib();
    // 400×340（短边 340 > 320 → 缩到 376×320；400×300 短边仅 300 属不放大档，覆盖不了缩放）
    let origin_name = add_anim(&h, "横图.gif", "AN-GIF", 400, 340);

    let p = ensure_anim_preview(&h.lib.assets_dir, "AN-GIF").expect("转码应成功");
    assert_eq!(p, paths::anim_preview_file(&h.lib.assets_dir, "AN-GIF", &origin_name));
    assert_eq!(p.file_name().unwrap().to_string_lossy(), "横图_anim.webp");
    assert!(!p.with_extension("tmp").exists(), "临时文件应已 rename");

    // image-webp 验证产物：3 帧、376×320（整数除法 floor）、逐帧 delay 保留（总时长 =600ms）
    let dec = image_webp::WebPDecoder::new(BufReader::new(fs::File::open(&p).unwrap())).unwrap();
    assert!(dec.is_animated());
    assert_eq!(dec.num_frames(), 3, "帧数应保留");
    assert_eq!(dec.dimensions(), (376, 320), "短边缩到 320、长边等比");
    let dur = dec.loop_duration();
    assert_eq!(dur, 600, "逐帧 delay 保留（200ms×3）：{dur}");

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

  /// 全帧组包判定：本组包产物 true；差分矩形 / blend=1 / 无 ANMF / 非 RIFF 一律 false
  /// （旧 ffmpeg 差分产物判陈旧 → ensure 重生成覆盖，存量残影缓存自动翻新）
  #[test]
  fn full_frame_pack_detection() {
    let h = make_lib();
    let frames: Vec<anim::AnimFrame> = [[200, 40, 40, 255], [40, 200, 40, 255]]
      .iter()
      .map(|c| anim::AnimFrame { rgba: image::RgbaImage::from_pixel(32, 24, image::Rgba(*c)), delay_ms: 100 })
      .collect();
    let good = h._dir.path().join("good.webp");
    fs::write(&good, anim::encode_webp_anim(&frames, 80.0).unwrap()).unwrap();
    assert!(is_full_frame_pack(&good), "本组包产物应判新鲜");

    // 差分篡改：把第一帧 ANMF 的矩形改为非全画布（x=2）→ 判陈旧
    let mut bad = fs::read(&good).unwrap();
    let pos = bad.windows(4).position(|w| w == b"ANMF").expect("应有 ANMF chunk");
    bad[pos + 8] = 1; // Frame X 存储值 1（实际坐标 2）
    let bad_path = h._dir.path().join("bad.webp");
    fs::write(&bad_path, &bad).unwrap();
    assert!(!is_full_frame_pack(&bad_path), "差分矩形应判陈旧");

    // blend=1 篡改：flags bit1 清零 → 判陈旧
    let mut bad2 = fs::read(&good).unwrap();
    bad2[pos + 8 + 15] = 0;
    let bad2_path = h._dir.path().join("bad2.webp");
    fs::write(&bad2_path, &bad2).unwrap();
    assert!(!is_full_frame_pack(&bad2_path), "blend=1 应判陈旧");

    // 非 RIFF / 缺失文件 → 判陈旧（ensure 重生成覆盖方向安全）
    let junk = h._dir.path().join("junk.webp");
    fs::write(&junk, b"not a webp").unwrap();
    assert!(!is_full_frame_pack(&junk));
    assert!(!is_full_frame_pack(&h._dir.path().join("不存在.webp")));
  }

  /// 差分陈旧缓存被 ensure 重生成覆盖（is_fresh 结构判定的修复闭环）
  #[test]
  fn stale_diff_pack_regenerates() {
    let h = make_lib();
    let origin_name = add_anim(&h, "动图.gif", "AN-DIFF", 64, 48);
    let dest = paths::anim_preview_file(&h.lib.assets_dir, "AN-DIFF", &origin_name);
    let p = ensure_anim_preview(&h.lib.assets_dir, "AN-DIFF").unwrap();
    assert!(is_full_frame_pack(&p));

    // 篡改为差分矩形（mtime 仍新）：is_fresh 判陈旧 → ensure 重生成回全帧组包
    let mut bad = fs::read(&dest).unwrap();
    let pos = bad.windows(4).position(|w| w == b"ANMF").unwrap();
    bad[pos + 8] = 1;
    fs::write(&dest, &bad).unwrap();
    assert!(!is_full_frame_pack(&dest));
    let p2 = ensure_anim_preview(&h.lib.assets_dir, "AN-DIFF").unwrap();
    assert!(is_full_frame_pack(&p2), "差分陈旧应被重生成翻新");
    let _ = origin_name;
  }
}
