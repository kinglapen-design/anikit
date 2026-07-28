//! 会话级撤销 / 重做双栈（6.0 地基；上限 50 步，库切换清空，不持久化——与 Eagle 一致）。
//!
//! UndoEntry = 操作标签 + 受影响数据文件操作前 / 后快照 + 磁盘逆操作清单（成对）。
//! 快照注入点 = 各数据文件单一写点：写操作命令按 `begin(标签)` → 写点前 `record_file(路径)`
//! → `commit()` 三步接入（6.1 起由 commands::operations 各命令接入）；写盘是原子写，回滚同样
//! 原子写（version 随快照原文恢复，门禁天然合法）。undo / redo = 磁盘逆操作（逆序）+ 快照写回，
//! 随后由命令层重建内存索引并通知前端刷新。新写操作 commit 即清空 redo 栈。
//! 回收站语义天然配合：删除仅数据层移动，undo 移回即可；大批量写操作前自动备份机制不变
//! （备份防灾、撤销管操作，PRD 06-6.3）。

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::json_io::{write_text_atomic, DataError};

/// 双栈上限（步），超出丢最旧
pub const MAX_STEPS: usize = 50;

/// 数据文件快照：content = None 表示快照时文件不存在（undo = 删除该文件，redo = 按 after 重建）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSnapshot {
  pub path: PathBuf,
  pub content: Option<String>,
}

/// 磁盘逆操作（文件系统副作用；撤销 / 重做成对记录，6.1 operations 命令接入）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiskOp {
  /// 删除该文件或目录（目录递归删除，如撤销 duplicate 新建的副本 info）
  Delete(PathBuf),
  /// 把 from 重命名为 to（如重命名 originName 改回；重做 = from / to 对调的同名变体）
  Rename { from: PathBuf, to: PathBuf },
  /// 复制 from 到 to（如重做 duplicate；目录递归复制，已存在文件覆盖）
  Copy { from: PathBuf, to: PathBuf },
}

/// 一条可撤销操作
#[derive(Debug, Clone)]
pub struct UndoEntry {
  /// 操作标签（通知条 / 菜单文案用，如 "移动 1 个项目至“08素材”"）
  pub label: String,
  /// 受影响数据文件操作前快照（undo 写回）
  before: Vec<FileSnapshot>,
  /// 受影响数据文件操作后快照（commit 时抓取，redo 写回）
  after: Vec<FileSnapshot>,
  /// 磁盘逆操作（undo 逆序执行）
  disk_undo: Vec<DiskOp>,
  /// 磁盘正向操作（redo 逆序执行）
  disk_redo: Vec<DiskOp>,
}

/// 栈状态（命令 get_undo_state 输出，菜单置灰用）
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoState {
  pub can_undo: bool,
  pub can_redo: bool,
  pub undo_label: Option<String>,
  pub redo_label: Option<String>,
}

/// 进行中的事务（begin ~ commit 之间）
struct Pending {
  label: String,
  before: Vec<FileSnapshot>,
  /// 同路径去重：只记第一次的旧内容
  recorded: HashSet<PathBuf>,
  disk_undo: Vec<DiskOp>,
  disk_redo: Vec<DiskOp>,
}

/// 会话级撤销 / 重做管理器（AppState 持有 Mutex；库切换 clear）
#[derive(Default)]
pub struct UndoManager {
  undo: Vec<UndoEntry>,
  redo: Vec<UndoEntry>,
  pending: Option<Pending>,
}

impl UndoManager {
  /// 开始一个事务；上一个事务未 commit 视为遗漏，丢弃并记日志（不吞错误但流程继续）
  pub fn begin(&mut self, label: impl Into<String>) {
    if let Some(old) = self.pending.take() {
      log::warn!("撤销事务“{}”未提交即被新事务覆盖，已丢弃", old.label);
    }
    self.pending = Some(Pending {
      label: label.into(),
      before: Vec::new(),
      recorded: HashSet::new(),
      disk_undo: Vec::new(),
      disk_redo: Vec::new(),
    });
  }

