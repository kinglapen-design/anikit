//! 储存空间（PRD 01_储存空间）：资源库的外层容器。
//! 空间根布局：space.json（标志与元数据）+ assets/（统一素材池，{ID}.info 平铺）
//! + inbox/（空间级共用收件箱，2026-08-12 用户定）
//! + workspace/（索引式工作空间，创建时建空目录，功能阶段九接）+ 各资源库目录。

use std::fs;
use std::path::{Path, PathBuf};

use std::collections::HashSet;

use crate::core::json_io::{read_json, write_json, DataError};
use crate::core::library::{self, FOLDERS_FILE, RECYCLE_FILE};
use crate::core::models::{AssetMeta, FoldersFile, RecycleFile, SpaceFile};
use crate::core::paths;

pub const SPACE_FILE: &str = "space.json";
pub const WORKSPACE_DIR: &str = "workspace";
/// 收件箱（空间级共用：盘符挂载恒定指向它、不随切库变化；自动导入目标 = 当前打开的库）。
/// 目录名即盘符显示名（subst 机制：资源管理器显示目标目录名）——"自动导入"与 UI 语义同步（2026-08-12 用户定）
pub const INBOX_DIR: &str = "自动导入";

/// 打开状态的储存空间
#[derive(Debug, Clone)]
pub struct Space {
  pub root: PathBuf,
  pub name: String,
  /// 创建时间（原样往返，save_space 单写点回写不丢）
  pub created_at: String,
  /// 自动导入开关（space.json，空间级设置）
  pub auto_import: bool,
  /// 资源库自定义顺序（菜单抓手拖动排序，PRD 01_储存空间-3.2）：新库排尾，列表按此返回
  pub library_order: Vec<String>,
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

/// 打开储存空间：校验 space.json → assets/、自动导入/ 缺失自愈补建（旧 inbox/ 迁移正名）→ 读空间元数据
pub fn open(path: &Path) -> Result<Space, DataError> {
  validate(path)?;
  fs::create_dir_all(paths::assets_dir(path))?;
  // 旧 inbox/ 迁移（2026-08-12 目录正名"自动导入"，盘符显示名与 UI 语义同步）
  let legacy = path.join("inbox");
  let inbox_dir = path.join(INBOX_DIR);
  if legacy.is_dir() && !inbox_dir.exists() {
    fs::rename(&legacy, &inbox_dir)?;
  }
  fs::create_dir_all(inbox_dir)?;
  let file: SpaceFile = read_json(&path.join(SPACE_FILE))?;
  Ok(Space {
    root: path.to_path_buf(),
    name: file.name,
    created_at: file.created_at,
    auto_import: file.auto_import,
    library_order: file.library_order,
  })
}

/// space.json 单一写点（01_开发规范 第 1 条）：全量保存空间元数据（autoImport / libraryOrder 等）
pub fn save_space(space: &Space) -> Result<(), DataError> {
  let file = SpaceFile {
    version: crate::core::models::DATA_VERSION,
    name: space.name.clone(),
    created_at: space.created_at.clone(),
    auto_import: space.auto_import,
    library_order: space.library_order.clone(),
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

/// 库列表按 library_order 排序返回（PRD 01_储存空间-3.2 拖动排序）：
/// 在 order 内的按 order 序；不在 order 内的（用户手动建的目录）排尾、按名字典序
pub fn ordered_libraries(space: &Space) -> Result<Vec<String>, DataError> {
  let mut names = list_libraries(&space.root)?;
  let order = &space.library_order;
  names.sort_by(|a, b| {
    match (order.iter().position(|n| n == a), order.iter().position(|n| n == b)) {
      (Some(i), Some(j)) => i.cmp(&j),
      (Some(_), None) => std::cmp::Ordering::Less,
      (None, Some(_)) => std::cmp::Ordering::Greater,
      (None, None) => a.cmp(b),
    }
  });
  Ok(names)
}

/// 持久化库列表自定义顺序（3.2 菜单抓手拖动排序，save_space 单写点）：
/// order 必须与空间内实际库名集合一致，多 / 少都拒绝；返回按新顺序排好的最新库名列表
pub fn reorder_libraries(space: &mut Space, order: Vec<String>) -> Result<Vec<String>, DataError> {
  let mut actual = list_libraries(&space.root)?;
  actual.sort();
  let mut incoming = order.clone();
  incoming.sort();
  if incoming != actual {
    return Err(DataError::NameInvalid("资源库列表与实际不一致，请刷新后重试"));
  }
  space.library_order = order;
  save_space(space)?;
  ordered_libraries(space)
}

/// 库引用的资产 ID 集（孤儿判定口径，2026-08-12 定案）：
/// folders.json items[].asset ∪ recycle.json items[].asset —— 回收站里的条目也算引用
/// （回收站资产实体未删，彻底删除才删 info）。recycle.json 缺失按空集读
fn referenced_assets(lib_dir: &Path) -> Result<HashSet<String>, DataError> {
  let folders: FoldersFile = read_json(&lib_dir.join(FOLDERS_FILE))?;
  let mut ids: HashSet<String> = folders.items.iter().map(|p| p.asset.clone()).collect();
  let recycle_path = lib_dir.join(RECYCLE_FILE);
  if recycle_path.is_file() {
    let recycle: RecycleFile = read_json(&recycle_path)?;
    ids.extend(recycle.items.iter().map(|i| i.asset.clone()));
  }
  Ok(ids)
}

/// 孤儿资产集：仅被该库引用的资产 = 该库引用集 − 空间内其余库引用集并集（2026-08-12 定案）。
/// 只读各库数据文件，不枚举 assets/（热路径纪律）；库不存在报 NotALibrary
pub fn library_orphans(space_root: &Path, name: &str) -> Result<HashSet<String>, DataError> {
  let lib_dir = space_root.join(name);
  if !(lib_dir.is_dir() && lib_dir.join(FOLDERS_FILE).is_file()) {
    return Err(DataError::NotALibrary);
  }
  let own = referenced_assets(&lib_dir)?;
  let mut others: HashSet<String> = HashSet::new();
  for other in list_libraries(space_root)? {
    if other != name {
      others.extend(referenced_assets(&space_root.join(other))?);
    }
  }
  Ok(own.difference(&others).cloned().collect())
}

/// 库容量（合并资源库弹窗容量列，2026-09-26）：引用资产集（folders ∪ recycle，口径同孤儿判定）
/// 的 metadata.size 求和——资产实体在空间级 assets 池多库共享，"库容量"= 该库引用资产的合计。
/// 按 ID 经 core::paths 推导读 metadata（不枚举 assets/）；info / metadata 缺失或损坏跳过不计
pub fn library_size(space_root: &Path, name: &str) -> Result<u64, DataError> {
  let lib_dir = space_root.join(name);
  if !(lib_dir.is_dir() && lib_dir.join(FOLDERS_FILE).is_file()) {
    return Err(DataError::NotALibrary);
  }
  let assets = paths::assets_dir(space_root);
  let mut total = 0u64;
  for id in referenced_assets(&lib_dir)? {
    let meta_path = paths::info_dir(&assets, &id).join("metadata.json");
    if meta_path.is_file() {
      if let Ok(meta) = read_json::<AssetMeta>(&meta_path) {
        total += meta.size;
      }
    }
  }
  Ok(total)
}

/// 删除资源库（3.2 删除×；确认弹窗在前端）：整个库目录递归删除，库内数据文件删除不可恢复；
/// 素材实体保留在空间级 assets 池。delete_orphans = true 时一并删除仅被该库引用的孤儿素材实体
/// （2026-08-12 定案：必须先算孤儿集再删库目录——删后读不到该库的 folders.json / recycle.json；
/// info 删除按 ID 经 core::paths 单点推导，不枚举 assets/）。
/// 从 library_order 移除后落盘（save_space 单写点），返回删除后剩余库名列表（按 library_order 序）
pub fn delete_library(space: &mut Space, name: &str, delete_orphans: bool) -> Result<Vec<String>, DataError> {
  let path = space.root.join(name);
  if !(path.is_dir() && path.join(FOLDERS_FILE).is_file()) {
    return Err(DataError::NotALibrary);
  }
  let orphans = if delete_orphans { library_orphans(&space.root, name)? } else { HashSet::new() };
  fs::remove_dir_all(&path)?;
  let assets = paths::assets_dir(&space.root);
  for id in &orphans {
    let info = paths::info_dir(&assets, id);
    if info.is_dir() {
      fs::remove_dir_all(&info)?;
    }
  }
  space.library_order.retain(|n| n != name);
  save_space(space)?;
  ordered_libraries(space)
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

  /// 旧版 space.json（无 autoImport / libraryOrder 字段，2026-08-12 前创建）可读，缺字段按默认读
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
    assert!(space.library_order.is_empty(), "旧 space.json 缺 libraryOrder 应按默认空列表读取");
  assert!(root.join(INBOX_DIR).is_dir(), "inbox/ 缺失应自愈补建（迁移正名后为 自动导入/）");
  }

  #[test]
  fn open_migrates_legacy_inbox_dir() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("空间");
    fs::create_dir_all(paths::assets_dir(&root)).unwrap();
    fs::create_dir_all(root.join("inbox")).unwrap();
    fs::write(root.join("inbox").join("遗留.txt"), "hi").unwrap();
    fs::write(
      root.join(SPACE_FILE),
      r#"{ "version": 1, "name": "空间", "createdAt": "2026-08-01T00:00:00+08:00" }"#,
    )
    .unwrap();
    let space = open(&root).unwrap();
    assert!(space.root.join(INBOX_DIR).join("遗留.txt").is_file(), "旧 inbox/ 内容应迁入 自动导入/");
    assert!(!space.root.join("inbox").exists(), "旧 inbox/ 应已被正名");
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

  /// 造库辅助：空间根下建含 folders.json 的库目录
  fn make_lib_dir(root: &Path, name: &str) {
    fs::create_dir_all(root.join(name)).unwrap();
    fs::write(root.join(name).join(FOLDERS_FILE), r#"{ "version": 1, "folders": [], "items": [] }"#).unwrap();
  }

  /// libraryOrder 排序（PRD 3.2）：在 order 内的按 order 序；不在 order 内的外来目录排尾、按名字典序
  #[test]
  fn ordered_libraries_follows_library_order() {
    let dir = tempfile::tempdir().unwrap();
    let mut space = create(dir.path(), "空间", false).unwrap();
    for lib in ["库A", "库B", "库C", "外来乙", "外来甲"] {
      make_lib_dir(&space.root, lib);
    }
    // order 里混一条已不存在的陈旧条目（删库未清 order 的遗留），应被容忍
    space.library_order = vec!["库C".into(), "陈旧库".into(), "库A".into(), "库B".into()];
    let ordered = ordered_libraries(&space).unwrap();
    assert_eq!(&ordered[..3], ["库C", "库A", "库B"], "order 内的按 order 序");
    assert_eq!(&ordered[3..], ["外来乙", "外来甲"], "不在 order 内的排尾、按名字典序");
    // 无 order（旧空间）= 全部按名字典序（码点序：外 U+5916 < 库 U+5E93）
    space.library_order = Vec::new();
    assert_eq!(ordered_libraries(&space).unwrap(), vec!["外来乙", "外来甲", "库A", "库B", "库C"]);
  }

  /// reorder：order 与实际库集合不一致（多 / 少）拒绝；一致则落盘并返回新序
  #[test]
  fn reorder_libraries_validates_and_persists() {
    let dir = tempfile::tempdir().unwrap();
    let mut space = create(dir.path(), "空间", false).unwrap();
    for lib in ["库A", "库B"] {
      make_lib_dir(&space.root, lib);
    }
    // 少一个 / 多一个都报中文错
    assert!(reorder_libraries(&mut space, vec!["库A".into()]).is_err());
    assert!(reorder_libraries(&mut space, vec!["库A".into(), "库B".into(), "库C".into()]).is_err());
    assert_eq!(space.library_order, Vec::<String>::new(), "校验失败不应改动 order");

    let sorted = reorder_libraries(&mut space, vec!["库B".into(), "库A".into()]).unwrap();
    assert_eq!(sorted, vec!["库B", "库A"]);
    // 落盘 + 重开不丢
    let disk: SpaceFile = read_json(&space.root.join(SPACE_FILE)).unwrap();
    assert_eq!(disk.library_order, vec!["库B", "库A"]);
    assert_eq!(open(&space.root).unwrap().library_order, vec!["库B", "库A"]);
  }

  /// delete：库目录递归删除 + order 移除落盘 + 返回剩余列表（按 order 序）；assets 池不动
  #[test]
  fn delete_library_removes_dir_and_order_entry() {
    let dir = tempfile::tempdir().unwrap();
    let mut space = create(dir.path(), "空间", false).unwrap();
    for lib in ["库A", "库B", "库C"] {
      make_lib_dir(&space.root, lib);
    }
    space.library_order = vec!["库C".into(), "库A".into(), "库B".into()];

    let remaining = delete_library(&mut space, "库A", false).unwrap();
    assert_eq!(remaining, vec!["库C", "库B"]);
    assert!(!space.root.join("库A").exists(), "库目录应整体消失");
    assert_eq!(space.library_order, vec!["库C", "库B"]);
    let disk: SpaceFile = read_json(&space.root.join(SPACE_FILE)).unwrap();
    assert_eq!(disk.library_order, vec!["库C", "库B"], "order 移除应落盘");
    assert!(paths::assets_dir(&space.root).is_dir(), "素材实体保留在 assets 池，不随删库动");
    // 不存在的库 / 普通目录都报错
    assert!(matches!(delete_library(&mut space, "不存在", false), Err(DataError::NotALibrary)));
    fs::create_dir_all(space.root.join("普通目录")).unwrap();
    assert!(matches!(delete_library(&mut space, "普通目录", false), Err(DataError::NotALibrary)));
  }

  /// library_size（合并弹窗容量列）：引用资产（folders ∪ recycle）的 metadata.size 求和；
  /// 未被他库引用不扣减（共享资产各库都计）；metadata 缺失跳过；非库目录报错
  #[test]
  fn library_size_sums_referenced_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let space = create(dir.path(), "空间", false).unwrap();
    let assets = paths::assets_dir(&space.root);
    // 两个资产：被引用 100B、回收站引用 50B；未引用 999B 不计
    for (id, size) in [("AN-A", 100u64), ("AN-R", 50), ("AN-X", 999)] {
      let info = paths::info_dir(&assets, id);
      fs::create_dir_all(&info).unwrap();
      fs::write(
        info.join("metadata.json"),
        format!(
          r#"{{ "version": 1, "id": "{id}", "name": "{id}", "originName": "{id}.fla", "ext": "fla",
               "size": {size}, "importedAt": "2026-09-26T00:00:00+08:00", "modifiedAt": "2026-09-26T00:00:00+08:00" }}"#
        ),
      )
      .unwrap();
    }
    let lib_dir = space.root.join("库A");
    fs::create_dir_all(&lib_dir).unwrap();
    fs::write(
      lib_dir.join(FOLDERS_FILE),
      r#"{ "version": 1, "folders": [], "items": [ { "asset": "AN-A", "folder": "root", "name": "A.fla", "order": 0, "origin": true } ] }"#,
    )
    .unwrap();
    fs::write(
      lib_dir.join(RECYCLE_FILE),
      r#"{ "version": 1, "items": [ { "asset": "AN-R", "placements": [], "deletedAt": "2026-09-26T01:00:00+08:00" } ] }"#,
    )
    .unwrap();

    assert_eq!(library_size(&space.root, "库A").unwrap(), 150, "folders ∪ recycle 引用求和");
    assert!(matches!(library_size(&space.root, "不存在"), Err(DataError::NotALibrary)));
  }
}
