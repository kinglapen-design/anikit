//! 命令 API：逐帧笔记（5.2）。命令层只做资源定位与 key 校验，读写走 core::notes（单一写点）。
//! get_notes / set_note 统一返回全量列表（按 key 排序），前端直接替换；set_note 空文本 = 删除该条。
//! set_note 同步维护 metadata.noteCount 冗余（先落盘 metadata.json 再同步内存索引，规范第 7 条）。
//! get_note_thumb = 详情面板悬停帧画面（5.2 展示层）：序列帧直给帧原图路径，**动图 `f:{索引}`
//! = origin 运行时解码抽帧**（2026-08-20 解包废除）存 cache/note-thumbs 幂等；
//! 视频按 key 时间点 ffmpeg 截帧存 cache/note-thumbs（会话级缓存，幂等）。
//! get_notes 对动图资产附带**旧 key 一次性迁移**（解包时代帧名 key `{标题}-00042.webp` → `f:41`）。

use std::fs;

use serde::Serialize;
use tauri::State;

use crate::adapters::ffmpeg;
use crate::commands::library::AppState;
use crate::commands::preview::locate_asset;
use crate::core::json_io::write_json;
use crate::core::library::{Library, CACHE_DIR};
use crate::core::notes::{self, NoteEntry};
use crate::services::{media, thumbnail};

/// 笔记定位 key 长度上限（字符）
const MAX_KEY_CHARS: usize = 64;

/// 笔记帧画面缓存子目录（cache/note-thumbs/{assetId}_{安全化key}.webp；会话级，打开库清空）
const NOTE_THUMBS_DIR: &str = "note-thumbs";

/// 读某资源全部逐帧笔记（无 notes.json = 空列表）。
/// 动图资产附带旧 key 一次性迁移：解包时代帧名 key（`{标题}-00042.webp`，序号 1 起）→
/// `f:{索引0起}`；有迁移即落盘（replace_all 保留原文本与时间）。
/// async：涉及磁盘 IO，一律走线程池（规范第 2 条）
#[tauri::command]
pub async fn get_notes(state: State<'_, AppState>, asset_id: String) -> Result<Vec<NoteEntry>, String> {
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  let (info_dir, origin, meta) = locate_asset(lib, &asset_id)?;
  let list = notes::load(&info_dir).map_err(|e| e.to_string())?;
  if crate::services::anim::is_animated_file(&meta, &origin) && list.iter().any(|n| legacy_anim_frame_index(&n.key).is_some()) {
    let migrated = migrate_anim_keys(list);
    return notes::replace_all(&info_dir, migrated).map_err(|e| e.to_string());
  }
  Ok(list)
}

/// 旧动图帧名 key 解析（`{标题}-{序号5位}.webp` → 索引 0 起）：标题含连字符不误判（只看末段 5 位数字）
fn legacy_anim_frame_index(key: &str) -> Option<u32> {
  let stem = key.strip_suffix(".webp")?;
  let dash = stem.rfind('-')?;
  let num = stem.get(dash + 1..)?;
  if num.len() != 5 || !num.bytes().all(|b| b.is_ascii_digit()) {
    return None;
  }
  let n: u32 = num.parse().ok()?;
  (n >= 1).then(|| n - 1)
}

/// 旧 key 全量迁移：帧名 → `f:{索引}`；目标 key 冲突时保留 updatedAt 较新的一条
fn migrate_anim_keys(list: Vec<NoteEntry>) -> Vec<NoteEntry> {
  let mut out: Vec<NoteEntry> = Vec::with_capacity(list.len());
  for mut n in list {
    if let Some(idx) = legacy_anim_frame_index(&n.key) {
      n.key = format!("f:{idx}");
    }
    match out.iter_mut().find(|x| x.key == n.key) {
      // 目标已存在：保留时间较新者（RFC3339 同格式可字典序比较）
      Some(x) => {
        if n.updated_at > x.updated_at {
          *x = n;
        }
      }
      None => out.push(n),
    }
  }
  out
}

/// 写一条逐帧笔记：text 去空白后为空 = 删除该条；否则新增 / 覆盖（同 key 刷新 updatedAt）。
/// key 校验：非空、≤64 字符。统一返回全量列表（按 key 排序）。
/// async：涉及磁盘 IO，一律走线程池（规范第 2 条）
#[tauri::command]
pub async fn set_note(
  state: State<'_, AppState>,
  asset_id: String,
  key: String,
  text: String,
) -> Result<Vec<NoteEntry>, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  apply_set(lib, &asset_id, &key, &text)
}

