//! 命令 API：动图（GIF / WEBP 动图）帧清单（5.2 预览播放套）。
//! 立即返回按 frameCount 预生成的占位清单；未缓存帧收集进预览解析队列（PreviewTask::AnimFrames）
//! 后台流式解码——cache/anim 全尺寸播放帧 + cache/frames 帧条逐帧落盘，
//! 批次结果（含准确 delayMs）经 preview-anim-frames-ready 事件渐进推送。

use serde::Serialize;
use tauri::State;

use crate::commands::library::AppState;
use crate::commands::preview::locate_asset;
use crate::core::library::Library;
use crate::core::models::AssetMeta;
use crate::services::anim;
use crate::services::preview_queue::{self, PreviewTask};
use crate::services::thumbnail;

/// 动图帧清单（预览窗口动图播放条）
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimFrames {
  /// 动图无固定帧率（逐帧 delayMs 驱动播放），恒为 null（与 SequenceFrames.fps 对齐形状）
  pub fps: Option<f64>,
  /// 总时长（秒，metadata.duration；缺失为 0）
  pub duration_secs: f64,
  pub frame_count: u32,
  /// 封面帧名（frame-00001 形式；metadata 未记时为 null）
  pub thumb_frame: Option<String>,
  pub frames: Vec<AnimFrameEntry>,
}

/// 动图单帧条目
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimFrameEntry {
  /// 帧名（frame- + 序号 5 位补齐，1 起）
  pub name: String,
  /// 该帧显示时长（ms）：占位值为 0，准确值随 preview-anim-frames-ready 事件逐批推送
  pub delay_ms: u32,
  /// 全尺寸播放帧绝对路径（cache/anim/{asset_id}/{帧名}.webp）；
  /// null = 未解码（已进预览解析队列后台生成）
  pub full_path: Option<String>,
  /// 帧条小图绝对路径（cache/frames/{asset_id}/{帧名}.webp，短边 128，与序列帧同构）；
  /// null = 未缓存（生成失败维持 null，前端直渲 fullPath 兜底）
  pub strip_path: Option<String>,
}

/// 动图帧清单：按 frameCount 预生成占位（delayMs 未知先 0）**立即返回**——已缓存的帧直接填
/// fullPath/stripPath（会话内重复进入零解码），未缓存帧进预览解析队列后台流式解码，
/// 批次完成经 preview-anim-frames-ready 推送。非动图资源（非 gif/webp 或无 frameCount）报错。
/// async：Tauri v2 非 async 命令在主线程执行，IO 重活一律线程池（规范第 2 条）
#[tauri::command]
pub async fn list_anim_frames(state: State<'_, AppState>, asset_id: String) -> Result<AnimFrames, String> {
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  anim_frames(lib, &asset_id)
}

fn anim_frames(lib: &Library, asset_id: &str) -> Result<AnimFrames, String> {
  let (_info_dir, origin, meta) = locate_asset(lib, asset_id)?;
  let Some(frame_count) = anim_frame_count(&meta) else {
    return Err(format!("「{}」不是动图资源", meta.name));
  };
  let mut has_uncached = false;
  let frames = (0..frame_count)
    .map(|i| {
      let name = anim::anim_frame_name(i);
      let full_path = thumbnail::anim_frame_path(&lib.root, asset_id, &name);
      let strip_path = thumbnail::frame_strip_path(&lib.root, asset_id, &name);
      if full_path.is_none() || strip_path.is_none() {
        has_uncached = true;
      }
      AnimFrameEntry { name, delay_ms: 0, full_path, strip_path }
    })
    .collect();
  if has_uncached {
    preview_queue::global().submit(PreviewTask::AnimFrames {
      root: lib.root.clone(),
      asset_id: asset_id.to_string(),
      origin,
    });
  }
  Ok(AnimFrames {
    fps: None,
    duration_secs: meta.duration.unwrap_or(0.0),
    frame_count,
    thumb_frame: meta.thumb_frame,
    frames,
  })
}

