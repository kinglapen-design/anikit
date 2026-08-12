//! 储存空间（PRD 01_储存空间）：资源库的外层容器。
//! 空间根布局：space.json（标志与元数据）+ assets/（统一素材池，{ID}.info 平铺）
//! + inbox/（空间级共用收件箱，2026-08-12 用户定）
//! + workspace/（索引式工作空间，创建时建空目录，功能阶段九接）+ 各资源库目录。

use std::fs;
use std::path::{Path, PathBuf};

use crate::core::json_io::{read_json, write_json, DataError};
use crate::core::library::{self, FOLDERS_FILE};
use crate::core::models::SpaceFile;
use crate::core::paths;

pub const SPACE_FILE: &str = "space.json";
pub const WORKSPACE_DIR: &str = "workspace";
/// 收件箱（空间级共用：盘符挂载恒定指向它、不随切库变化；自动导入目标 = 当前打开的库）
pub const INBOX_DIR: &str = "inbox";

/// 打开状态的储存空间
#[derive(Debug, Clone)]
pub struct Space {
  pub root: PathBuf,
  pub name: String,
  /// 创建时间（原样往返，save_space 单写点回写不丢）
  pub created_at: String,
  /// 自动导入开关（space.json，空间级设置）
  pub auto_import: bool,
}

/// 在 parent 下创建储存空间（文件夹名即空间名）。auto_import 写 space.json（空间级设置）。
/// parent/name 已是合法空间 → 视为打开；已存在且非空非空间 → 报错；不存在（或空目录）→ 创建。
pub fn create(parent: &Path, name: &str, auto_import: bool) -> Result<Space, DataError> {
  // 名称校验在 mkdir 之前，避免建出非法 / 半截目录（与资源库同一套 Windows 名称规则）
  library::validate_library_name(name)?;
  let root = parent.join(name);
  if root.exists() {
    if validate(&root).is_ok() {
      return open(&root);
    }
    if !root.is_dir() {
      return Err(DataError::NameInvalid("同名文件已存在，请换一个名称"));
    }
    if fs::read_dir(&root)?.next().is_some() {
      return Err(DataError::NotEmpty);
    }
  }
  fs::create_dir_all(paths::assets_dir(&root))?;
  fs::create_dir_all(root.join(INBOX_DIR))?;
  fs::create_dir_all(root.join(WORKSPACE_DIR))?;
  write_json(&root.join(SPACE_FILE), &SpaceFile::new(name, auto_import))?;
  log::info!("储存空间已创建：{}", root.display());
  open(&root)
}

/// 打开准入校验：space.json 存在且可解析（不是什么文件夹都能当储存空间打开）。
/// assets/ 与 inbox/ 缺失不在此拦截——open 时自愈补建（与库 open 自愈同风格）
pub fn validate(path: &Path) -> Result<(), DataError> {
  let space_file = path.join(SPACE_FILE);
  if !space_file.is_file() {
    return Err(DataError::NotASpace);
  }
  read_json::<serde_json::Value>(&space_file).map_err(|_| DataError::NotASpace)?;
  Ok(())
}

/// 打开储存空间：校验 space.json → assets/、inbox/ 缺失自愈补建 → 读空间元数据
pub fn open(path: &Path) -> Result<Space, DataError> {
  validate(path)?;
  fs::create_dir_all(paths::assets_dir(path))?;
  fs::create_dir_all(path.join(INBOX_DIR))?;
  let file: SpaceFile = read_json(&path.join(SPACE_FILE))?;
  Ok(Space { root: path.to_path_buf(), name: file.name, created_at: file.created_at, auto_import: file.auto_import })
}

/// space.json 单一写点（01_开发规范 第 1 条）：全量保存空间元数据（autoImport 等）
pub fn save_space(space: &Space) -> Result<(), DataError> {
  let file = SpaceFile {
    version: crate::core::models::DATA_VERSION,
    name: space.name.clone(),
    created_at: space.created_at.clone(),
    auto_import: space.auto_import,
  };
  write_json(&space.root.join(SPACE_FILE), &file)
}