/// set_note 实现（命令层薄封装 State，逻辑在此便于单测）：
/// key 去空白校验（非空、≤64 字符）；text 去空白后为空 = 删除该条，否则新增 / 覆盖；
/// 写完同步 metadata.noteCount 冗余（先落盘 metadata.json 再同步内存索引；条数未变不重复落盘）
fn apply_set(lib: &mut Library, asset_id: &str, key: &str, text: &str) -> Result<Vec<NoteEntry>, String> {
  let key = key.trim();
  if key.is_empty() {
    return Err("笔记定位（key）不能为空".to_string());
  }
  if key.chars().count() > MAX_KEY_CHARS {
    return Err(format!("笔记定位（key）过长，最多 {MAX_KEY_CHARS} 字符"));
  }
  let (info_dir, _origin, mut meta) = locate_asset(lib, asset_id)?;
  let result = if text.trim().is_empty() {
    notes::remove(&info_dir, key)
  } else {
    notes::upsert(&info_dir, key, text.trim())
  };
  let result = result.map_err(|e| e.to_string())?;
  // noteCount 冗余：0 记 None 不写字段（None = 无笔记，比 Some(0) 干净；卡片角标据此显隐）
  let count = (!result.is_empty()).then_some(result.len() as u32);
  if meta.note_count != count {
    meta.note_count = count;
    write_json(&info_dir.join("metadata.json"), &meta).map_err(|e| e.to_string())?;
    lib.index.assets.insert(asset_id.to_string(), meta);
  }
  Ok(result)
}

/// 详情面板悬停帧画面（5.2 展示层）
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NoteThumb {
  /// 帧画面绝对路径（webview 直渲）；取不到为 null（前端 C4 占位）
  pub path: Option<String>,
}

/// 取笔记对应帧画面：序列帧 key = 帧名，直给 info/frames/{key}（不存在返回 null）；
/// 动图 key = `f:{索引}`，origin 解码抽帧 → cache/note-thumbs/{assetId}_f-{索引5位}.webp（幂等）；
/// 视频 key = `t:{秒}`，ffmpeg 截帧 → cache/note-thumbs/{assetId}_{安全化key}.webp（幂等，
/// 已存在直给；截帧失败返回 null 不报错——悬停预览是辅助展示，不打扰用户）。
/// async：视频 / 动图分支涉及解码 IO 重活，一律走线程池（规范第 2 条）
#[tauri::command]
pub async fn get_note_thumb(state: State<'_, AppState>, asset_id: String, key: String) -> Result<NoteThumb, String> {
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  note_thumb(lib, &asset_id, &key)
}

fn note_thumb(lib: &Library, asset_id: &str, key: &str) -> Result<NoteThumb, String> {
  // key 安全化：帧名 / f:{索引} / t:{秒} 三种形态都不含路径分隔符，拒路径穿越
  if key.is_empty() || key.contains(['/', '\\']) || key.contains("..") {
    return Err("笔记定位（key）不合法".to_string());
  }
  let (info_dir, origin, meta) = locate_asset(lib, asset_id)?;
  if let Some(secs) = key.strip_prefix("t:") {
    return video_thumb(lib, &origin, asset_id, key, secs);
  }
  // 动图 `f:{索引}`：origin 运行时解码抽帧 → cache/note-thumbs 幂等（解包废除后无磁盘帧图）
  if let Some(idx) = key.strip_prefix("f:") {
    if !crate::services::anim::is_animated_file(&meta, &origin) {
      return Ok(NoteThumb { path: None }); // f: key 挂在非动图上：静默 null（占位）
    }
    let idx: u32 = idx.parse().map_err(|_| "笔记定位（key）不合法".to_string())?;
    let dest = lib.root.join(CACHE_DIR).join(NOTE_THUMBS_DIR).join(format!("{asset_id}_f-{idx:05}.webp"));
    if dest.is_file() {
      return Ok(NoteThumb { path: Some(dest.to_string_lossy().into_owned()) });
    }
    let Some((img, _delay)) = crate::services::anim::decode_frame(&origin, idx) else {
      return Ok(NoteThumb { path: None });
    };
    let thumb = thumbnail::shrink_to_thumb(image::DynamicImage::ImageRgba8(img));
    if let Some(dir) = dest.parent() {
      if let Err(e) = fs::create_dir_all(dir) {
        log::warn!("笔记帧画面缓存目录创建失败：{e}");
        return Ok(NoteThumb { path: None });
      }
    }
    return match thumbnail::save_webp(&thumb, &dest) {
      Ok(_) => Ok(NoteThumb { path: Some(dest.to_string_lossy().into_owned()) }),
      Err(e) => {
        log::warn!("笔记帧画面缓存落盘失败（{}）：{e}", origin.display());
        Ok(NoteThumb { path: None })
      }
    };
  }
  // 序列帧：帧原图已在 info/frames/
  let frame = info_dir.join("frames").join(key);
  Ok(NoteThumb { path: frame.is_file().then(|| frame.to_string_lossy().into_owned()) })
}

