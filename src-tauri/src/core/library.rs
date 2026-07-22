//! 素材库结构：创建 / 校验 / 打开（PRD 01-1、06-01）。
//! 库根布局：asstes/ backup/ cache/ vectors/ + 五个数据文件；
//! 内部路径全部相对化，整个目录可整体搬迁、换盘符不影响使用（PRD 06-01.1）。

use std::collections::HashSet;
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
/// 收件箱（PRD 05-5.1：自动导入目录，过境语义、常态为空；subst 挂载与 notify 监听在阶段 3 实现）
pub const INBOX_DIR: &str = "inbox";
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
  /// 已缓存系统格式图标的扩展名集（cache/thumbs/{ext}.webp，4.2）——视图层回退查内存集，免逐卡片 stat
  pub format_icons: HashSet<String>,
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

/// 资源库名称合法性（Windows 文件夹命名规则；前端表单同规则，命令接口统一把关）
fn validate_library_name(name: &str) -> Result<(), DataError> {
  if name.is_empty() {
    return Err(DataError::NameInvalid("名称不能留空"));
  }
  if name.contains(['\\', '/', ':', '*', '?', '"', '<', '>', '|']) {
    return Err(DataError::NameInvalid("名称不能包含 \\ / : * ? \" < > | 特殊符号"));
  }
  if name.ends_with([' ', '.']) {
    return Err(DataError::NameInvalid("名称不能以空格或点结尾"));
  }
  const RESERVED: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
  ];
  if RESERVED.contains(&name.to_ascii_lowercase().as_str()) {
    return Err(DataError::NameInvalid("该名称为系统保留名，请换一个"));
  }
  Ok(())
}

/// 在指定文件夹创建素材库（文件夹名即素材库名）。要求文件夹为空；已是资源库则提示改用打开。
pub fn create(path: &Path) -> Result<Library, DataError> {
  // 名称校验在 mkdir 之前，避免建出非法 / 半截目录
  let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
  validate_library_name(name)?;
  fs::create_dir_all(path)?;
  if validate(path).is_ok() {
    return Err(DataError::AlreadyLibrary);
  }
  if fs::read_dir(path)?.next().is_some() {
    return Err(DataError::NotEmpty);
  }
  for dir in [ASSTES_DIR, BACKUP_DIR, CACHE_DIR, VECTORS_DIR, INBOX_DIR] {
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
  for dir in [BACKUP_DIR, CACHE_DIR, VECTORS_DIR, INBOX_DIR] {
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
  Ok(Library { root: path.to_path_buf(), folders, tags, pins, recycle, recent, index, format_icons: scan_format_icons(path) })
}

/// 扫描 cache/thumbs/*.webp 建格式图标内存集（4.2；缓存可重建，目录缺失 / 读取失败按空集处理）
fn scan_format_icons(root: &Path) -> HashSet<String> {
  fs::read_dir(root.join(CACHE_DIR).join("thumbs"))
    .map(|rd| {
      rd.filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("webp"))
        .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .collect()
    })
    .unwrap_or_default()
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

/// folders.json 单一写点（01_开发规范 第 1 条）：全量保存文件夹树 + 全部 Placement。
pub fn save_folders(lib: &Library) -> Result<(), DataError> {
  write_json(&lib.root.join(FOLDERS_FILE), &lib.folders)
}

/// 同文件夹内显示名唯一化：无冲突原样返回；冲突时在扩展名前追加 -序号
/// （如 男人-1.fla、男人-2.fla；无扩展名则在末尾追加，如 参考-1）。
pub fn unique_name(existing: &HashSet<String>, name: &str) -> String {
  if !existing.contains(name) {
    return name.to_string();
  }
  // rsplit_once 取最后一个点："a.b.fla" → ("a.b", "fla")；".hidden" 不视为扩展名
  let (stem, ext) = match name.rsplit_once('.') {
    Some((s, e)) if !s.is_empty() => (s.to_string(), format!(".{e}")),
    _ => (name.to_string(), String::new()),
  };
  let mut n = 1;
  loop {
    let candidate = format!("{stem}-{n}{ext}");
    if !existing.contains(&candidate) {
      return candidate;
    }
    n += 1;
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn create_initializes_full_structure() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("我的素材库");
    let lib = create(&root).unwrap();

    for d in [ASSTES_DIR, BACKUP_DIR, CACHE_DIR, VECTORS_DIR, INBOX_DIR] {
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
  fn create_rejects_invalid_names() {
    let dir = tempfile::tempdir().unwrap();
    // 特殊符号 / 结尾点 / 系统保留名（Windows 文件夹命名规则）
    for name in ["bad:name", "bad|name", "lib.", "lib ", "CON", "com1", "Lpt9"] {
      let result = create(&dir.path().join(name));
      assert!(matches!(result, Err(DataError::NameInvalid(_))), "名称 {name} 应被拒绝");
      assert!(!dir.path().join(name).exists(), "名称 {name} 不应建出目录");
    }
    // 合法中文名可用
    assert!(create(&dir.path().join("我的素材")).is_ok());
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

  #[test]
  fn save_folders_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("库");
    let mut lib = create(&root).unwrap();
    lib.folders.folders.push(Folder { id: "F01".into(), name: "人物".into(), parent: None, order: 0 });
    save_folders(&lib).unwrap();
    let back: FoldersFile = read_json(&root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(back.folders.len(), 1);
    assert_eq!(back.folders[0].name, "人物");
  }

  #[test]
  fn unique_name_appends_counter_before_ext() {
    let existing: HashSet<String> =
      ["男人.fla".to_string(), "男人-1.fla".to_string(), "参考".to_string()].into_iter().collect();
    // 无冲突原样返回
    assert_eq!(unique_name(&existing, "女人.fla"), "女人.fla");
    // 冲突在扩展名前追加 -序号，已占用的序号跳过
    assert_eq!(unique_name(&existing, "男人.fla"), "男人-2.fla");
    // 无扩展名在末尾追加
    assert_eq!(unique_name(&existing, "参考"), "参考-1");
    // 点开头不视为扩展名
    let hidden: HashSet<String> = [".gitignore".to_string()].into_iter().collect();
    assert_eq!(unique_name(&hidden, ".gitignore"), ".gitignore-1");
  }
}