/// 动图判定：gif/webp 且已 probe 出多帧（frame_count 有值，导入时 anim::probe 写入）
fn anim_frame_count(meta: &AssetMeta) -> Option<u32> {
  if matches!(meta.ext.to_lowercase().as_str(), "gif" | "webp") {
    meta.frame_count
  } else {
    None
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::path::{Path, PathBuf};

  use crate::core::json_io::write_json;
  use crate::core::library::{self, ASSTES_DIR, CACHE_DIR};
  use crate::services::import::{self, PendingFile, Source};
  use crate::services::preview_queue::TEST_LOCK;

  /// 轮询等待条件成立（10s 超时判定失败），不裸 sleep 长等
  fn wait_until(mut cond: impl FnMut() -> bool, what: &str) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !cond() {
      assert!(std::time::Instant::now() < deadline, "等待超时：{what}");
      std::thread::sleep(std::time::Duration::from_millis(5));
    }
  }

  struct Harness {
    dir: tempfile::TempDir,
    lib: Library,
  }

  fn make_lib() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("测试库");
    let lib = library::create(&root).unwrap();
    Harness { dir, lib }
  }

  /// 造测试 gif（红/绿/蓝各 200ms，64x48）
  fn make_gif(path: &Path) {
    let colors = [[220, 40, 40, 255], [40, 200, 40, 255], [40, 40, 220, 255]];
    let file = std::fs::File::create(path).unwrap();
    let mut enc = image::codecs::gif::GifEncoder::new(file);
    enc.set_repeat(image::codecs::gif::Repeat::Infinite).unwrap();
    let frames: Vec<image::Frame> = colors
      .iter()
      .map(|c| {
        image::Frame::from_parts(
          image::RgbaImage::from_pixel(64, 48, image::Rgba(*c)),
          0,
          0,
          image::Delay::from_numer_denom_ms(200, 1),
        )
      })
      .collect();
    enc.encode_frames(frames).unwrap();
  }

  /// 造一个动图资源（真 gif 入库 + frameCount/duration 写 metadata）并登记进内存索引
  fn add_anim(h: &mut Harness, id: &str) -> AssetMeta {
    let src = h.dir.path().join("动图.gif");
    make_gif(&src);
    let pf = PendingFile { path: src, chain: Vec::new() };
    let mut meta = import::build_asset_with_id(&h.lib.root, &pf, "sha256:t", id, Source::Copy, None).unwrap();
    meta.frame_count = Some(3);
    meta.duration = Some(0.6);
    let info = h.lib.root.join(ASSTES_DIR).join(format!("{id}.info"));
    write_json(&info.join("metadata.json"), &meta).unwrap();
    h.lib.index.assets.insert(id.to_string(), meta.clone());
    meta
  }

  fn add_png(h: &mut Harness, id: &str) {
    let src = h.dir.path().join("单张.png");
    image::RgbaImage::from_pixel(100, 80, image::Rgba([30, 30, 200, 255])).save(&src).unwrap();
    let pf = PendingFile { path: src, chain: Vec::new() };
    let meta = import::build_asset_with_id(&h.lib.root, &pf, "sha256:t", id, Source::Copy, None).unwrap();
    h.lib.index.assets.insert(id.to_string(), meta);
  }

  #[test]
  fn list_anim_frames_returns_immediately_and_enqueues_decode() {
    let _lock = TEST_LOCK.lock().unwrap();
    let mut h = make_lib();
    add_anim(&mut h, "AN-GIF");
    let list = anim_frames(&h.lib, "AN-GIF").unwrap();

    // 占位清单按 frameCount 预生成：帧名 frame-00001..00003，delayMs 未知先 0，路径未缓存先 null
    assert_eq!(list.frame_count, 3);
    assert_eq!(list.duration_secs, 0.6);
    assert_eq!(list.fps, None);
    let names: Vec<&str> = list.frames.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["frame-00001", "frame-00002", "frame-00003"]);
    for f in &list.frames {
      assert_eq!(f.delay_ms, 0);
      assert_eq!(f.full_path, None);
      assert_eq!(f.strip_path, None);
    }

    // 后台队列最终落盘：cache/anim 播放帧 + cache/frames 帧条（与序列帧同构）
    let cache = h.lib.root.join(CACHE_DIR);
    let fulls: Vec<PathBuf> = names.iter().map(|n| cache.join("anim").join("AN-GIF").join(format!("{n}.webp"))).collect();
    let strips: Vec<PathBuf> =
      names.iter().map(|n| cache.join("frames").join("AN-GIF").join(format!("{n}.webp"))).collect();
    wait_until(|| fulls.iter().chain(&strips).all(|p| p.is_file()), "动图帧后台解码落盘");
    for p in fulls.iter().chain(&strips) {
      assert_eq!(image::image_dimensions(p).unwrap(), (64, 48), "原尺寸不足上限不缩放");
    }

    // 二次调用（会话内重复进入）：全部命中缓存直给路径，不重复解码
    let again = anim_frames(&h.lib, "AN-GIF").unwrap();
    for (f, (full, strip)) in again.frames.iter().zip(fulls.iter().zip(&strips)) {
      assert_eq!(f.full_path.as_deref(), Some(full.to_string_lossy().as_ref()));
      assert_eq!(f.strip_path.as_deref(), Some(strip.to_string_lossy().as_ref()));
    }
    let mtime = std::fs::metadata(&fulls[0]).unwrap().modified().unwrap();
    anim_frames(&h.lib, "AN-GIF").unwrap();
    assert_eq!(std::fs::metadata(&fulls[0]).unwrap().modified().unwrap(), mtime, "幂等不重复落盘");
  }

  #[test]
  fn list_anim_frames_rejects_non_anim() {
    let _lock = TEST_LOCK.lock().unwrap();
    let mut h = make_lib();
    add_png(&mut h, "AN-PNG");
    let err = anim_frames(&h.lib, "AN-PNG").unwrap_err();
    assert!(err.contains("不是动图资源"), "{err}");
  }
}