/// 视频分支：按 key 时间点截帧缓存（{assetId}_{安全化key}.webp，安全化 = : 与 . 换 _）
fn video_thumb(lib: &Library, origin: &std::path::Path, asset_id: &str, key: &str, secs: &str) -> Result<NoteThumb, String> {
  if !media::is_video(origin) {
    return Ok(NoteThumb { path: None }); // t: key 挂在非视频上：静默 null（占位），不算错误
  }
  let secs: f64 = secs.parse().map_err(|_| "笔记定位（key）不合法".to_string())?;
  let safe = key.replace([':', '.'], "_");
  let dest = lib.root.join(CACHE_DIR).join(NOTE_THUMBS_DIR).join(format!("{asset_id}_{safe}.webp"));
  if dest.is_file() {
    return Ok(NoteThumb { path: Some(dest.to_string_lossy().into_owned()) });
  }
  let Some(png) = ffmpeg::video_frame_png(origin, secs) else {
    return Ok(NoteThumb { path: None });
  };
  let Ok(img) = image::load_from_memory(&png) else {
    return Ok(NoteThumb { path: None });
  };
  let thumb = thumbnail::shrink_to_thumb(img);
  if let Some(dir) = dest.parent() {
    if let Err(e) = fs::create_dir_all(dir) {
      log::warn!("笔记帧画面缓存目录创建失败：{e}");
      return Ok(NoteThumb { path: None });
    }
  }
  match thumbnail::save_webp(&thumb, &dest) {
    Ok(_) => Ok(NoteThumb { path: Some(dest.to_string_lossy().into_owned()) }),
    Err(e) => {
      log::warn!("笔记帧画面缓存落盘失败（{}）：{e}", origin.display());
      Ok(NoteThumb { path: None })
    }
  }
}

#[cfg(test)]
mod tests {
  use std::fs;

  use super::*;
  use crate::core::json_io::read_json;
  use crate::core::library::{self, Library};
  use crate::core::models::AssetMeta;
  use crate::services::import::{self, PendingFile, Source};

  struct Harness {
    dir: tempfile::TempDir,
    lib: Library,
  }