/// 空间内资源库名列表：只扫空间根一级子目录，含 folders.json 的目录即库。
/// 热路径纪律（PRD assets 池访问纪律）：不递归、不碰 assets/
pub fn list_libraries(space_root: &Path) -> Result<Vec<String>, DataError> {
  let mut names: Vec<String> = fs::read_dir(space_root)?
    .filter_map(|e| e.ok())
    .map(|e| e.path())
    .filter(|p| p.is_dir() && p.join(FOLDERS_FILE).is_file())
    .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
    .collect();
  names.sort();
  Ok(names)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn create_initializes_full_structure() {
    let dir = tempfile::tempdir().unwrap();
    let space = create(dir.path(), "我的空间", true).unwrap();

    let root = dir.path().join("我的空间");
    assert!(root.join(SPACE_FILE).is_file());
    assert!(paths::assets_dir(&root).is_dir());
    assert!(root.join(INBOX_DIR).is_dir(), "空间级收件箱目录应随创建建立");
    assert!(root.join(WORKSPACE_DIR).is_dir());
    assert_eq!(space.name, "我的空间");
    assert!(space.auto_import);
    // space.json 结构：{version, name, createdAt, autoImport}
    let file: SpaceFile = read_json(&root.join(SPACE_FILE)).unwrap();
    assert_eq!(file.version, crate::core::models::DATA_VERSION);
    assert_eq!(file.name, "我的空间");
    assert!(!file.created_at.is_empty());
    assert!(file.auto_import);
    // 单写点改写后落盘，createdAt 原样保留
    let mut space = space;
    space.auto_import = false;
    save_space(&space).unwrap();
    let disk: SpaceFile = read_json(&root.join(SPACE_FILE)).unwrap();
    assert!(!disk.auto_import);
    assert_eq!(disk.created_at, file.created_at);
  }

  /// 旧版 space.json（无 autoImport 字段，2026-08-12 前创建）可读，auto_import 默认 false
  #[test]
  fn open_reads_legacy_space_file_without_auto_import() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("空间");
    fs::create_dir_all(paths::assets_dir(&root)).unwrap();
    fs::write(
      root.join(SPACE_FILE),
      r#"{ "version": 1, "name": "空间", "createdAt": "2026-08-01T00:00:00+08:00" }"#,
    )
    .unwrap();
    let space = open(&root).unwrap();
    assert!(!space.auto_import, "旧 space.json 缺 autoImport 应按默认 false 读取");
    assert!(root.join(INBOX_DIR).is_dir(), "inbox/ 缺失应自愈补建");
  }

  #[test]
  fn create_opens_existing_space() {
    let dir = tempfile::tempdir().unwrap();
    create(dir.path(), "空间", false).unwrap();
    // 已存在且是合法空间 → 视为打开，不报错
    let again = create(dir.path(), "空间", false).unwrap();
    assert_eq!(again.name, "空间");
  }

  #[test]
  fn create_rejects_non_empty_non_space() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("空间");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("随便一个文件.txt"), "hi").unwrap();
    assert!(matches!(create(dir.path(), "空间", false), Err(DataError::NotEmpty)));
  }

  #[test]
  fn create_fills_empty_existing_dir() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("空间")).unwrap();
    assert!(create(dir.path(), "空间", false).is_ok(), "已存在的空目录应可直接初始化为空间");
  }

  #[test]
  fn create_rejects_invalid_names() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["bad:name", "空间.", "空间 ", "CON"] {
      let result = create(dir.path(), name, false);
      assert!(matches!(result, Err(DataError::NameInvalid(_))), "名称 {name} 应被拒绝");
      assert!(!dir.path().join(name).exists(), "名称 {name} 不应建出目录");
    }
  }

  #[test]
  fn open_heals_missing_assets_dir() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("空间");
    create(dir.path(), "空间", false).unwrap();
    fs::remove_dir_all(paths::assets_dir(&root)).unwrap();

    let space = open(&root).unwrap();
    assert_eq!(space.name, "空间");
    assert!(paths::assets_dir(&root).is_dir(), "assets/ 缺失应自愈补建");
  }

  #[test]
  fn open_rejects_non_space() {
    let dir = tempfile::tempdir().unwrap();
    // 无 space.json
    assert!(matches!(open(dir.path()), Err(DataError::NotASpace)));
    // space.json 损坏
    fs::write(dir.path().join(SPACE_FILE), "{ broken").unwrap();
    assert!(matches!(open(dir.path()), Err(DataError::NotASpace)));
  }

  #[test]
  fn list_libraries_scans_only_top_level() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("空间");
    create(dir.path(), "空间", false).unwrap();
    // 两个库目录（含 folders.json）+ 一个普通目录 + assets/workspace（无 folders.json）
    for lib in ["库B", "库A"] {
      fs::create_dir_all(root.join(lib)).unwrap();
      fs::write(root.join(lib).join(FOLDERS_FILE), r#"{ "version": 1, "folders": [], "items": [] }"#).unwrap();
    }
    fs::create_dir_all(root.join("普通目录")).unwrap();
    // assets/ 里塞一个带 folders.json 的 info（不应被扫到——只扫空间根一级）
    let info = paths::info_dir(&paths::assets_dir(&root), "AN-1");
    fs::create_dir_all(&info).unwrap();
    fs::write(info.join(FOLDERS_FILE), "{}").unwrap();

    assert_eq!(list_libraries(&root).unwrap(), vec!["库A".to_string(), "库B".to_string()]);
  }
}
