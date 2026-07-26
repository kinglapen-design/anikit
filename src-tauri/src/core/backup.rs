//! 数据备份（PRD 06-04）：
//! - 文件夹形式存放 backup/{时间戳}-{版本}/，直接存放数据文件副本，不用压缩包；
//! - 仅变动文件复制实体，未变动文件硬链接上一份备份（NTFS 硬链接零额外空间；
//!   每份备份都是完整快照，回滚不需要跨备份拼接，删除旧备份不影响其余）；
//! - 滚动 10 份，超出删最旧；缓存与向量库不备份（可重建）。
//!
//! 备份对象 = library::data_files() 清单：库根数据文件 + 全部 info 内 metadata.json / annotation.json / notes.json / frames.json。

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{Local, TimeZone};

use crate::core::json_io::DataError;
use crate::core::library::{data_files, BACKUP_DIR};
use crate::core::models::DATA_VERSION;

const MAX_BACKUPS: usize = 10;

/// 执行一次备份，返回新备份文件夹路径。
pub fn backup_library(root: &Path) -> Result<PathBuf, DataError> {
  let backup_root = root.join(BACKUP_DIR);
  fs::create_dir_all(&backup_root)?;

  let dest = backup_root.join(new_backup_name(&backup_root));
  fs::create_dir_all(&dest)?;

  let previous = latest_backup(&backup_root).filter(|p| *p != dest);
  let mut copied = 0usize;
  for file in data_files(root)? {
    let rel = file.strip_prefix(root).expect("数据文件必须在库根内").to_path_buf();
    let target = dest.join(&rel);
    if let Some(parent) = target.parent() {
      fs::create_dir_all(parent)?;
    }
    // 与上一份备份同源文件对比：内容一致则硬链接，不同（或上一份没有）才复制实体
    let unchanged = previous.as_ref().is_some_and(|p| same_content(&p.join(&rel), &file));
    if unchanged {
      let prev_file = previous.as_ref().expect("内容一致必有上一份").join(&rel);
      if fs::hard_link(&prev_file, &target).is_ok() {
        continue;
      }
      // 硬链接失败（文件系统不支持等）退回复制，不阻断备份
    }
    fs::copy(&file, &target)?;
    copied += 1;
  }
  log::info!("数据备份完成：{}（{copied} 个变动文件）", dest.display());

  prune_old_backups(&backup_root)?;
  Ok(dest)
}

/// 距上次备份超过 24 小时则备份（PRD 06-04 备份时机：启动时）；从未备份过立即备份。
/// 备份失败不阻断库打开，只记错误日志。
pub fn backup_if_stale(root: &Path) {
  let stale = match latest_backup(&root.join(BACKUP_DIR)) {
    None => true,
    Some(p) => backup_time(&p).map(|t| Local::now().signed_duration_since(t).num_hours() >= 24).unwrap_or(true),
  };
  if stale {
    if let Err(e) = backup_library(root) {
      log::error!("定时备份失败（不阻断打开）：{e}");
    }
  }
}

/// 备份文件夹名：2026.0721.13.05-1（时间戳-数据版本，PRD 06-01 示例格式），重名追加 .序号
fn new_backup_name(backup_root: &Path) -> String {
  let stamp = Local::now().format("%Y.%m%d.%H.%M");
  let mut name = format!("{stamp}-{DATA_VERSION}");
  let mut n = 1;
  while backup_root.join(&name).exists() {
    n += 1;
    name = format!("{stamp}-{DATA_VERSION}.{n}");
  }
  name
}

/// 最新一份备份（名称含时间戳且等宽，字典序 = 时间序）
pub fn latest_backup(backup_root: &Path) -> Option<PathBuf> {
  let mut names: Vec<_> = fs::read_dir(backup_root)
    .ok()?
    .filter_map(|e| e.ok())
    .filter(|e| e.path().is_dir())
    .map(|e| e.file_name())
    .collect();
  names.sort();
  names.last().map(|n| backup_root.join(n))
}

/// 从备份文件夹名解析备份时间（前 15 字符为 %Y.%m%d.%H.%M）
fn backup_time(path: &Path) -> Option<chrono::DateTime<Local>> {
  let name = path.file_name()?.to_str()?;
  let stamp = name.get(..15)?;
  let naive = chrono::NaiveDateTime::parse_from_str(stamp, "%Y.%m%d.%H.%M").ok()?;
  Local.from_local_datetime(&naive).single()
}

fn same_content(a: &Path, b: &Path) -> bool {
  match (fs::read(a), fs::read(b)) {
    (Ok(x), Ok(y)) => x == y,
    _ => false,
  }
}

/// 滚动保留最新 10 份，删除最旧
fn prune_old_backups(backup_root: &Path) -> Result<(), DataError> {
  let mut dirs: Vec<_> = fs::read_dir(backup_root)?
    .filter_map(|e| e.ok())
    .filter(|e| e.path().is_dir())
    .collect();
  dirs.sort_by_key(|e| e.file_name());
  while dirs.len() > MAX_BACKUPS {
    let oldest = dirs.remove(0);
    fs::remove_dir_all(oldest.path())?;
    log::info!("旧备份已滚动删除：{}", oldest.path().display());
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::library;

  #[test]
  fn backup_copies_changed_and_links_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    library::create(root).unwrap();

    let first = backup_library(root).unwrap();
    assert!(first.join("folders.json").is_file());

    // 改动一个数据文件后再备份
    let new_tags = r#"{ "version": 1, "tags": [ { "id": "T01", "name": "男人", "group": null } ], "groups": [] }"#;
    fs::write(root.join("tags.json"), new_tags).unwrap();
    let second = backup_library(root).unwrap();

    // 第二份里变动文件是新内容（与库根一致、与第一份不同），未变动文件与上一份内容一致
    assert_eq!(fs::read_to_string(second.join("tags.json")).unwrap(), new_tags);
    assert_ne!(
      fs::read(first.join("tags.json")).unwrap(),
      fs::read(second.join("tags.json")).unwrap()
    );
    assert_eq!(
      fs::read(first.join("folders.json")).unwrap(),
      fs::read(second.join("folders.json")).unwrap()
    );
    // 两份备份各自是完整快照（库根五个数据文件齐全）
    for name in ["folders.json", "tags.json", "pins.json", "recycle.json", "recent.json"] {
      assert!(first.join(name).is_file(), "第一份缺 {name}");
      assert!(second.join(name).is_file(), "第二份缺 {name}");
    }
  }

  #[test]
  fn backup_covers_info_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    library::create(root).unwrap();
    let info = root.join("asstes/AN2607211305-X7K2Q.info");
    fs::create_dir_all(&info).unwrap();
    fs::write(info.join("metadata.json"), r#"{ "version": 1 }"#).unwrap();
    fs::write(info.join("notes.json"), r#"{ "version": 1, "notes": [] }"#).unwrap();

    let backup = backup_library(root).unwrap();
    assert!(backup.join("asstes/AN2607211305-X7K2Q.info/metadata.json").is_file());
    assert!(backup.join("asstes/AN2607211305-X7K2Q.info/notes.json").is_file());
  }

  #[test]
  fn prunes_to_ten_backups() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    library::create(root).unwrap();

    for _ in 0..MAX_BACKUPS + 1 {
      backup_library(root).unwrap();
    }
    let backups: Vec<_> = fs::read_dir(root.join(BACKUP_DIR))
      .unwrap()
      .filter_map(|e| e.ok())
      .filter(|e| e.path().is_dir())
      .collect();
    assert_eq!(backups.len(), MAX_BACKUPS);
  }
}
