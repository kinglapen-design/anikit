//! 命令 API：逐帧笔记（5.2）。命令层只做资源定位与 key 校验，读写走 core::notes（单一写点）。
//! 两条命令统一返回全量列表（按 key 排序），前端直接替换；set_note 空文本 = 删除该条。

use tauri::State;

use crate::commands::library::AppState;
use crate::commands::preview::locate_asset;
use crate::core::notes::{self, NoteEntry};

/// 笔记定位 key 长度上限（字符）
const MAX_KEY_CHARS: usize = 64;

/// 读某资源全部逐帧笔记（无 notes.json = 空列表）
/// async：涉及磁盘 IO，一律走线程池（规范第 2 条）
#[tauri::command]
pub async fn get_notes(state: State<'_, AppState>, asset_id: String) -> Result<Vec<NoteEntry>, String> {
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  let (info_dir, _origin, _meta) = locate_asset(lib, &asset_id)?;
  notes::load(&info_dir).map_err(|e| e.to_string())
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
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  apply_set(lib, &asset_id, &key, &text)
}

/// set_note 实现（命令层薄封装 State，逻辑在此便于单测）：
/// key 去空白校验（非空、≤64 字符）；text 去空白后为空 = 删除该条，否则新增 / 覆盖
fn apply_set(lib: &crate::core::library::Library, asset_id: &str, key: &str, text: &str) -> Result<Vec<NoteEntry>, String> {
  let key = key.trim();
  if key.is_empty() {
    return Err("笔记定位（key）不能为空".to_string());
  }
  if key.chars().count() > MAX_KEY_CHARS {
    return Err(format!("笔记定位（key）过长，最多 {MAX_KEY_CHARS} 字符"));
  }
  let (info_dir, _origin, _meta) = locate_asset(lib, asset_id)?;
  let result = if text.trim().is_empty() {
    notes::remove(&info_dir, key)
  } else {
    notes::upsert(&info_dir, key, text.trim())
  };
  result.map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
  use std::fs;

  use super::*;
  use crate::core::library::{self, Library};
  use crate::services::import::{self, PendingFile, Source};

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

  fn add_file(h: &mut Harness, id: &str, name: &str) {
    let src = h.dir.path().join(name);
    fs::write(&src, b"fake").unwrap();
    let pf = PendingFile { path: src, chain: Vec::new() };
    let meta = import::build_asset_with_id(&h.lib.root, &pf, "sha256:t", id, Source::Copy, None).unwrap();
    h.lib.index.assets.insert(id.to_string(), meta);
  }

  #[test]
  fn set_note_blank_text_deletes_and_validates_key() {
    let mut h = make_lib();
    add_file(&mut h, "AN-VID", "片段.mp4");

    // 写入两条
    assert_eq!(apply_set(&h.lib, "AN-VID", "frame-00001", "  预备  ").unwrap().len(), 1);
    let notes = apply_set(&h.lib, "AN-VID", "t:12.500", "起跳").unwrap();
    assert_eq!(notes.len(), 2);
    assert_eq!(notes[0].text, "预备", "text 应去首尾空白（按 key 排序 frame-00001 在前）");

    // 空白文本 = 删除该条（含全空白）
    let notes = apply_set(&h.lib, "AN-VID", "t:12.500", "   ").unwrap();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].key, "frame-00001");

    // key 校验：空 / 超长中文报错
    assert!(apply_set(&h.lib, "AN-VID", "   ", "x").unwrap_err().contains("不能为空"));
    let long_key = "k".repeat(65);
    assert!(apply_set(&h.lib, "AN-VID", &long_key, "x").unwrap_err().contains("过长"));

    // 资源不存在报错
    assert!(apply_set(&h.lib, "AN-不存在", "k", "x").is_err());
  }
}