  fn make_lib() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    Harness { dir, lib }
  }

  fn add_file(h: &mut Harness, id: &str, name: &str) -> AssetMeta {
    let src = h.dir.path().join(name);
    fs::write(&src, b"fake").unwrap();
    let pf = PendingFile { path: src, chain: Vec::new() };
    let meta = import::build_asset_with_id(&h.lib.assets_dir, &pf, "sha256:t", id, Source::Copy, None).unwrap();
    h.lib.index.assets.insert(id.to_string(), meta.clone());
    meta
  }

  #[test]
  fn set_note_blank_text_deletes_and_validates_key() {
    let mut h = make_lib();
    add_file(&mut h, "AN-VID", "片段.mp4");

    // 写入两条
    assert_eq!(apply_set(&mut h.lib, "AN-VID", "frame-00001", "  预备  ").unwrap().len(), 1);
    let notes = apply_set(&mut h.lib, "AN-VID", "t:12.500", "起跳").unwrap();
    assert_eq!(notes.len(), 2);
    assert_eq!(notes[0].text, "预备", "text 应去首尾空白（按 key 排序 frame-00001 在前）");

    // 空白文本 = 删除该条（含全空白）
    let notes = apply_set(&mut h.lib, "AN-VID", "t:12.500", "   ").unwrap();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].key, "frame-00001");

    // key 校验：空 / 超长中文报错
    assert!(apply_set(&mut h.lib, "AN-VID", "   ", "x").unwrap_err().contains("不能为空"));
    let long_key = "k".repeat(65);
    assert!(apply_set(&mut h.lib, "AN-VID", &long_key, "x").unwrap_err().contains("过长"));

    // 资源不存在报错
    assert!(apply_set(&mut h.lib, "AN-不存在", "k", "x").is_err());
  }

  /// noteCount 冗余（5.2 展示层）：随 set_note 同步——先落盘 metadata.json 再同步内存索引；
  /// 0 记 None 不写字段；同条数重复写不重复落盘
  #[test]
  fn set_note_syncs_note_count_to_meta_and_index() {
    let mut h = make_lib();
    add_file(&mut h, "AN-VID", "片段.mp4");
    let info = h.lib.info_dir("AN-VID");

    apply_set(&mut h.lib, "AN-VID", "t:01.000", "a").unwrap();
    apply_set(&mut h.lib, "AN-VID", "t:02.000", "b").unwrap();
    let disk: AssetMeta = read_json(&info.join("metadata.json")).unwrap();
    assert_eq!(disk.note_count, Some(2), "盘上 metadata 应落 noteCount");
    assert_eq!(h.lib.index.assets["AN-VID"].note_count, Some(2), "内存索引应同步");

    // 删到 0 → None（字段不写回文件）
    apply_set(&mut h.lib, "AN-VID", "t:01.000", "").unwrap();
    apply_set(&mut h.lib, "AN-VID", "t:02.000", "").unwrap();
    let disk: AssetMeta = read_json(&info.join("metadata.json")).unwrap();
    assert_eq!(disk.note_count, None);
    assert!(!fs::read_to_string(info.join("metadata.json")).unwrap().contains("noteCount"), "None 不序列化字段");
    assert_eq!(h.lib.index.assets["AN-VID"].note_count, None);

    // 同条数覆盖已有 key：条数未变不重复落盘（mtime 不动）
    apply_set(&mut h.lib, "AN-VID", "t:03.000", "c").unwrap();
    let mtime = fs::metadata(info.join("metadata.json")).unwrap().modified().unwrap();
    apply_set(&mut h.lib, "AN-VID", "t:03.000", "c2").unwrap();
    assert_eq!(fs::metadata(info.join("metadata.json")).unwrap().modified().unwrap(), mtime);
  }

  /// get_note_thumb 路由分支（假数据不真起 ffmpeg）：
  /// 序列帧 / 动图直给 info/frames/{key}；视频幂等缓存直给；失败一律 null 不报错
  #[test]
  fn note_thumb_routes_frame_video_and_cache() {
    let mut h = make_lib();
    add_file(&mut h, "AN-SEQ", "走路.fla");
    let info = h.lib.info_dir("AN-SEQ");
    fs::create_dir_all(info.join("frames")).unwrap();
    fs::write(info.join("frames").join("走路_00001.webp"), b"fake").unwrap();

    // 序列帧 / 动图分支：帧存在直给路径，缺失返回 null
    let hit = note_thumb(&h.lib, "AN-SEQ", "走路_00001.webp").unwrap();
    assert_eq!(hit.path.as_deref(), Some(info.join("frames").join("走路_00001.webp").to_string_lossy().as_ref()));
    assert_eq!(note_thumb(&h.lib, "AN-SEQ", "走路_99999.webp").unwrap(), NoteThumb { path: None });

    // t: key 挂在非视频上：静默 null
    assert_eq!(note_thumb(&h.lib, "AN-SEQ", "t:01.000").unwrap(), NoteThumb { path: None });

    // 路径穿越 / 空 key 拒绝
    assert!(note_thumb(&h.lib, "AN-SEQ", "../x.png").is_err());
    assert!(note_thumb(&h.lib, "AN-SEQ", "a/b.png").is_err());
    assert!(note_thumb(&h.lib, "AN-SEQ", "").is_err());
    // 资源不存在报错
    assert!(note_thumb(&h.lib, "AN-不存在", "k").is_err());
  }

  /// 视频分支幂等：缓存已存在直给（不依赖 ffmpeg）；未缓存且截帧失败返回 null
  #[test]
  fn note_thumb_video_cache_idempotent() {
    let mut h = make_lib();
    add_file(&mut h, "AN-VID", "片段.mp4");

    // 假视频（fake bytes）：ffmpeg 截帧失败（sidecar 缺失同路）→ null 不报错
    assert_eq!(note_thumb(&h.lib, "AN-VID", "t:12.500").unwrap(), NoteThumb { path: None });

    // 缓存已存在（安全化 key = : 与 . 换 _）→ 幂等直给
    let cache = h.lib.root.join(CACHE_DIR).join(NOTE_THUMBS_DIR).join("AN-VID_t_12_500.webp");
    fs::create_dir_all(cache.parent().unwrap()).unwrap();
    fs::write(&cache, b"fake").unwrap();
    assert_eq!(note_thumb(&h.lib, "AN-VID", "t:12.500").unwrap().path.as_deref(), Some(cache.to_string_lossy().as_ref()));

    // 秒值非法 → 中文报错
    assert!(note_thumb(&h.lib, "AN-VID", "t:abc").is_err());
  }
}
