//! 素材库结构：创建 / 校验 / 打开（PRD 01_储存空间、06-01）。
//! 库目录布局：folders/tags/pins/recycle/recent 五个数据文件 + settings.json（库级设置）
//! + backup/ cache/ vectors/ inbox/；素材实体在空间级 assets/ 池（{ID}.info 平铺，多库共池）。
//! 内部路径全部相对化，整个空间可整体搬迁、换盘符不影响使用（PRD 06-01.1）。

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::backup;
use crate::core::index::LibraryIndex;
use crate::core::json_io::{read_json, write_json, DataError};
use crate::core::migrate;
use crate::core::models::*;
use crate::core::paths;
use crate::core::space::{self, Space};

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
/// 库级设置（PRD 01_储存空间-02：autoImport 等；单一写点 = 本模块）
pub const SETTINGS_FILE: &str = "settings.json";

/// 库根数据文件（均为备份对象，PRD 06-04）
pub const ROOT_DATA_FILES: &[&str] =
  &[FOLDERS_FILE, TAGS_FILE, PINS_FILE, RECYCLE_FILE, RECENT_FILE, SETTINGS_FILE];

/// info 文件夹内的数据文件（备份对象；缩略图/波形图是缓存可重建，不备份；
/// frames.json = 动图解包的逐帧 delay 清单，2026-07-26 定案持久化）
const INFO_DATA_FILES: &[&str] = &["metadata.json", "annotation.json", "notes.json", "frames.json"];

/// 打开状态的素材库：数据 + 内存索引（运行期查询走内存，写操作先落盘再更新内存，PRD 06-03）
#[allow(dead_code)] // tags / space_root 字段随阶段 7 标签业务 / 后续空间能力启用
pub struct Library {
  /// 库目录自身（5 数据文件 + settings.json + backup/cache/vectors/inbox）
  pub root: PathBuf,
  /// 所属储存空间根
  pub space_root: PathBuf,
  /// 空间级统一素材池（assets/）：info 路径只经 core::paths 推导
  pub assets_dir: PathBuf,
  pub folders: FoldersFile,
  pub tags: TagsFile,
  pub pins: PinsFile,
  pub recycle: RecycleFile,
  pub recent: RecentFile,
  pub settings: LibrarySettingsFile,
  pub index: LibraryIndex,
  /// 已缓存系统格式图标的扩展名集（cache/thumbs/{ext}.webp，4.2）——视图层回退查内存集，免逐卡片 stat
  pub format_icons: HashSet<String>,
}

impl Library {
  /// 资源 ID → info 文件夹（core::paths 单点的便捷入口）
  pub fn info_dir(&self, id: &str) -> PathBuf {
    paths::info_dir(&self.assets_dir, id)
  }
}

/// 数据文件全量清单：库根数据文件 + assets/*.info/{metadata,annotation,notes,frames}.json（备份与迁移共用）
pub fn data_files(root: &Path, assets_dir: &Path) -> Result<Vec<PathBuf>, DataError> {
  let mut files: Vec<PathBuf> =
    ROOT_DATA_FILES.iter().map(|f| root.join(f)).filter(|p| p.exists()).collect();
  for info in info_dirs(assets_dir)? {
    for name in INFO_DATA_FILES {
      let p = info.join(name);
      if p.exists() {
        files.push(p);
      }
    }
  }
  Ok(files)
}

/// 列出 assets/ 下全部 {ID}.info 文件夹（打开 / 备份 / 迁移等冷路径用；
/// 热路径禁止枚举 assets/，一律按 ID 经 core::paths 推导——PRD assets 池访问纪律）
fn info_dirs(assets_dir: &Path) -> Result<Vec<PathBuf>, DataError> {
  if !assets_dir.is_dir() {
    return Ok(Vec::new());
  }
  Ok(fs::read_dir(assets_dir)?
    .filter_map(|e| e.ok())
    .map(|e| e.path())
    .filter(|p| p.is_dir() && p.extension().and_then(|s| s.to_str()) == Some("info"))
    .collect())
}