  /// 写点前记录数据文件旧内容（事务内同路径只记首次）；无事务 = 该操作不要求可撤销，静默跳过
  pub fn record_file(&mut self, path: &Path) -> Result<(), DataError> {
    let Some(p) = self.pending.as_mut() else { return Ok(()) };
    if !p.recorded.insert(path.to_path_buf()) {
      return Ok(());
    }
    let content = match fs::read_to_string(path) {
      Ok(text) => Some(text),
      Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
      Err(e) => return Err(DataError::Io(e)),
    };
    p.before.push(FileSnapshot { path: path.to_path_buf(), content });
    Ok(())
  }

  /// 记录磁盘逆操作（成对：撤销 / 重做）；无事务静默跳过
  pub fn record_disk_op(&mut self, undo_op: DiskOp, redo_op: DiskOp) {
    if let Some(p) = self.pending.as_mut() {
      p.disk_undo.push(undo_op);
      p.disk_redo.push(redo_op);
    }
  }

  /// 提交事务：抓取操作后快照入 undo 栈、清空 redo 栈；无事务静默跳过
  pub fn commit(&mut self) -> Result<(), DataError> {
    let Some(p) = self.pending.take() else { return Ok(()) };
    let mut after = Vec::with_capacity(p.before.len());
    for snap in &p.before {
      after.push(FileSnapshot { path: snap.path.clone(), content: read_text(&snap.path)? });
    }
    self.undo.push(UndoEntry {
      label: p.label,
      before: p.before,
      after,
      disk_undo: p.disk_undo,
      disk_redo: p.disk_redo,
    });
    if self.undo.len() > MAX_STEPS {
      self.undo.remove(0);
    }
    self.redo.clear();
    Ok(())
  }

  /// 丢弃进行中的事务（操作中途失败回滚业务后调用）
  pub fn cancel(&mut self) {
    self.pending = None;
  }

  /// 撤销一步：回滚成功才出栈（失败栈不动，错误上抛中文人话）；空栈返回 None
  pub fn undo(&mut self) -> Result<Option<String>, DataError> {
    let Some(entry) = self.undo.last().cloned() else { return Ok(None) };
    apply_disk(&entry.disk_undo)?;
    apply_snapshots(&entry.before)?;
    let entry = self.undo.pop().expect("已确认非空");
    let label = entry.label.clone();
    self.redo.push(entry);
    Ok(Some(label))
  }

  /// 重做一步：规则同 undo；空栈返回 None
  pub fn redo(&mut self) -> Result<Option<String>, DataError> {
    let Some(entry) = self.redo.last().cloned() else { return Ok(None) };
    apply_disk(&entry.disk_redo)?;
    apply_snapshots(&entry.after)?;
    let entry = self.redo.pop().expect("已确认非空");
    let label = entry.label.clone();
    self.undo.push(entry);
    Ok(Some(label))
  }

  /// 栈状态（菜单置灰用）
  pub fn state(&self) -> UndoState {
    UndoState {
      can_undo: !self.undo.is_empty(),
      can_redo: !self.redo.is_empty(),
      undo_label: self.undo.last().map(|e| e.label.clone()),
      redo_label: self.redo.last().map(|e| e.label.clone()),
    }
  }

  /// 清空双栈与事务（库切换）
  pub fn clear(&mut self) {
    self.undo.clear();
    self.redo.clear();
    self.pending = None;
  }
}

/// 读文件文本；不存在 = None（快照语义）
fn read_text(path: &Path) -> Result<Option<String>, DataError> {
  match fs::read_to_string(path) {
    Ok(text) => Ok(Some(text)),
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
    Err(e) => Err(DataError::Io(e)),
  }
}

