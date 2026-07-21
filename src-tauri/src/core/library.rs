//! 素材库结构：创建 / 校验 / 打开（PRD 01-1、06-01）。
//! 库根布局：asstes/ backup/ cache/ vectors/ + 五个数据文件；
//! 内部路径全部相对化，整个目录可整体搬迁、换盘符不影响使用（PRD 06-01.1）。

use std::fs;
use std::path::{Path, PathBuf};

use crate::core::backup;
use crate::core::index::LibraryIndex;
use crate::core::json_io::{read_json, write_json, DataError};
use crate::core::migrate;
use crate::core::models::*;

pub const ASSTES_DIR: &str = "asstes";
pub const BACKUP_DIR: &str = "backup";
pub const CACHE_DIR: &str = "cache";
pub const VECTORS_DIR: &str = "vectors";
pub const FOLDERS_FILE: &str = "folders.json";
pub const TAGS_FILE: &str = "tags.json";
pub const PINS_FILE: &str = "pins.json";
pub const RECYCLE_FILE: &str = "recycle.json";
pub const RECENT_FILE: &str = "recent.json";

/// 库根数据文件（均为备份对象，PRD 06-04）
pub const ROOT_DATA_FILES: &[&str] = &[FOLDERS_FILE, TAGS_FILE, PINS_FILE, RECYCLE_FILE, RECENT_FILE];

/// info 文件夹内的数据文件（备份对象；缩略图/波形图是缓存可重建，不备份）
const INFO_DATA_FILES: &[&str] = &["metadata.json", "annotation.json"];

/// 打开状态的素材库：数据 + 内存索引（运行期查询走内存，写操作先落盘再更新内存，PRD 06-03）
#[allow(dead_code)] // 各数据字段随阶段 3/4 业务命令落地逐一启用
pub struct Library {
  pub root: PathBuf,
  pub folders: FoldersFile,
  pub tags: TagsFile,
  pub pins: PinsFile,
  pub recycle: RecycleFile,
  pub recent: RecentFile,
  pub index: LibraryIndex,
}

/// 数据文件全量清单：库根五个 + asstes/*.info/{metadata,annotation}.json（备份与迁移共用）
pub fn data_files(root: &Path) -> Result<Vec<PathBuf>, DataError> {
  let mut files: Vec<PathBuf> =
    ROOT_DATA_FILES.iter().map(|f| root.join(f)).filter(|p| p.exists()).collect();
  for info in info_dirs(root)? {
    for name in INFO_DATA_FILES {
      let p = info.join(name);
      if p.exists() {
        files.push(p);
      }
    }
  }
  Ok(files)
}

/// 列出 asstes/ 下全部 {ID}.info 文件夹
fn info_dirs(root: &Path) -> Result<Vec<PathBuf>, DataError> {
  let asstes = root.join(ASSTES_DIR);
  if !asstes.is_dir() {
    return Ok(Vec::new());
  }
  Ok(fs::read_dir(asstes)?
    .filter_map(|e| e.ok())
    .map(|e| e.path())
    .filter(|p| p.is_dir() && p.extension().and_then(|s| s.to_str()) == Some("info"))
    .collect())
}

/// 打开准入校验（UI 06_创建资源库：不是什么文件夹都能打开，必须是我们的资源库结构）。
/// 判据：asstes/ 目录存在 + folders.json 可解析且版本合法。
pub fn validate(path: &Path) -> Result<(), DataError> {
  if !path.join(ASSTES_DIR).is_dir() {
    return Err(DataError::NotALibrary);
  }
  let folders_path = path.join(FOLDERS_FILE);
  if !folders_path.is_file() {
    return Err(DataError::NotALibrary);
  }
  read_json::<serde_json::Value>(&folders_path).map(|_| ())
}

/// 在指定文件夹创建素材库（文件夹名即素材库名）。要求文件夹为空；已是资源库则提示改用打开。
pub fn create(path: &Path) -> Result<Library, DataError> {
  fs::create_dir_all(path)?;
  if validate(path).is_ok() {
    return Err(DataError::AlreadyLibrary);
  }
  if fs::read_dir(path)?.next().is_some() {
    return Err(DataError::NotEmpty);
  }
  for dir in [ASSTES_DIR, BACKUP_DIR, CACHE_DIR, VECTORS_DIR] {
    fs::create_dir_all(path.join(dir))?;
  }
  write_json(&path.join(FOLDERS_FILE), &FoldersFile::new())?;
  write_json(&path.join(TAGS_FILE), &TagsFile::new())?;
  write_json(&path.join(PINS_FILE), &PinsFile::new())?;
  write_json(&path.join(RECYCLE_FILE), &RecycleFile::new())?;
  write_json(&path.join(RECENT_FILE), &RecentFile::new())?;
  log::info!("素材库已创建：{}", path.display());
  open(path)
}

