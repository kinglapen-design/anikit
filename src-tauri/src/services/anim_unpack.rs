//! 动图解包（2026-07-26 用户定案：动图 = 导入时自动解包的序列帧——逐帧高清图**持久化**
//! 存 info/frames/，与序列帧同构，推翻 2026-07-25 cache 会话级定案；依据用户原话：
//! 逐帧播放需要高清图配合，所以持久化）。
//! 帧图命名与序列帧同格式 `{标题}-{序号5位补齐}.webp`（webp q80 长边 ≤4096，同既有管线）；
//! 逐帧 delay 清单持久化 `info/frames.json`（`{ "version": 1, "delays": [ms, ...] }`，
//! 单一写点 = 本模块，规范第 1 条；备份清单 INFO_DATA_FILES 已收录）。
//! 帧条小图仍维持 cache/frames 会话级按需生成（预览解析队列），不在解包时预生成。
//! 两阶段：unpack_frames 纯磁盘重活（不持库锁，worker 线程跑）→ apply_meta 短锁同步
//! metadata（thumbFrame / modifiedAt）与内存索引。失败清理半成品 frames/（防黑箱半态）。

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::core::json_io::{read_json, write_json};
use crate::core::library::{Library, ASSTES_DIR};
use crate::core::models::{AssetMeta, DATA_VERSION};
use crate::services::{anim, sequence, thumbnail};

/// info 内逐帧 delay 清单文件名（备份对象，INFO_DATA_FILES 已收录）
pub const FRAMES_FILE: &str = "frames.json";

/// 解包帧序号位宽（归一命名 `{标题}-{序号5位补齐}.webp`，与序列帧同构）
const FRAME_NUM_WIDTH: usize = 5;

/// 解码失败帧的 delay 占位（ms）：坏帧跳过不回调，对齐全靠按序——占位保持 delays 与帧文件一一对应
const FALLBACK_DELAY_MS: u32 = 100;

/// frames.json：逐帧显示时长清单（ms），与 info/frames/ 内按文件名排序的帧清单一一对应
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FramesFile {
  pub version: u32,
  pub delays: Vec<u32>,
}

/// 解包结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnpackOutcome {
  /// 本次新解包完成：首帧文件名（供 metadata.thumbFrame 兜底）+ 帧数
  Unpacked { first_frame: String, frames: u32 },
  /// 幂等跳过（已解包且帧数与 frameCount 一致）或非动图资产
  Skipped,
}

/// 读 frames.json 的逐帧 delay 清单（ms）；文件不存在 / 损坏返回 None（调用方按无 delay 处理）
pub fn read_delays(info_dir: &Path) -> Option<Vec<u32>> {
  read_json::<FramesFile>(&info_dir.join(FRAMES_FILE)).ok().map(|f| f.delays)
}

/// 动图解包（纯磁盘，不持库锁）：定位 info 与 origin → decode_frames 流式逐帧落盘
/// info/frames/ → 写 frames.json。已解包（帧数与 frameCount 一致且 frames.json 在）幂等跳过；
/// 解码失败清理半成品 frames/ 并返回中文错误（资产仍可用，维持原生播放兜底）。
pub fn unpack_frames(root: &Path, asset_id: &str) -> Result<UnpackOutcome, String> {
  let info_dir = root.join(ASSTES_DIR).join(format!("{asset_id}.info"));
  let meta: AssetMeta = read_json(&info_dir.join("metadata.json")).map_err(|e| e.to_string())?;
  if !anim::is_anim(&meta) {
    return Ok(UnpackOutcome::Skipped);
  }
  let frames_dir = info_dir.join("frames");
  if already_unpacked(&info_dir, &frames_dir, meta.frame_count.unwrap_or(0)) {
    return Ok(UnpackOutcome::Skipped);
  }
  decode_to_frames(&info_dir, &frames_dir, &meta)
}

/// 幂等判定：frames/ 内 webp 帧数与 frameCount 一致且 frames.json 存在
fn already_unpacked(info_dir: &Path, frames_dir: &Path, frame_count: u32) -> bool {
  if !info_dir.join(FRAMES_FILE).is_file() {
    return false;
  }
  let webp_count = fs::read_dir(frames_dir)
    .map(|rd| {
      rd.filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("webp"))
        .count()
    })
    .unwrap_or(0);
  webp_count == frame_count as usize
}