/// 快照写回：有内容原子写回原文，无内容删除文件（已不存在跳过）
fn apply_snapshots(snapshots: &[FileSnapshot]) -> Result<(), DataError> {
  for snap in snapshots {
    match &snap.content {
      Some(text) => write_text_atomic(&snap.path, text)?,
      None => {
        if snap.path.exists() {
          fs::remove_file(&snap.path)?;
        }
      }
    }
  }
  Ok(())
}

/// 磁盘操作执行（逆序；Delete / Rename 目标缺失跳过并记日志——外部已删不算失败）
fn apply_disk(ops: &[DiskOp]) -> Result<(), DataError> {
  for op in ops.iter().rev() {
    match op {
      DiskOp::Delete(path) => {
        if path.is_file() {
          fs::remove_file(path)?;
        } else if path.is_dir() {
          fs::remove_dir_all(path)?;
        }
      }
      DiskOp::Rename { from, to } => {
        if from.exists() {
          fs::rename(from, to)?;
        } else {
          log::warn!("撤销磁盘逆操作跳过：源不存在（{}）", from.display());
        }
      }
      DiskOp::Copy { from, to } => {
        if from.is_dir() {
          copy_dir(from, to)?;
        } else {
          fs::copy(from, to)?;
        }
      }
    }
  }
  Ok(())
}

