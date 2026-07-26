//! 逐帧笔记 notes.json（5.2）：info 内单资源数据文件，单一写点 = 本模块（规范第 1 条）。
//! 结构 `{ "version": 1, "notes": [{ key, text, updatedAt }] }`（camelCase）；
//! key 语义约定：序列帧 / 动图 = 帧名（归一名，如 标题-001.png / 标题-00001.webp），视频 = `t:{秒:.3}`（如 t:12.500）。
//! 读写复用 json_io 原子写 + version 门禁；文件不存在 = 空列表（不算错误）。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::core::json_io::{read_json, write_json, DataError};
use crate::core::models::DATA_VERSION;

pub const NOTES_FILE: &str = "notes.json";

/// 单条笔记长度上限（字符），超出截断
const MAX_TEXT_CHARS: usize = 2000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NotesFile {
  pub version: u32,
  #[serde(default)]
  pub notes: Vec<NoteEntry>,
}

/// 单条逐帧笔记（updatedAt = RFC3339）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteEntry {
  pub key: String,
  pub text: String,
  pub updated_at: String,
}

fn path(info_dir: &Path) -> PathBuf {
  info_dir.join(NOTES_FILE)
}

/// 读全部笔记（按 key 排序）；文件不存在 = 空列表
pub fn load(info_dir: &Path) -> Result<Vec<NoteEntry>, DataError> {
  let p = path(info_dir);
  if !p.is_file() {
    return Ok(Vec::new());
  }
  let mut file: NotesFile = read_json(&p)?;
  file.notes.sort_by(|a, b| a.key.cmp(&b.key));
  Ok(file.notes)
}

/// 新增 / 覆盖一条：同 key 覆盖 text 并刷新 updatedAt；text 超 2000 字符截断。返回全量列表（按 key 排序）
pub fn upsert(info_dir: &Path, key: &str, text: &str) -> Result<Vec<NoteEntry>, DataError> {
  let mut notes = load(info_dir)?;
  let text: String = text.chars().take(MAX_TEXT_CHARS).collect();
  let now = chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, false);
  match notes.iter_mut().find(|n| n.key == key) {
    Some(n) => {
      n.text = text;
      n.updated_at = now;
    }
    None => notes.push(NoteEntry { key: key.to_string(), text, updated_at: now }),
  }
  persist(info_dir, notes)
}

/// 删除一条（不存在静默成功；文件不存在 = 空列表不落盘）。返回全量列表（按 key 排序）
pub fn remove(info_dir: &Path, key: &str) -> Result<Vec<NoteEntry>, DataError> {
  if !path(info_dir).is_file() {
    return Ok(Vec::new());
  }
  let mut notes = load(info_dir)?;
  notes.retain(|n| n.key != key);
  persist(info_dir, notes)
}

/// 先落盘再返回（规范第 7 条：写操作先落盘）
fn persist(info_dir: &Path, mut notes: Vec<NoteEntry>) -> Result<Vec<NoteEntry>, DataError> {
  notes.sort_by(|a, b| a.key.cmp(&b.key));
  write_json(&path(info_dir), &NotesFile { version: DATA_VERSION, notes: notes.clone() })?;
  Ok(notes)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn load_missing_file_returns_empty() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(load(dir.path()).unwrap(), Vec::new());
    // 不存在时 remove 也静默返回空且不创建文件
    assert_eq!(remove(dir.path(), "frame-00001").unwrap(), Vec::new());
    assert!(!path(dir.path()).exists());
  }

  #[test]
  fn upsert_adds_overwrites_and_refreshes_time() {
    let dir = tempfile::tempdir().unwrap();
    let notes = upsert(dir.path(), "t:12.500", "起跳前预备").unwrap();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].key, "t:12.500");
    assert!(!notes[0].updated_at.is_empty());

    // 再插一条，返回按 key 排序
    let notes = upsert(dir.path(), "frame-00001", "第一帧").unwrap();
    let keys: Vec<&str> = notes.iter().map(|n| n.key.as_str()).collect();
    assert_eq!(keys, ["frame-00001", "t:12.500"]);

    // 同 key 覆盖 text 并刷新 updatedAt（盘上调旧时间验证刷新）
    let mut file: NotesFile = read_json(&path(dir.path())).unwrap();
    file.notes.iter_mut().find(|n| n.key == "t:12.500").unwrap().updated_at = "2000-01-01T00:00:00+08:00".into();
    write_json(&path(dir.path()), &file).unwrap();
    let notes = upsert(dir.path(), "t:12.500", "改成落地缓冲").unwrap();
    let entry = notes.iter().find(|n| n.key == "t:12.500").unwrap();
    assert_eq!(entry.text, "改成落地缓冲");
    assert_ne!(entry.updated_at, "2000-01-01T00:00:00+08:00");
    assert_eq!(notes.len(), 2, "覆盖不应新增条目");
  }

  #[test]
  fn remove_deletes_existing_key() {
    let dir = tempfile::tempdir().unwrap();
    upsert(dir.path(), "frame-00001", "a").unwrap();
    upsert(dir.path(), "frame-00002", "b").unwrap();
    let notes = remove(dir.path(), "frame-00001").unwrap();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].key, "frame-00002");
    // 盘上同步（先落盘）
    assert_eq!(load(dir.path()).unwrap().len(), 1);
    // 删不存在的 key 静默成功
    assert_eq!(remove(dir.path(), "frame-99999").unwrap().len(), 1);
  }

  #[test]
  fn load_rejects_version_too_new() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(path(dir.path()), r#"{ "version": 999, "notes": [] }"#).unwrap();
    assert!(matches!(load(dir.path()), Err(DataError::VersionTooNew(999))));
  }

  #[test]
  fn upsert_truncates_overlong_text() {
    let dir = tempfile::tempdir().unwrap();
    let long: String = "字".repeat(2500);
    let notes = upsert(dir.path(), "k", &long).unwrap();
    assert_eq!(notes[0].text.chars().count(), MAX_TEXT_CHARS);
  }
}