/// 流式解码逐帧落盘 + frames.json；失败清理半成品 frames/（防黑箱半态）
fn decode_to_frames(info_dir: &Path, frames_dir: &Path, meta: &AssetMeta) -> Result<UnpackOutcome, String> {
  let origin = info_dir.join(&meta.origin_name);
  fs::create_dir_all(frames_dir).map_err(|e| format!("帧目录创建失败：{e}"))?;
  let mut delays: Vec<u32> = Vec::new();
  let mut first_frame = String::new();
  let mut written = 0u32;
  let decoded = anim::decode_frames(&origin, |index, img, delay| {
    // 坏帧跳过不回调：占位保持 delays 与按序落盘的帧文件一一对应
    while delays.len() < index as usize {
      delays.push(FALLBACK_DELAY_MS);
    }
    delays.push(delay);
    let name = sequence::frame_name(&meta.name, index + 1, FRAME_NUM_WIDTH, "webp");
    let img = thumbnail::shrink_long_edge(image::DynamicImage::ImageRgba8(img), thumbnail::FULL_PREVIEW_LONG_EDGE);
    match thumbnail::save_webp(&img, &frames_dir.join(&name)) {
      Ok(()) => {
        if first_frame.is_empty() {
          first_frame = name;
        }
        written += 1;
      }
      Err(e) => log::warn!("动图解包帧落盘失败（{}/{}）：{e}", meta.id, name),
    }
    true
  });
  if decoded.is_none() || written == 0 {
    let _ = fs::remove_dir_all(frames_dir);
    return Err(format!("「{}」动图解码失败", meta.name));
  }
  write_json(&info_dir.join(FRAMES_FILE), &FramesFile { version: DATA_VERSION, delays }).map_err(|e| e.to_string())?;
  log::info!("动图解包完成：{}（{}，{written} 帧）", meta.id, meta.name);
  Ok(UnpackOutcome::Unpacked { first_frame, frames: written })
}