/// 递归复制目录（DiskOp::Copy 目录形态，如重做 duplicate 的 info 目录）
fn copy_dir(from: &Path, to: &Path) -> Result<(), DataError> {
  fs::create_dir_all(to)?;
  for entry in fs::read_dir(from)? {
    let entry = entry?;
    let path = entry.path();
    let dest = to.join(entry.file_name());
    if path.is_dir() {
      copy_dir(&path, &dest)?;
    } else {
      fs::copy(&path, &dest)?;
    }
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  fn manager() -> UndoManager {
    UndoManager::default()
  }

  /// 修改已有文件：undo 恢复旧内容，redo 恢复新内容
  #[test]
  fn undo_redo_restores_file_content() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("folders.json");
    fs::write(&path, "旧内容").unwrap();

    let mut m = manager();
    m.begin("移动项目");
    m.record_file(&path).unwrap();
    fs::write(&path, "新内容").unwrap();
    m.commit().unwrap();

    let st = m.state();
    assert!(st.can_undo && !st.can_redo);
    assert_eq!(st.undo_label.as_deref(), Some("移动项目"));

    assert_eq!(m.undo().unwrap().as_deref(), Some("移动项目"));
    assert_eq!(fs::read_to_string(&path).unwrap(), "旧内容");
    assert!(m.state().can_redo);

    assert_eq!(m.redo().unwrap().as_deref(), Some("移动项目"));
    assert_eq!(fs::read_to_string(&path).unwrap(), "新内容");
    assert!(m.state().can_undo);
  }

  /// 操作前不存在的文件：undo 删除，redo 按 after 快照重建
  #[test]
  fn undo_deletes_created_file_and_redo_rebuilds() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("notes.json");

    let mut m = manager();
    m.begin("新建笔记");
    m.record_file(&path).unwrap();
    fs::write(&path, "笔记内容").unwrap();
    m.commit().unwrap();

    m.undo().unwrap();
    assert!(!path.exists(), "undo 应删除操作前不存在的文件");
    m.redo().unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "笔记内容");
  }

  /// 同路径重复 record 只记第一次的旧内容（多次写同一文件回滚到操作前）
  #[test]
  fn record_file_dedupes_same_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.json");
    fs::write(&path, "v1").unwrap();

    let mut m = manager();
    m.begin("连续写");
    m.record_file(&path).unwrap();
    fs::write(&path, "v2").unwrap();
    m.record_file(&path).unwrap(); // 第二次不覆盖 v1 快照
    fs::write(&path, "v3").unwrap();
    m.commit().unwrap();

    m.undo().unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "v1");
  }

  /// 上限 50 步丢最旧；commit 清空 redo 栈
  #[test]
  fn stack_cap_and_commit_clears_redo() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.json");
    fs::write(&path, "0").unwrap();

    let mut m = manager();
    for i in 1..=(MAX_STEPS + 5) {
      m.begin(format!("第{i}步"));
      m.record_file(&path).unwrap();
      fs::write(&path, i.to_string()).unwrap();
      m.commit().unwrap();
    }
    // 连续撤销 3 步后新写操作清空 redo 栈
    m.undo().unwrap();
    m.undo().unwrap();
    m.undo().unwrap();
    assert!(m.state().can_redo);
    m.begin("新操作");
    m.record_file(&path).unwrap();
    fs::write(&path, "新").unwrap();
    m.commit().unwrap();
    assert!(!m.state().can_redo, "新写操作应清空 redo 栈");

    let mut steps = 0;
    while m.undo().unwrap().is_some() {
      steps += 1;
    }
    // 55 步截到 50 → 撤销 3 步剩 47 → 新 commit 1 步 = 48
    assert_eq!(steps, 48, "undo 栈上限 {MAX_STEPS}，最旧的 5 步已丢弃");
    assert!(!m.state().can_undo, "空栈 can_undo = false");
    assert!(m.redo().unwrap().is_some(), "全部撤销后 redo 栈可用");
  }

  /// 磁盘逆操作：Rename 成对记录，undo 改回 / redo 再改过去；目标缺失跳过不炸
  #[test]
  fn disk_rename_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("旧名.fla");
    let new = dir.path().join("新名.fla");
    fs::write(&old, "x").unwrap();

    let mut m = manager();
    m.begin("重命名");
    fs::rename(&old, &new).unwrap();
    m.record_disk_op(
      DiskOp::Rename { from: new.clone(), to: old.clone() },
      DiskOp::Rename { from: old.clone(), to: new.clone() },
    );
    m.commit().unwrap();

    m.undo().unwrap();
    assert!(old.exists() && !new.exists());
    m.redo().unwrap();
    assert!(new.exists() && !old.exists());
    // 外部把文件删了再撤销：跳过不报错
    fs::remove_file(&new).unwrap();
    m.undo().unwrap();
    assert!(!old.exists());
  }

  /// 目录形态的磁盘逆操作（duplicate 副本 info）：undo 递归删除、redo 递归复制还原
  #[test]
  fn disk_copy_delete_directories_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("旧.info");
    let copy = dir.path().join("新.info");
    fs::create_dir_all(src.join("frames")).unwrap();
    fs::write(src.join("metadata.json"), "{}").unwrap();
    fs::write(src.join("frames/f1.png"), "x").unwrap();

    let mut m = manager();
    m.begin("创建副本");
    copy_dir(&src, &copy).unwrap();
    m.record_disk_op(DiskOp::Delete(copy.clone()), DiskOp::Copy { from: src.clone(), to: copy.clone() });
    m.commit().unwrap();

    m.undo().unwrap();
    assert!(!copy.exists(), "undo 应递归删除副本目录");
    m.redo().unwrap();
    assert_eq!(fs::read_to_string(copy.join("metadata.json")).unwrap(), "{}");
    assert_eq!(fs::read_to_string(copy.join("frames/f1.png")).unwrap(), "x", "redo 应递归复制还原");
  }

  /// 无事务时 record 静默；begin 覆盖未提交事务；clear 清空全部
  #[test]
  fn pending_guards_and_clear() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.json");
    fs::write(&path, "x").unwrap();

    let mut m = manager();
    m.record_file(&path).unwrap(); // 无事务静默
    m.commit().unwrap(); // 无事务静默
    assert!(!m.state().can_undo);

    m.begin("被覆盖的事务");
    m.begin("正式事务"); // 前者未 commit 被丢弃
    m.record_file(&path).unwrap();
    m.commit().unwrap();
    assert_eq!(m.state().undo_label.as_deref(), Some("正式事务"));

    m.clear();
    let st = m.state();
    assert!(!st.can_undo && !st.can_redo && st.undo_label.is_none());
  }
}