/// 打开准入校验（不是什么文件夹都能打开，必须是我们的资源库结构）。
/// 判据：folders.json 可解析且版本合法；库根自含 asstes/ 的旧版结构明确拒绝（不迁移，2026-08-12 定案）。
pub fn validate(path: &Path) -> Result<(), DataError> {
  if path.join("asstes").is_dir() {
    return Err(DataError::LegacyLibrary);
  }
  let folders_path = path.join(FOLDERS_FILE);
  if !folders_path.is_file() {
    return Err(DataError::NotALibrary);
  }
  read_json::<serde_json::Value>(&folders_path).map(|_| ())
}

/// 资源库名称合法性（Windows 文件夹命名规则；前端表单同规则，命令接口统一把关）。
/// pub(crate)：6.1 新建文件夹 / 重命名 Placement 复用同一套名称规则
pub(crate) fn validate_library_name(name: &str) -> Result<(), DataError> {
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

/// 在储存空间内创建资源库（库目录名即资源库名）。要求空间根下无同名冲突；
/// 已是资源库则提示改用打开。auto_import 写库级 settings.json（实际行为 2.6 接）。
pub fn create(space: &Space, name: &str, auto_import: bool) -> Result<Library, DataError> {
  // 名称校验在 mkdir 之前，避免建出非法 / 半截目录
  validate_library_name(name)?;
  // 库目录与空间级目录同名会撞结构（assets/workspace），直接拒绝
  if [paths::ASSETS_DIR, space::WORKSPACE_DIR].contains(&name.to_ascii_lowercase().as_str()) {
    return Err(DataError::NameInvalid("该名称为储存空间保留名，请换一个"));
  }
  let path = space.root.join(name);
  fs::create_dir_all(&path)?;
  if validate(&path).is_ok() {
    return Err(DataError::AlreadyLibrary);
  }
  if fs::read_dir(&path)?.next().is_some() {
    return Err(DataError::NotEmpty);
  }
  for dir in [BACKUP_DIR, CACHE_DIR, VECTORS_DIR, INBOX_DIR] {
    fs::create_dir_all(path.join(dir))?;
  }
  write_json(&path.join(FOLDERS_FILE), &FoldersFile::new())?;
  write_json(&path.join(TAGS_FILE), &TagsFile::new())?;
  write_json(&path.join(PINS_FILE), &PinsFile::new())?;
  write_json(&path.join(RECYCLE_FILE), &RecycleFile::new())?;
  write_json(&path.join(RECENT_FILE), &RecentFile::new())?;
  write_json(&path.join(SETTINGS_FILE), &LibrarySettingsFile::new(auto_import))?;
  log::info!("素材库已创建：{}", path.display());
  open(&space.root, &path)
}

/// 打开素材库：校验结构 → 版本迁移检查（迁移前自动备份）→ 读全部数据 → 建内存索引 → 定期备份。
/// 缺失的子目录自动补建（自愈）；缺失的数据文件视为损坏，报错提示回滚（PRD 06-04）。
/// 旧版结构（空间缺 space.json / 库根自含 asstes/）不迁移，明确报错引导新建（2026-08-12 定案）。
pub fn open(space_root: &Path, path: &Path) -> Result<Library, DataError> {
  if space::validate(space_root).is_err() {
    return Err(DataError::LegacyLibrary);
  }
  validate(path)?;
  let assets_dir = paths::assets_dir(space_root);
  fs::create_dir_all(&assets_dir)?; // assets/ 缺失自愈补建（space::open 已补，此处双保险）
  for dir in [BACKUP_DIR, CACHE_DIR, VECTORS_DIR, INBOX_DIR] {
    fs::create_dir_all(path.join(dir))?;
  }
  // 会话级缓存（2026-07-25 用户定）：tmp（冲突弹窗预览）/ previews（PSD 高清层）/ frames（序列帧+动图帧条小图）
  // / waveforms（按需重采样波形图，2026-07-26）/ note-thumbs（详情面板笔记悬停帧画面，5.2）
  // 打开库时清空上一轮遗留——点开过的素材缓存不留到下一会话，防容量膨胀；cache/thumbs 格式图标按格式跨会话保留。
  // 动图逐帧高清图持久化在 info/frames/（2026-07-26 用户定案），不进 cache 不清空
  for sub in ["tmp", "previews", "frames", "waveforms", "note-thumbs"] {
    let dir = path.join(CACHE_DIR).join(sub);
    if dir.is_dir() {
      if let Err(e) = fs::remove_dir_all(&dir) {
        log::warn!("会话级缓存目录清理失败（{sub}）：{e}");
      }
    }
  }
  migrate::migrate_library_if_needed(path, &assets_dir)?;
  let mut folders: FoldersFile = read_json(&path.join(FOLDERS_FILE))?;
  let tags: TagsFile = read_json(&path.join(TAGS_FILE))?;
  let pins: PinsFile = read_json(&path.join(PINS_FILE))?;
  let recycle: RecycleFile = read_json(&path.join(RECYCLE_FILE))?;
  let recent: RecentFile = read_json(&path.join(RECENT_FILE))?;
  let settings = load_settings(path)?;
  let assets = load_assets(&assets_dir)?;
  // 打开自检（2026-07-25 用户反馈：手动删 assets/{ID}.info 后数量成永久错误）：
  // 剔除指向缺失 info 的 Placement（内存与磁盘同清，数量统计随索引重建归零）。
  // 落盘前先强制备份一份含脏数据的快照；备份失败则不写盘（只内存清理，防误删无可回滚）。
  let removed = prune_missing_placements(&mut folders, &assets);
  if removed > 0 {
    log::warn!("打开自检剔除 {removed} 条指向缺失 info 的 Placement");
    match backup::backup_library(path, &assets_dir) {
      Ok(_) => write_json(&path.join(FOLDERS_FILE), &folders)?,
      Err(e) => log::error!("自检清理前备份失败，folders.json 保持原样（仅本会话内存清理）：{e}"),
    }
  }
  let index = LibraryIndex::build(&folders, assets);
  backup::backup_if_stale(path, &assets_dir);
  Ok(Library {
    root: path.to_path_buf(),
    space_root: space_root.to_path_buf(),
    assets_dir,
    folders,
    tags,
    pins,
    recycle,
    recent,
    settings,
    index,
    format_icons: scan_format_icons(path),
  })
}

/// 读库级 settings.json；文件缺失按默认值自愈补写（新结构 create 必写，缺失只可能来自手动删除）
fn load_settings(root: &Path) -> Result<LibrarySettingsFile, DataError> {
  let path = root.join(SETTINGS_FILE);
  if path.is_file() {
    return read_json(&path);
  }
  log::warn!("库级 settings.json 缺失，按默认值自愈补写：{}", root.display());
  let settings = LibrarySettingsFile::default();
  write_json(&path, &settings)?;
  Ok(settings)
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

/// 扫描 assets/*.info/metadata.json 读入全部资源元数据；任一损坏即报错（提示回滚，不吞错误）
fn load_assets(assets_dir: &Path) -> Result<Vec<AssetMeta>, DataError> {
  let mut assets = Vec::new();
  for info in info_dirs(assets_dir)? {
    let meta_path = info.join("metadata.json");
    if meta_path.is_file() {
      assets.push(read_json(&meta_path)?);
    }
  }
  Ok(assets)
}

/// 打开自检：剔除 folders.items 中指向缺失资源（info 已不在 / metadata 未载入）的 Placement，
/// 返回剔除条数。只动 Placement，不动文件夹节点与资源数据
fn prune_missing_placements(folders: &mut FoldersFile, assets: &[AssetMeta]) -> usize {
  let ids: HashSet<&str> = assets.iter().map(|a| a.id.as_str()).collect();
  let before = folders.items.len();
  folders.items.retain(|p| ids.contains(p.asset.as_str()));
  before - folders.items.len()
}

/// folders.json 单一写点（01_开发规范 第 1 条）：全量保存文件夹树 + 全部 Placement。
pub fn save_folders(lib: &Library) -> Result<(), DataError> {
  write_json(&lib.root.join(FOLDERS_FILE), &lib.folders)
}

/// recycle.json 单一写点（01_开发规范 第 1 条）：全量保存回收站记录（6.1 删除落地启用）。
pub fn save_recycle(lib: &Library) -> Result<(), DataError> {
  write_json(&lib.root.join(RECYCLE_FILE), &lib.recycle)
}

/// recent.json 单一写点（01_开发规范 第 1 条）：全量保存最近使用记录（6.1 第二轮启用）。
pub fn save_recent(lib: &Library) -> Result<(), DataError> {
  write_json(&lib.root.join(RECENT_FILE), &lib.recent)
}

/// pins.json 单一写点（01_开发规范 第 1 条）：全量保存快捷入口
/// （pins 功能未启用；delete_recycle 彻底删除时引用清理用）。
pub fn save_pins(lib: &Library) -> Result<(), DataError> {
  write_json(&lib.root.join(PINS_FILE), &lib.pins)
}

/// settings.json 单一写点（01_开发规范 第 1 条）：全量保存库级设置（autoImport 等）
#[allow(dead_code)] // 设置项命令随自动导入行为接入（2.6）启用
pub fn save_settings(lib: &Library) -> Result<(), DataError> {
  write_json(&lib.root.join(SETTINGS_FILE), &lib.settings)
}

/// 撤销 / 重做回滚后重载：盘上是事实来源——重读全部数据文件 + 重建内存索引（6.0）。
/// 不触发备份 / 迁移 / 缓存清理（与 open 区分：回滚只是把内存同步回盘上现状）。
pub fn reload(lib: &mut Library) -> Result<(), DataError> {
  let root = lib.root.clone();
  lib.folders = read_json(&root.join(FOLDERS_FILE))?;
  lib.tags = read_json(&root.join(TAGS_FILE))?;
  lib.pins = read_json(&root.join(PINS_FILE))?;
  lib.recycle = read_json(&root.join(RECYCLE_FILE))?;
  lib.recent = read_json(&root.join(RECENT_FILE))?;
  lib.settings = load_settings(&root)?;
  let assets = load_assets(&lib.assets_dir)?;
  lib.index = LibraryIndex::build(&lib.folders, assets);
  Ok(())
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

/// 测试辅助：在 base 下建测试空间（测试空间/）+ 测试库（测试空间/测试库），返回打开状态的库
#[cfg(test)]
pub(crate) fn create_test(base: &Path) -> Library {
  let space = space::create(base, "测试空间").unwrap();
  create(&space, "测试库", false).unwrap()
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 建测试空间并返回（空间, 空间根路径）
  fn make_space(dir: &Path) -> (Space, PathBuf) {
    let space = space::create(dir, "空间").unwrap();
    let root = space.root.clone();
    (space, root)
  }

  #[test]
  fn create_initializes_full_structure() {
    let dir = tempfile::tempdir().unwrap();
    let (space, space_root) = make_space(dir.path());
    let lib = create(&space, "我的素材库", false).unwrap();
    let root = space_root.join("我的素材库");

    // 库目录 = 纯数据 + backup/cache/vectors/inbox（asstes 已上移空间级 assets/）
    for d in [BACKUP_DIR, CACHE_DIR, VECTORS_DIR, INBOX_DIR] {
      assert!(root.join(d).is_dir(), "缺目录 {d}");
    }
    assert!(!root.join("asstes").exists(), "库目录不应再有 asstes/");
    for f in ROOT_DATA_FILES {
      assert!(root.join(f).is_file(), "缺数据文件 {f}");
    }
    assert_eq!(lib.root, root);
    assert_eq!(lib.space_root, space_root.clone());
    assert_eq!(lib.assets_dir, paths::assets_dir(&space_root));
    assert_eq!(lib.folders.version, DATA_VERSION);
    assert!(lib.index.assets.is_empty());
  }

  #[test]
  fn create_writes_settings_with_auto_import() {
    let dir = tempfile::tempdir().unwrap();
    let (space, space_root) = make_space(dir.path());
    let lib = create(&space, "库", true).unwrap();
    assert!(lib.settings.auto_import);
    let disk: LibrarySettingsFile = read_json(&space_root.join("库").join(SETTINGS_FILE)).unwrap();
    assert_eq!(disk, LibrarySettingsFile::new(true));
    // 单写点改写后落盘
    let mut lib = lib;
    lib.settings.auto_import = false;
    save_settings(&lib).unwrap();
    let disk: LibrarySettingsFile = read_json(&space_root.join("库").join(SETTINGS_FILE)).unwrap();
    assert!(!disk.auto_import);
  }

  #[test]
  fn open_heals_missing_settings_with_default() {
    let dir = tempfile::tempdir().unwrap();
    let (space, space_root) = make_space(dir.path());
    create(&space, "库", true).unwrap();
    fs::remove_file(space_root.join("库").join(SETTINGS_FILE)).unwrap();
    let lib = open(&space_root, &space_root.join("库")).unwrap();
    assert!(!lib.settings.auto_import, "缺失的 settings.json 按默认值自愈");
    assert!(space_root.join("库").join(SETTINGS_FILE).is_file(), "自愈补写落盘");
  }

  #[test]
  fn create_rejects_non_empty_folder() {
    let dir = tempfile::tempdir().unwrap();
    let (space, space_root) = make_space(dir.path());
    let lib_dir = space_root.join("库");
    fs::create_dir_all(&lib_dir).unwrap();
    fs::write(lib_dir.join("随便一个文件.txt"), "hi").unwrap();
    assert!(matches!(create(&space, "库", false), Err(DataError::NotEmpty)));
  }

  #[test]
  fn create_rejects_invalid_names() {
    let dir = tempfile::tempdir().unwrap();
    let (space, space_root) = make_space(dir.path());
    // 特殊符号 / 结尾点 / 系统保留名（Windows 文件夹命名规则）/ 空间级保留目录名
    for name in ["bad:name", "bad|name", "lib.", "lib ", "CON", "com1", "Lpt9", "assets", "workspace"] {
      let result = create(&space, name, false);
      assert!(matches!(result, Err(DataError::NameInvalid(_))), "名称 {name} 应被拒绝");
      assert!(!space_root.join(name).join(FOLDERS_FILE).exists(), "名称 {name} 不应建出库");
    }
    // 合法中文名可用
    assert!(create(&space, "我的素材", false).is_ok());
  }

  #[test]
  fn create_rejects_existing_library() {
    let dir = tempfile::tempdir().unwrap();
    let (space, _) = make_space(dir.path());
    create(&space, "库", false).unwrap();
    assert!(matches!(create(&space, "库", false), Err(DataError::AlreadyLibrary)));
  }

  #[test]
  fn validate_rejects_plain_and_legacy_folders() {
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(validate(dir.path()), Err(DataError::NotALibrary)));
    // 旧版结构（库根自含 asstes/）：明确报旧结构，不迁移（2026-08-12 定案）
    fs::write(dir.path().join(FOLDERS_FILE), r#"{ "version": 1, "folders": [], "items": [] }"#).unwrap();
    fs::create_dir_all(dir.path().join("asstes")).unwrap();
    assert!(matches!(validate(dir.path()), Err(DataError::LegacyLibrary)));
  }

  #[test]
  fn open_rejects_legacy_structure() {
    let dir = tempfile::tempdir().unwrap();
    // 情形一：库根自含 asstes/（空间本身合法）
    let (_space, space_root) = make_space(dir.path());
    let lib_dir = space_root.join("旧库");
    fs::create_dir_all(lib_dir.join("asstes")).unwrap();
    fs::write(lib_dir.join(FOLDERS_FILE), r#"{ "version": 1, "folders": [], "items": [] }"#).unwrap();
    let result = open(&space_root, &lib_dir);
    assert!(matches!(&result, Err(DataError::LegacyLibrary)));
    assert_eq!(result.err().unwrap().to_string(), "旧版库结构，请新建资源库");

    // 情形二：空间缺 space.json（旧扁平库整体当空间打开）
    let dir2 = tempfile::tempdir().unwrap();
    let legacy = dir2.path().join("旧扁平库");
    fs::create_dir_all(legacy.join("asstes")).unwrap();
    fs::write(legacy.join(FOLDERS_FILE), r#"{ "version": 1, "folders": [], "items": [] }"#).unwrap();
    assert!(matches!(open(&legacy, &legacy), Err(DataError::LegacyLibrary)));
  }

  #[test]
  fn open_loads_assets_into_index() {
    let dir = tempfile::tempdir().unwrap();
    let lib = create_test(dir.path());
    let root = lib.root.clone();

    // 造一个资源 info 文件夹（空间级 assets 池）+ 一条 Placement
    let info = lib.info_dir("AN2607211305-X7K2Q");
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

    let lib = open(&lib.space_root, &root).unwrap();
    assert_eq!(lib.index.assets.len(), 1);
    assert_eq!(lib.index.placements_of("AN2607211305-X7K2Q").len(), 1);
    assert_eq!(lib.index.placements_by_folder["F01"].len(), 1);
    // 首次打开触发一次定时备份
    assert!(root.join(BACKUP_DIR).read_dir().unwrap().next().is_some());
  }

  /// 打开自检（2026-07-25 用户反馈）：手动删除 assets/{ID}.info 后，指向它的 Placement
  /// 必须剔除（内存计数即时正确，并落盘防永久错误数据）；落盘前强制备份含脏数据快照
  #[test]
  fn open_prunes_placements_pointing_to_missing_info() {
    let dir = tempfile::tempdir().unwrap();
    let lib = create_test(dir.path());
    let root = lib.root.clone();

    // 真实资源 AN-REAL 的 info 存在；AN-GONE 只有 Placement，info 已被用户手动删掉
    let info = lib.info_dir("AN-REAL");
    fs::create_dir_all(&info).unwrap();
    fs::write(
      info.join("metadata.json"),
      r#"{
        "version": 1, "id": "AN-REAL", "name": "在", "originName": "在.fla",
        "ext": "fla", "size": 1, "importedAt": "2026-07-25T00:00:00+08:00",
        "modifiedAt": "2026-07-25T00:00:00+08:00"
      }"#,
    )
    .unwrap();
    let folders: FoldersFile = serde_json::from_str(
      r#"{
        "version": 1,
        "folders": [ { "id": "F01", "name": "人物", "parent": null, "order": 0 } ],
        "items": [
          { "asset": "AN-REAL", "folder": "F01", "name": "在.fla", "order": 0, "origin": true },
          { "asset": "AN-GONE", "folder": "F01", "name": "没了.fla", "order": 1, "origin": true }
        ]
      }"#,
    )
    .unwrap();
    write_json(&root.join(FOLDERS_FILE), &folders).unwrap();

    let lib = open(&lib.space_root, &root).unwrap();
    assert_eq!(lib.index.placements_by_folder["F01"].len(), 1, "缺失 info 的 Placement 不进索引");
    assert_eq!(lib.index.placements_by_folder["F01"][0].asset, "AN-REAL");
    let persisted: FoldersFile = read_json(&root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(persisted.items.len(), 1, "folders.json 应落盘清理，防永久错误计数");
    assert_eq!(persisted.items[0].asset, "AN-REAL");
  }

  #[test]
  fn open_reports_corrupt_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let lib = create_test(dir.path());
    let info = lib.info_dir("AN2607211305-X7K2Q");
    fs::create_dir_all(&info).unwrap();
    fs::write(info.join("metadata.json"), "{ broken").unwrap();
    assert!(matches!(open(&lib.space_root, &lib.root), Err(DataError::Corrupt(_))));
  }

  #[test]
  fn open_clears_session_caches() {
    let dir = tempfile::tempdir().unwrap();
    let lib = create_test(dir.path());
    let root = lib.root.clone();
    // 会话级缓存（2026-07-25 用户定）：tmp / previews / frames / waveforms / note-thumbs 打开库时整目录清空
    for sub in ["tmp", "previews", "frames", "waveforms", "note-thumbs"] {
      let d = root.join("cache").join(sub);
      fs::create_dir_all(&d).unwrap();
      fs::write(d.join("a.webp"), b"x").unwrap();
    }
    // 格式图标缓存跨会话保留
    let thumbs = root.join("cache/thumbs");
    fs::create_dir_all(&thumbs).unwrap();
    fs::write(thumbs.join("fla.webp"), b"x").unwrap();
    open(&lib.space_root, &root).unwrap();
    for sub in ["tmp", "previews", "frames", "waveforms", "note-thumbs"] {
      assert!(!root.join("cache").join(sub).exists(), "打开库应清空 cache/{sub}");
    }
    assert!(thumbs.join("fla.webp").exists(), "格式图标缓存应保留");
  }

  #[test]
  fn save_folders_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let mut lib = create_test(dir.path());
    lib.folders.folders.push(Folder { id: "F01".into(), name: "人物".into(), parent: None, order: 0 });
    save_folders(&lib).unwrap();
    let back: FoldersFile = read_json(&lib.root.join(FOLDERS_FILE)).unwrap();
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