/// 解包完成后 metadata 同步（短锁阶段，unpack_queue worker 调）：thumbFrame 原本为空则记
/// 首帧文件名，刷新 modifiedAt，先落盘再同步内存索引（规范第 7 条）
pub fn apply_meta(lib: &mut Library, asset_id: &str, first_frame: &str) -> Result<(), String> {
  let info_dir = lib.root.join(ASSTES_DIR).join(format!("{asset_id}.info"));
  let mut meta: AssetMeta = read_json(&info_dir.join("metadata.json")).map_err(|e| e.to_string())?;
  if meta.thumb_frame.is_none() {
    meta.thumb_frame = Some(first_frame.to_string());
  }
  meta.modified_at = chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, false);
  write_json(&info_dir.join("metadata.json"), &meta).map_err(|e| e.to_string())?;
  lib.index.assets.insert(asset_id.to_string(), meta);
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::path::PathBuf;

  use crate::core::library;
  use crate::services::import::{self, PendingFile, Source};

  struct Harness {
    _dir: tempfile::TempDir,
    lib: Library,
  }

  fn make_lib() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("测试库");
    let lib = library::create(&root).unwrap();
    Harness { _dir: dir, lib }
  }

  /// 真实样本入库（build_asset_with_id 会跑 probe 写 frameCount/duration），返回 asset_id
  fn add_anim(h: &Harness, sample: &str, asset_id: &str) -> AssetMeta {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/test-assets").join(sample);
    let pf = PendingFile { path: src, chain: Vec::new() };
    let meta = import::build_asset_with_id(&h.lib.root, &pf, "sha256:t", asset_id, Source::Copy, None).unwrap();
    assert!(anim::is_anim(&meta), "{sample} 应判定为动图资产");
    meta
  }

  fn frames_dir(h: &Harness, asset_id: &str) -> PathBuf {
    h.lib.root.join(ASSTES_DIR).join(format!("{asset_id}.info")).join("frames")
  }

  #[test]
  fn unpack_persists_frames_and_delays() {
    for (sample, asset_id) in [("anim3.gif", "AN-GIF"), ("anim3.webp", "AN-WEBP")] {
      let h = make_lib();
      let meta = add_anim(&h, sample, asset_id);

      let outcome = unpack_frames(&h.lib.root, asset_id).unwrap();
      let UnpackOutcome::Unpacked { first_frame, frames } = outcome else { panic!("应新解包：{outcome:?}") };
      assert_eq!(frames, 3);
      assert_eq!(first_frame, format!("{}-00001.webp", meta.name));

      // 帧图归一命名 + 可解码 + 原尺寸（64x48 不足长边上限不缩放）
      let dir = frames_dir(&h, asset_id);
      for n in 1..=3 {
        let p = dir.join(format!("{}-{n:05}.webp", meta.name));
        assert_eq!(image::image_dimensions(&p).unwrap(), (64, 48), "{sample} 第 {n} 帧");
      }
      // frames.json：单一写点原子写，逐帧 delay 200ms
      let delays = read_delays(dir.parent().unwrap()).unwrap();
      assert_eq!(delays, [200, 200, 200], "{sample} delays");

      // 幂等跳过：帧数与 frameCount 一致不重复解包（mtime 不变）
      let mtime = fs::metadata(dir.join(format!("{}-00001.webp", meta.name))).unwrap().modified().unwrap();
      assert_eq!(unpack_frames(&h.lib.root, asset_id).unwrap(), UnpackOutcome::Skipped);
      assert_eq!(fs::metadata(dir.join(format!("{}-00001.webp", meta.name))).unwrap().modified().unwrap(), mtime);
    }
  }

  #[test]
  fn unpack_failure_cleans_partial_frames() {
    let h = make_lib();
    // 假 gif 字节（probe 判不出动图，手动补 frameCount 模拟异常数据）
    let src = h._dir.path().join("坏.gif");
    fs::write(&src, b"not a gif").unwrap();
    let pf = PendingFile { path: src, chain: Vec::new() };
    let mut meta = import::build_asset_with_id(&h.lib.root, &pf, "sha256:t", "AN-BAD", Source::Copy, None).unwrap();
    meta.frame_count = Some(3);
    let info = h.lib.root.join(ASSTES_DIR).join("AN-BAD.info");
    write_json(&info.join("metadata.json"), &meta).unwrap();

    let err = unpack_frames(&h.lib.root, "AN-BAD").unwrap_err();
    assert!(err.contains("动图解码失败"), "{err}");
    assert!(!info.join("frames").exists(), "失败应清理半成品 frames/");
    assert!(!info.join(FRAMES_FILE).exists(), "失败不应留 frames.json");
  }

  #[test]
  fn apply_meta_sets_thumb_frame_and_syncs_index() {
    let mut h = make_lib();
    let mut meta = add_anim(&h, "anim3.gif", "AN-GIF");
    assert_eq!(meta.thumb_frame, None, "导入时 thumbFrame 为空");
    // 与建档时间区分（同秒刷新无法断言"已刷新"），先落一个哨兵时间
    meta.modified_at = "2020-01-01T00:00:00+08:00".to_string();
    let info = h.lib.root.join(ASSTES_DIR).join("AN-GIF.info");
    write_json(&info.join("metadata.json"), &meta).unwrap();
    h.lib.index.assets.insert("AN-GIF".to_string(), meta);

    apply_meta(&mut h.lib, "AN-GIF", "动图-00001.webp").unwrap();
    let indexed = &h.lib.index.assets["AN-GIF"];
    assert_eq!(indexed.thumb_frame.as_deref(), Some("动图-00001.webp"));
    assert_ne!(indexed.modified_at, "2020-01-01T00:00:00+08:00", "modifiedAt 应刷新");
    let disk: AssetMeta = read_json(&h.lib.root.join(ASSTES_DIR).join("AN-GIF.info/metadata.json")).unwrap();
    assert_eq!(disk, *indexed, "先落盘再同步内存索引");

    // 已有 thumbFrame 不覆盖
    apply_meta(&mut h.lib, "AN-GIF", "动图-00002.webp").unwrap();
    assert_eq!(h.lib.index.assets["AN-GIF"].thumb_frame.as_deref(), Some("动图-00001.webp"));
  }
}