/// 打开素材库：校验结构 → 版本迁移检查（迁移前自动备份）→ 读全部数据 → 建内存索引 → 定期备份。
/// 缺失的子目录自动补建（自愈）；缺失的数据文件视为损坏，报错提示回滚（PRD 06-04）。
pub fn open(path: &Path) -> Result<Library, DataError> {
  validate(path)?;
  for dir in [BACKUP_DIR, CACHE_DIR, VECTORS_DIR] {
    fs::create_dir_all(path.join(dir))?;
  }
  migrate::migrate_library_if_needed(path)?;
  let folders: FoldersFile = read_json(&path.join(FOLDERS_FILE))?;
  let tags: TagsFile = read_json(&path.join(TAGS_FILE))?;
  let pins: PinsFile = read_json(&path.join(PINS_FILE))?;
  let recycle: RecycleFile = read_json(&path.join(RECYCLE_FILE))?;
  let recent: RecentFile = read_json(&path.join(RECENT_FILE))?;
  let index = LibraryIndex::build(&folders, load_assets(path)?);
  backup::backup_if_stale(path);
  Ok(Library { root: path.to_path_buf(), folders, tags, pins, recycle, recent, index })
}

/// 扫描 asstes/*.info/metadata.json 读入全部资源元数据；任一损坏即报错（提示回滚，不吞错误）
fn load_assets(root: &Path) -> Result<Vec<AssetMeta>, DataError> {
  let mut assets = Vec::new();
  for info in info_dirs(root)? {
    let meta_path = info.join("metadata.json");
    if meta_path.is_file() {
      assets.push(read_json(&meta_path)?);
    }
  }
  Ok(assets)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn create_initializes_full_structure() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("我的素材库");
    let lib = create(&root).unwrap();

    for d in [ASSTES_DIR, BACKUP_DIR, CACHE_DIR, VECTORS_DIR] {
      assert!(root.join(d).is_dir(), "缺目录 {d}");
    }
    for f in ROOT_DATA_FILES {
      assert!(root.join(f).is_file(), "缺数据文件 {f}");
    }
    assert_eq!(lib.folders.version, DATA_VERSION);
    assert!(lib.index.assets.is_empty());
  }

  #[test]
  fn create_rejects_non_empty_folder() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("随便一个文件.txt"), "hi").unwrap();
    assert!(matches!(create(dir.path()), Err(DataError::NotEmpty)));
  }

  #[test]
  fn create_rejects_existing_library() {
    let dir = tempfile::tempdir().unwrap();
    create(dir.path()).unwrap();
    assert!(matches!(create(dir.path()), Err(DataError::AlreadyLibrary)));
  }

  #[test]
  fn validate_rejects_plain_folder() {
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(validate(dir.path()), Err(DataError::NotALibrary)));
    // 只有 folders.json 没有 asstes/ 也不算
    fs::write(dir.path().join(FOLDERS_FILE), r#"{ "version": 1, "folders": [], "items": [] }"#).unwrap();
    assert!(matches!(validate(dir.path()), Err(DataError::NotALibrary)));
  }

  #[test]
  fn open_loads_assets_into_index() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    create(root).unwrap();

    // 造一个资源 info 文件夹 + 一条 Placement
    let info = root.join("asstes/AN2607211305-X7K2Q.info");
    fs::create_dir_all(&info).unwrap();
    fs::write(
      info.join("metadata.json"),
      r#"{
        "version": 1, "id": "AN2607211305-X7K2Q", "name": "人物", "originName": "人物.fla",
        "ext": "fla", "size": 100, "importedAt": "2026-07-21T13:05:00+08:00",
        "modifiedAt": "2026-07-21T13:05:00+08:00"
      }"#,
    )
    .unwrap();
    let folders: FoldersFile = serde_json::from_str(
      r#"{
        "version": 1,
        "folders": [ { "id": "F01", "name": "人物", "parent": null, "order": 0 } ],
        "items": [ { "asset": "AN2607211305-X7K2Q", "folder": "F01", "name": "男人.fla", "order": 0, "origin": true } ]
      }"#,
    )
    .unwrap();
    write_json(&root.join(FOLDERS_FILE), &folders).unwrap();

    let lib = open(root).unwrap();
    assert_eq!(lib.index.assets.len(), 1);
    assert_eq!(lib.index.placements_of("AN2607211305-X7K2Q").len(), 1);
    assert_eq!(lib.index.placements_by_folder["F01"].len(), 1);
    // 首次打开触发一次定时备份
    assert!(root.join(BACKUP_DIR).read_dir().unwrap().next().is_some());
  }

  #[test]
  fn open_reports_corrupt_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    create(root).unwrap();
    let info = root.join("asstes/AN2607211305-X7K2Q.info");
    fs::create_dir_all(&info).unwrap();
    fs::write(info.join("metadata.json"), "{ broken").unwrap();
    assert!(matches!(open(root), Err(DataError::Corrupt(_))));
  }
}
