//! 素材库结构：创建 / 校验 / 打开（PRD 01_储存空间、06-01）。
//! 库目录布局：folders/tags/pins/recycle/recent 五个数据文件 + backup/ cache/ vectors/；
//! 素材实体在空间级 assets/ 池（{ID}.info 平铺，多库共池）；收件箱 inbox/ 为空间级共用（2026-08-12 用户定）。
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
use crate::core::relations;
use crate::core::space::{self, Space};

pub const BACKUP_DIR: &str = "backup";
pub const CACHE_DIR: &str = "cache";
pub const VECTORS_DIR: &str = "vectors";
/// 临时文件总目录（8.1，2026-09-01 用户定：temp 与 cache 分离——temp = 中转产物用完即删，
/// cache = 可重建缓存；各功能临时文件在 temp/ 下各有归属子目录，如 CEP 导出中转 temp/cep-out）
pub const TEMP_DIR: &str = "temp";
pub const FOLDERS_FILE: &str = "folders.json";
pub const TAGS_FILE: &str = "tags.json";
pub const PINS_FILE: &str = "pins.json";
pub const RECYCLE_FILE: &str = "recycle.json";
pub const RECENT_FILE: &str = "recent.json";
pub const ARCHIVES_FILE: &str = "archives.json";
/// 档案封面目录名（cache/covers/，6.1 独立存的烘培封面图，不入库为资产；
/// 2026-09-08 用户定：挪入 cache 归可再生缓存 + 覆盖式烘培一档案一文件，不累积历史封面；
/// 惰性创建，无封面不建目录）
pub const COVERS_DIR: &str = "covers";
/// 封面撤销暂存目录名（cache/covers-trash/）：覆盖式烘培的旧封面改名暂存支撑撤销/重做
///（同 seq-trash 口径，会话级——开库清空）
pub const COVERS_TRASH_DIR: &str = "covers-trash";

/// 档案封面目录（cache/covers/）
pub fn covers_dir(root: &Path) -> PathBuf {
  root.join(CACHE_DIR).join(COVERS_DIR)
}

/// 封面撤销暂存目录（cache/covers-trash/）
pub fn covers_trash_dir(root: &Path) -> PathBuf {
  root.join(CACHE_DIR).join(COVERS_TRASH_DIR)
}

/// 库根数据文件（均为备份对象，PRD 06-04）
pub const ROOT_DATA_FILES: &[&str] = &[FOLDERS_FILE, TAGS_FILE, PINS_FILE, RECYCLE_FILE, RECENT_FILE, ARCHIVES_FILE];

/// info 文件夹内的数据文件（备份对象；缩略图/波形图是缓存可重建，不备份；
/// frames.json 已随动图解包废除移除（2026-08-20，播放走自研解码帧流，逐帧 delay 运行时读取源文件）
const INFO_DATA_FILES: &[&str] = &["metadata.json", "annotation.json", "notes.json"];

/// 无主资产通知（2026-09-02 开库自检产物）：孤儿 info 实况清单——打开库时经
/// 「library-orphan-infos」事件推前端弹警告窗（完整管理入口后续补稿）
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrphanNotice {
  pub id: String,
  pub name: String,
  /// info 目录完整路径（后续「打开资源管理器查看」入口用）
  pub path: String,
}

/// 打开状态的素材库：数据 + 内存索引（运行期查询走内存，写操作先落盘再更新内存，PRD 06-03）
#[allow(dead_code)] // tags / space_root / orphan_infos 字段随阶段 7 标签业务 / 后续空间能力启用
pub struct Library {
  /// 库目录自身（5 数据文件 + backup/cache/vectors）
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
  /// 档案数据（2026-08-15 档案子系统一轮，PRD 03_数据链接 06）：旧库无 archives.json 按空表加载
  pub archives: ArchivesFile,
  pub index: LibraryIndex,
  /// 已缓存系统格式图标的扩展名集（cache/thumbs/{ext}.webp，4.2）——视图层回退查内存集，免逐卡片 stat
  pub format_icons: HashSet<String>,
  /// 开库自检无主资产实况（2026-09-02）：本次 open 扫出的孤儿清单（空壳已自动清理不在此列）
  pub orphan_infos: Vec<OrphanNotice>,
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
/// 已是资源库则提示改用打开。创建成功把新库名 append 进 library_order（新库排尾，PRD 3.2），
/// 经 save_space 单写点落盘
pub fn create(space: &mut Space, name: &str) -> Result<Library, DataError> {
  // 名称校验在 mkdir 之前，避免建出非法 / 半截目录
  validate_library_name(name)?;
  // 库目录与空间级目录同名会撞结构（assets/workspace/inbox），直接拒绝
  if [paths::ASSETS_DIR, space::WORKSPACE_DIR, space::INBOX_DIR].contains(&name.to_ascii_lowercase().as_str()) {
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
  for dir in [BACKUP_DIR, CACHE_DIR, VECTORS_DIR, TEMP_DIR] {
    fs::create_dir_all(path.join(dir))?;
  }
  write_json(&path.join(FOLDERS_FILE), &FoldersFile::new())?;
  write_json(&path.join(TAGS_FILE), &TagsFile::new())?;
  write_json(&path.join(PINS_FILE), &PinsFile::new())?;
  write_json(&path.join(RECYCLE_FILE), &RecycleFile::new())?;
  write_json(&path.join(RECENT_FILE), &RecentFile::new())?;
  write_json(&path.join(ARCHIVES_FILE), &ArchivesFile::new())?;
  // 新库排尾：登记进 library_order 并落盘（save_space 单写点；失败不吞，库已建出会按外来目录排尾显示）
  space.library_order.push(name.to_string());
  space::save_space(space)?;
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
  for dir in [BACKUP_DIR, CACHE_DIR, VECTORS_DIR, TEMP_DIR] {
    fs::create_dir_all(path.join(dir))?;
  }
  // 会话级缓存（2026-07-25 用户定）：tmp（冲突弹窗预览）/ previews（PSD 高清层）/ frames（序列帧+动图帧条小图）
  // / waveforms（按需重采样波形图，2026-07-26）/ note-thumbs（详情面板笔记悬停帧画面，5.2）
  // / seq-trash（序列帧删帧暂存——支撑撤销，2026-08-21，5.2 包内视图）
  // / covers-trash（档案覆盖式烘培旧封面暂存——支撑撤销，2026-09-08）
  // 打开库时清空上一轮遗留——点开过的素材缓存不留到下一会话，防容量膨胀；cache/thumbs 格式图标按格式跨会话保留。
  // seq-trash / covers-trash 清空 = 暂存物会话结束真弃（undo 栈同为会话级，重启后无从撤销）；
  // 序列帧帧图持久化在 info/frames/，不进 cache 不清空；cache/covers 跨会话保留（开库自检清理无引用封面）；
  // 音视频转码产物在 cache/media（2026-09-13 用户实机定：原放 tmp 随开库清空 → 每次启动首次播放都重转、
  // CPU 长时间满载——stamp 记源"大小:mtime秒"已能保证源变化重转，迁出 tmp 跨会话保留，不在本清空名单）
  for sub in ["tmp", "previews", "frames", "waveforms", "note-thumbs", "seq-trash", "covers-trash"] {
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
  // archives.json 是 3.2 新增文件：旧库无此文件按空表加载（不算损坏，同 PRD 06-04 口径的例外）
  let mut archives = read_archives(&path.join(ARCHIVES_FILE))?;
  let mut assets = load_assets(&assets_dir)?;
  // 打开自检（2026-07-25 用户反馈：手动删 assets/{ID}.info 后数量成永久错误）：
  // 剔除指向缺失 info 的 Placement（内存与磁盘同清，数量统计随索引重建归零）。
  // 落盘前先强制备份一份含脏数据的快照；备份失败则不写盘（只内存清理，防误删无可回滚）。
  let removed = prune_missing_placements(&mut folders, &assets);
  if removed > 0 {
    log::warn!("打开自检剔除 {removed} 条指向缺失 info 的 Placement");
  }
  // 无 origin 扶正（2026-09-08 副本 origin 标记修复存量自愈）：有活位置无 origin 且回收站
  // 无其 origin 快照的资产，首条活位置置 origin（历史副本误标链接语义致回收站漏物理删）
  let origins_fixed = fix_missing_origins(&mut folders, &recycle);
  if origins_fixed > 0 {
    log::info!("打开自检扶正 {origins_fixed} 个无 origin 资产（首条位置置原始标记）");
  }
  // 无主 info 扫描（2026-09-03 资产污染修复配套）：空壳自动物理删、孤儿清单存实况字段
  //（命令层 emit 前端弹警告窗；2026-09-02 用户定：不自动删，删 / 复活由用户处置）。
  // 引用集口径（2026-09-26 修复）：空间级并集——各库 folders ∪ recycle 磁盘态 ∪ 当前库内存态
  //（自检修复未落盘的窗口期保守兜底）；assets 池空间级共享，单库口径会把他库资产误判无主。
  // 并集构建失败（某库数据文件损坏）→ None 跳过孤儿判定只清空壳：宁可漏报不可误报
  let referenced = match space::referenced_assets_space(space_root) {
    Ok(mut set) => {
      set.extend(folders.items.iter().map(|p| p.asset.clone()));
      set.extend(recycle.items.iter().map(|i| i.asset.clone()));
      Some(set)
    }
    Err(e) => {
      log::error!("无主 info 扫描引用集构建失败（跳过孤儿判定）：{e}");
      None
    }
  };
  let orphans = scan_orphan_infos(&assets_dir, referenced.as_ref(), &assets);
  if !orphans.is_empty() {
    let detail = orphans.iter().map(|o| format!("{}（{}）", o.id, o.name)).collect::<Vec<_>>().join("、");
    log::warn!("打开自检发现 {} 个无主资产（数据层无记录，info 残留磁盘）：{detail}", orphans.len());
  }
  // 区块表自检（2026-08-14 区块子系统）：blocks 缺默认区块 → 注入「目录」；
  // Folder.block 指向不存在的区块 → 置 None（存量自愈）
  let blocks_fixed = fix_blocks(&mut folders);
  if blocks_fixed {
    log::info!("打开自检修复区块表（注入默认区块 / 失效区块引用置空）");
  }
  // 自定义图标值迁移（2026-09-01，7.1 二十五轮）：02自定义 新 30 图标集——旧文件干
  // 就近映射（animal→dog 等 5 项），失效值回退默认 None（用户定）
  let icons_fixed = fix_custom_icons(&mut folders);
  if icons_fixed {
    log::info!("打开自检迁移自定义图标值（旧文件干就近映射 / 失效值回退默认）");
  }
  // 伴生缩略图引用自检（8.1，PRD 02-15）：thumb_asset_id 指向已不存在资产 → 剔除（失效引用）。
  // 反向（hidden 资产无引用）合法不动——换绑遗留由用户手动「解除缩略图」或删除
  let thumb_refs_fixed = fix_thumb_refs(&mut assets);
  if !thumb_refs_fixed.is_empty() {
    log::info!("打开自检剔除 {} 条失效伴生缩略图引用（thumb_asset_id）", thumb_refs_fixed.len());
  }
  // 提示词引用自检（2026-09-26 提示词轮，PRD 03_数据链接 02_文件）：prompt.asset_id 指向
  // 已不存在资产 → 引用置 None、快照保留降级为已扫描态（资产侧逐 metadata 落盘 + 档案侧随 archives）
  let prompt_refs_fixed = fix_prompt_refs(&mut assets);
  if !prompt_refs_fixed.is_empty() {
    log::info!("打开自检降级 {} 条失效提示词引用（prompt.assetId 置 None、快照保留）", prompt_refs_fixed.len());
  }
  let archive_prompt_fixed = fix_archive_prompt_refs(&mut archives, &assets);
  if archive_prompt_fixed {
    log::info!("打开自检降级档案侧失效提示词引用（快照保留）");
  }
  // 档案数据自检（2026-08-15 档案子系统）：剔除指向不存在 folder / folder 非档案形态的记录；
  // 同 folder_id 重复记录去重（2026-09-01 封面设置懒建 BUG 存量自愈）
  let archives_fixed = fix_archives(&mut archives, &folders);
  if archives_fixed {
    log::info!("打开自检修复 archives.json（剔除失效档案记录 / 重复记录去重）");
  }
  // 档案封面自检（2026-09-08 覆盖式烘培定案配套，纯缓存操作失败只记日志不阻断开库）：
  // ① 旧版库根 covers/ 迁入 cache/covers/；② 清理无 archives 记录引用的封面文件
  //（覆盖式后一档案一文件，历史过期封面不再累积——用户实锤"covers 里几十个过期封面"）
  {
    let (migrated, purged) = migrate_and_purge_covers(path, &archives);
    if migrated > 0 {
      log::info!("打开自检迁移旧版档案封面 {migrated} 个（库根 covers/ → cache/covers/）");
    }
    if purged > 0 {
      log::info!("打开自检清理过期档案封面 {purged} 个（无记录引用）");
    }
  }
  // 关系自检（6.2 选择模式一轮，PRD 03_数据链接 06）：空间级 relations.json——剔除 from/to
  // 端点指向本库已失效目标的关系（跨库端点无法校验，保留）。空间级文件不在库备份清单，
  // 剔除项均为失效引用，有改动直接原子写回（不随下方库文件备份门禁段）
  {
    let lib_name = path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    match relations::load_relations(space_root) {
      Ok(mut rels) => {
        if relations::prune_relations(&mut rels, &lib_name, &folders, &assets) {
          match relations::save_relations(space_root, &rels) {
            Ok(()) => log::info!("打开自检修复 relations.json（剔除本库失效端点的关系）"),
            Err(e) => log::error!("关系自检落盘失败（数据文件保持原样）：{e}"),
          }
        }
      }
      Err(e) => log::error!("关系自检读取失败（跳过）：{e}"),
    }
  }
  let folders_changed = removed > 0 || blocks_fixed || icons_fixed || origins_fixed > 0;
  let archives_changed = archives_fixed || archive_prompt_fixed;
  if folders_changed || archives_changed || !thumb_refs_fixed.is_empty() || !prompt_refs_fixed.is_empty() {
    match backup::backup_library(path, &assets_dir) {
      Ok(_) => {
        if folders_changed {
          write_json(&path.join(FOLDERS_FILE), &folders)?;
        }
        if archives_changed {
          write_json(&path.join(ARCHIVES_FILE), &archives)?;
        }
        // thumb_asset_id / prompt 自检落盘：逐资产 metadata.json（备份门禁同 folders——失败仅内存清理）；
        // 两清单并集去重（同一资产可能双命中）
        let meta_fixed: HashSet<&String> = thumb_refs_fixed.iter().chain(prompt_refs_fixed.iter()).collect();
        for id in meta_fixed {
          if let Some(a) = assets.iter().find(|a| &a.id == id) {
            let meta_path = paths::info_dir(&assets_dir, id).join("metadata.json");
            if let Err(e) = write_json(&meta_path, a) {
              log::error!("引用自检落盘失败（{id}）：{e}");
            }
          }
        }
      }
      Err(e) => log::error!("自检清理前备份失败，数据文件保持原样（仅本会话内存清理）：{e}"),
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
    archives,
    index,
    format_icons: scan_format_icons(path),
    orphan_infos: orphans,
  })
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

/// 打开自检（2026-09-08 副本 origin 标记修复配套存量自愈）：扶正「无 origin」资产——
/// 库内有活位置但无 origin、且回收站无其 origin 快照的资产（历史 duplicate 把副本唯一
/// Placement 误标 origin=false 的产物：回收站彻底清除按"快照含 origin = 本体在站"判定，
/// 副本被当纯位置记录只删记录、info 永久残留），首条活位置（items 序）置 origin = true。
/// 排除回收站有 origin 快照的 = "origin 在站、全链接存活"合法过渡态（PRD 01 规则 8）不动；
/// 一资产只扶正首条，保库内 origin 恒 ≤ 1（hidden / segment 经正常建档有 origin 不受影响）。
fn fix_missing_origins(folders: &mut FoldersFile, recycle: &RecycleFile) -> usize {
  let origin_in_recycle: HashSet<&str> = recycle
    .items
    .iter()
    .filter(|i| i.placements.iter().any(|p| p.origin))
    .map(|i| i.asset.as_str())
    .collect();
  let has_origin: HashSet<String> = folders.items.iter().filter(|p| p.origin).map(|p| p.asset.clone()).collect();
  let mut seen: HashSet<String> = HashSet::new();
  let mut fixed = 0usize;
  for p in &mut folders.items {
    if p.origin || has_origin.contains(p.asset.as_str()) || origin_in_recycle.contains(p.asset.as_str()) {
      continue;
    }
    if seen.insert(p.asset.clone()) {
      p.origin = true;
      fixed += 1;
    }
  }
  fixed
}

/// 无主 info 扫描（2026-09-03 资产污染修复配套，打开自检冷路径；2026-09-26 口径修复：
/// referenced = 空间级引用集——各库 folders ∪ recycle 并集 ∪ 当前库内存态；None = 引用集
/// 构建失败，跳过孤儿判定只清空壳，宁可漏报不误报——误报会诱导用户清掉他库资产）：
/// - 空壳（info 目录无 metadata.json 未载入）= 导入 / 删除半截残骸 → **自动物理删**
///   （100% 垃圾；无数据文件，不涉及备份门禁；删除失败仅记录，下轮开库再扫）；
/// - 孤儿（metadata 在，但空间级引用集无引用）= 异常路径残留 → **只返回清单
///   不自动删**（可能是用户数据唯一副本，删 / 复活由用户定），调用方负责 log + 存 Library
///   实况字段（命令层 emit 前端弹警告窗）。
///
/// 返回孤儿清单（测试断言用）
fn scan_orphan_infos(assets_dir: &Path, referenced: Option<&HashSet<String>>, assets: &[AssetMeta]) -> Vec<OrphanNotice> {
  let infos = match info_dirs(assets_dir) {
    Ok(v) => v,
    Err(e) => {
      log::error!("无主 info 扫描枚举失败（跳过）：{e}");
      return Vec::new();
    }
  };
  let loaded: HashSet<&str> = assets.iter().map(|a| a.id.as_str()).collect();
  let mut orphans = Vec::new();
  for info in infos {
    let Some(stem) = info.file_stem().map(|s| s.to_string_lossy().into_owned()) else { continue };
    if !loaded.contains(stem.as_str()) {
      match fs::remove_dir_all(&info) {
        Ok(()) => log::warn!("打开自检清理无主 info 空壳：{}", info.display()),
        Err(e) => log::error!("无主 info 空壳清理失败（{}）：{e}", info.display()),
      }
    } else if referenced.is_some_and(|r| !r.contains(stem.as_str())) {
      let name = assets.iter().find(|a| a.id == stem).map(|a| a.name.clone()).unwrap_or_default();
      orphans.push(OrphanNotice { id: stem, name, path: info.to_string_lossy().into_owned() });
    }
  }
  orphans
}

/// 打开自检（2026-08-14 区块子系统）：区块表自愈——默认区块「文件夹」缺失（含 blocks 为空）→
/// 注入队首；Folder.block 指向不存在的区块 → 置 None。返回是否有改动；
/// 2026-09-17 实机走查「目录语义改成文件夹」迁移：默认区块名仍为旧名「目录」（= 用户从未改名）→ 更名「文件夹」
fn fix_blocks(folders: &mut FoldersFile) -> bool {
  let mut changed = false;
  if !folders.blocks.iter().any(|b| b.id == DEFAULT_BLOCK_ID) {
    folders.blocks.insert(0, Block { id: DEFAULT_BLOCK_ID.to_string(), name: "文件夹".to_string(), icon: None, order: 0 });
    changed = true;
  }
  for b in &mut folders.blocks {
    if b.id == DEFAULT_BLOCK_ID && b.name == "目录" {
      b.name = "文件夹".to_string();
      changed = true;
    }
  }
  let ids: HashSet<&str> = folders.blocks.iter().map(|b| b.id.as_str()).collect();
  for f in &mut folders.folders {
    let dangling = matches!(&f.block, Some(b) if !ids.contains(b.as_str()));
    if dangling {
      f.block = None;
      changed = true;
    }
  }
  changed
}

/// 02自定义 新图标集（2026-09-01，7.1 二十五轮 30 个；同日走查 archive 移出 = 29——档案图标
/// 档案专属不可选，存量 Folder/Block.icon = "archive" 随本白名单剔除回退 None；
/// assets/icons/命名对照.md 表序，folder-closed/folder-open 为基础件不作自定义值）。
/// 与前端 src/lib/customIcons.ts 清单同源
const CUSTOM_ICONS_NEW: [&str; 29] = [
  "inbox", "window", "briefcase", "medal", "card", "slime", "layers", "image", "chat", "layout", "film", "house", "tag", "building", "car", "browser", "flame", "thumbs-up", "dog", "camera", "kanban", "vegetable", "emoji", "character", "palette", "shopping-basket", "music", "plane", "tableware",
];

/// 单个图标值迁移（fix_custom_icons 用）：新集内原样；旧 05_custom-icon 文件干就近映射；
/// 对不上的旧名与任意未知名 → None 回退默认（2026-09-01 用户定）
fn migrate_icon_value(icon: &mut Option<String>) -> bool {
  let Some(name) = icon.as_deref() else { return false };
  if CUSTOM_ICONS_NEW.contains(&name) {
    return false;
  }
  *icon = match name {
    "animal" => Some("dog".to_string()),
    "audio" => Some("music".to_string()),
    "video" => Some("film".to_string()),
    "vehicle" => Some("car".to_string()),
    "monster" => Some("slime".to_string()),
    _ => None,
  };
  true
}

/// 打开自检（2026-09-01，7.1 二十五轮）：自定义图标值迁移——02自定义 新 30 图标集接入后，
/// 存量 Folder.icon / Block.icon 旧文件干（05_custom-icon 集，svg 已随旧版归档）按
/// migrate_icon_value 就近映射改写、失效值回退默认（未迁移值渲染命中前端旧版回退层，
/// 回退层退役后即报"图标不存在"）。返回是否有改动
fn fix_custom_icons(folders: &mut FoldersFile) -> bool {
  let mut changed = false;
  for f in &mut folders.folders {
    changed |= migrate_icon_value(&mut f.icon);
  }
  for b in &mut folders.blocks {
    changed |= migrate_icon_value(&mut b.icon);
  }
  changed
}

/// 打开自检（8.1，PRD 02-15）：剔除失效伴生缩略图引用（thumb_asset_id 指向已不存在的资产）。
/// 只改内存返回改动 id 列表——逐资产 metadata.json 落盘在备份门禁段（备份失败则仅内存清理）。
/// 反向不查：hidden 资产无引用是合法态（换绑遗留，用户手动处理）
fn fix_thumb_refs(assets: &mut [AssetMeta]) -> Vec<String> {
  let ids: HashSet<String> = assets.iter().map(|a| a.id.clone()).collect();
  let mut changed = Vec::new();
  for a in assets.iter_mut() {
    if a.thumb_asset_id.as_deref().is_some_and(|t| !ids.contains(t)) {
      a.thumb_asset_id = None;
      changed.push(a.id.clone());
    }
  }
  changed
}

/// 打开自检（2026-09-26 提示词轮，PRD 03_数据链接 02_文件「源删除不断链」）：
/// prompt.asset_id 指向已不存在资产 → 引用置 None、**快照保留**降级为已扫描态
///（与 sync_after_purge 的 purge 降级同口径——脏引用唯一修复方向）。只改内存返回改动 id 列表，
/// 逐资产 metadata.json 落盘在备份门禁段（与 fix_thumb_refs 同径）
fn fix_prompt_refs(assets: &mut [AssetMeta]) -> Vec<String> {
  let ids: HashSet<String> = assets.iter().map(|a| a.id.clone()).collect();
  let mut changed = Vec::new();
  for a in assets.iter_mut() {
    if a.prompt.as_ref().and_then(|p| p.asset_id.as_deref()).is_some_and(|t| !ids.contains(t)) {
      if let Some(p) = a.prompt.as_mut() {
        p.asset_id = None;
      }
      changed.push(a.id.clone());
    }
  }
  changed
}

/// 打开自检（2026-09-26 提示词轮）：档案侧 prompt 引用同口径降级（引用资产不存在于
/// 资产集 → 置 None 留快照）。只改内存返回是否有改动，落盘随 archives.json 备份门禁段
fn fix_archive_prompt_refs(archives: &mut ArchivesFile, assets: &[AssetMeta]) -> bool {
  let ids: HashSet<&str> = assets.iter().map(|a| a.id.as_str()).collect();
  let mut changed = false;
  for a in archives.archives.iter_mut() {
    if a.prompt.as_ref().and_then(|p| p.asset_id.as_deref()).is_some_and(|t| !ids.contains(t)) {
      if let Some(p) = a.prompt.as_mut() {
        p.asset_id = None;
      }
      changed = true;
    }
  }
  changed
}

/// archives.json 读取（2026-08-15 档案子系统）：文件不存在 = 空表（3.2 新增文件，旧库无此文件不算损坏）
fn read_archives(path: &Path) -> Result<ArchivesFile, DataError> {
  if path.is_file() {
    read_json(path)
  } else {
    Ok(ArchivesFile::new())
  }
}

/// 打开自检（2026-08-15 档案子系统）：archives.json 记录自愈——指向不存在 folder 或
/// folder 非档案形态（kind != "archive"）的记录剔除；同 folder_id 重复记录去重保留首条
///（2026-09-01 前封面设置懒建条件写错会追加同 ID 空壳副本；更新永远落在 find 命中的首条，
/// 副本必为陈旧空壳，故首条即真数据）。返回是否有改动
fn fix_archives(archives: &mut ArchivesFile, folders: &FoldersFile) -> bool {
  let before = archives.archives.len();
  archives
    .archives
    .retain(|a| folders.folders.iter().any(|f| f.id == a.folder_id && f.kind.as_deref() == Some(ARCHIVE_KIND)));
  let mut seen = HashSet::new();
  archives.archives.retain(|a| seen.insert(a.folder_id.clone()));
  before != archives.archives.len()
}

/// 档案封面迁移与清理（打开自检，2026-09-08）：
/// ① 旧版库根 covers/ 内文件整体迁入 cache/covers/（封面改存缓存目录，归可再生缓存）；
/// ② cache/covers/ 内无 archives 记录引用的文件一律删除（覆盖式烘培一档案一文件，
/// 过期封面 = 换封面 / 删档案的遗留）。返回（迁移数, 清理数）；纯缓存操作，失败静默容错。
fn migrate_and_purge_covers(root: &Path, archives: &ArchivesFile) -> (usize, usize) {
  let mut migrated = 0usize;
  let legacy = root.join(COVERS_DIR);
  let dir = covers_dir(root);
  if legacy.is_dir() {
    if let Err(e) = fs::create_dir_all(&dir) {
      log::warn!("封面目录创建失败（跳过迁移）：{e}");
    } else if let Ok(rd) = fs::read_dir(&legacy) {
      for e in rd.flatten() {
        let from = e.path();
        if !from.is_file() {
          continue;
        }
        let to = dir.join(e.file_name());
        // 目标已存在 = 同名冲突（理论不发生：新位置文件名恒 {folder_id}.webp）：旧文件内容弃用
        if to.exists() {
          let _ = fs::remove_file(&from);
        } else if let Err(e) = fs::rename(&from, &to) {
          log::warn!("封面迁移失败（{}）：{e}", from.display());
          continue;
        }
        migrated += 1;
      }
    }
    // 只清空目录（子目录等残留不递归强删，留残不阻断）
    let _ = fs::remove_dir(&legacy);
  }
  let referenced: HashSet<&str> = archives.archives.iter().filter_map(|a| a.avatar.as_ref().map(|v| v.file.as_str())).collect();
  let mut purged = 0usize;
  if let Ok(rd) = fs::read_dir(&dir) {
    for e in rd.flatten() {
      let p = e.path();
      if p.is_file()
        && !p.file_name().and_then(|n| n.to_str()).is_some_and(|n| referenced.contains(n))
        && fs::remove_file(&p).is_ok()
      {
        purged += 1;
      }
    }
  }
  (migrated, purged)
}

/// folders.json 单一写点（01_开发规范 第 1 条）：全量保存文件夹树 + 全部 Placement。
pub fn save_folders(lib: &Library) -> Result<(), DataError> {
  write_json(&lib.root.join(FOLDERS_FILE), &lib.folders)
}

/// archives.json 单一写点（01_开发规范 第 1 条）：全量保存档案数据（3.2 一轮骨架）
pub fn save_archives(lib: &Library) -> Result<(), DataError> {
  write_json(&lib.root.join(ARCHIVES_FILE), &lib.archives)
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

/// 撤销 / 重做回滚后重载：盘上是事实来源——重读全部数据文件 + 重建内存索引（6.0）。
/// 不触发备份 / 迁移 / 缓存清理（与 open 区分：回滚只是把内存同步回盘上现状）。
pub fn reload(lib: &mut Library) -> Result<(), DataError> {
  let root = lib.root.clone();
  lib.folders = read_json(&root.join(FOLDERS_FILE))?;
  lib.tags = read_json(&root.join(TAGS_FILE))?;
  lib.pins = read_json(&root.join(PINS_FILE))?;
  lib.recycle = read_json(&root.join(RECYCLE_FILE))?;
  lib.recent = read_json(&root.join(RECENT_FILE))?;
  lib.archives = read_archives(&root.join(ARCHIVES_FILE))?;
  let assets = load_assets(&lib.assets_dir)?;
  lib.index = LibraryIndex::build(&lib.folders, assets);
  Ok(())
}

/// 同文件夹内显示名唯一化：无冲突原样返回；冲突时在扩展名前追加 _序号
/// （如 男人_1.fla、男人_2.fla；无扩展名则在末尾追加，如 参考_1。2026-09-23 序号格式自 - 改 _）
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
    let candidate = format!("{stem}_{n}{ext}");
    if !existing.contains(&candidate) {
      return candidate;
    }
    n += 1;
  }
}

/// 测试辅助：在 base 下建测试空间（测试空间/）+ 测试库（测试空间/测试库），返回打开状态的库
#[cfg(test)]
pub(crate) fn create_test(base: &Path) -> Library {
  let mut space = space::create(base, "测试空间", false).unwrap();
  create(&mut space, "测试库").unwrap()
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 建测试空间并返回（空间, 空间根路径）
  fn make_space(dir: &Path) -> (Space, PathBuf) {
    let space = space::create(dir, "空间", false).unwrap();
    let root = space.root.clone();
    (space, root)
  }

  #[test]
  fn open_prunes_dangling_thumb_asset_id() {
    // 8.1 伴生缩略图引用自检：thumb_asset_id 指向不存在资产 → 开库剔除 + 落盘一致
    let dir = tempfile::tempdir().unwrap();
    let lib = create_test(dir.path());
    let space_root = lib.space_root.clone();
    let root = lib.root.clone();
    // 造一个真实资产并把它的 thumb_asset_id 改为悬空引用落盘
    let src = dir.path().join("工程.fla");
    fs::write(&src, b"fake").unwrap();
    let pf = crate::services::import::PendingFile { path: src, chain: Vec::new() };
    let mut meta = crate::services::import::build_asset_with_id(&lib.assets_dir, &pf, "sha256:t", "AN-F", crate::services::import::Source::Copy, None).unwrap();
    meta.thumb_asset_id = Some("AN-GHOST".into());
    write_json(&lib.info_dir("AN-F").join("metadata.json"), &meta).unwrap();
    drop(lib);

    let lib2 = open(&space_root, &root).unwrap();
    assert_eq!(lib2.index.assets["AN-F"].thumb_asset_id, None, "开库剔除悬空引用");
    let disk: AssetMeta = read_json(&lib2.info_dir("AN-F").join("metadata.json")).unwrap();
    assert_eq!(disk.thumb_asset_id, None, "落盘一致");
  }

  #[test]
  fn open_degrades_dangling_prompt_refs() {
    // 2026-09-26 提示词轮：prompt.assetId 指向不存在资产 → 开库降级（引用置 None、
    // 快照保留为已扫描态）+ 落盘一致；档案侧同口径（archives.json 落盘）
    let dir = tempfile::tempdir().unwrap();
    let lib = create_test(dir.path());
    let space_root = lib.space_root.clone();
    let root = lib.root.clone();
    let src = dir.path().join("雪牧.png");
    fs::write(&src, b"fake").unwrap();
    let pf = crate::services::import::PendingFile { path: src, chain: Vec::new() };
    let mut meta = crate::services::import::build_asset_with_id(&lib.assets_dir, &pf, "sha256:t", "AN-A", crate::services::import::Source::Copy, None).unwrap();
    meta.prompt = Some(PromptBinding {
      asset_id: Some("AN-GHOST".into()),
      name: "提示词.md".into(),
      content: "快照内容保留".into(),
      updated_at: "2026-09-26T00:00:00+08:00".into(),
    });
    write_json(&lib.info_dir("AN-A").join("metadata.json"), &meta).unwrap();
    // 档案侧脏引用（档案记录落 archives.json）
    let mut lib = lib;
    lib.folders.folders.push(Folder {
      id: "F-A".into(),
      name: "档案".into(),
      parent: None,
      block: None,
      icon: None,
      kind: Some(ARCHIVE_KIND.into()),
      color: None,
      groups: None,
      order: 0,
    });
    write_json(&root.join(FOLDERS_FILE), &lib.folders).unwrap();
    let mut rec = ArchiveData::new("F-A");
    rec.prompt = meta.prompt.clone();
    lib.archives.archives.push(rec);
    write_json(&root.join(ARCHIVES_FILE), &lib.archives).unwrap();
    drop(lib);

    let lib2 = open(&space_root, &root).unwrap();
    let p = lib2.index.assets["AN-A"].prompt.clone().unwrap();
    assert_eq!(p.asset_id, None, "资产侧引用置 None 降级已扫描态");
    assert_eq!(p.content, "快照内容保留", "快照保留不断链");
    assert_eq!(p.name, "提示词.md", "显示名快照保留");
    let disk: AssetMeta = read_json(&lib2.info_dir("AN-A").join("metadata.json")).unwrap();
    assert_eq!(disk.prompt.as_ref().unwrap().asset_id, None, "资产侧落盘一致");
    let rec = lib2.archives.archives.iter().find(|a| a.folder_id == "F-A").unwrap();
    assert_eq!(rec.prompt.as_ref().unwrap().asset_id, None, "档案侧同降级");
    let disk_a: ArchivesFile = read_json(&root.join(ARCHIVES_FILE)).unwrap();
    assert_eq!(disk_a.archives[0].prompt.as_ref().unwrap().content, "快照内容保留", "档案侧落盘一致");
  }

  #[test]
  fn create_initializes_full_structure() {
    let dir = tempfile::tempdir().unwrap();
    let (mut space, space_root) = make_space(dir.path());
    let lib = create(&mut space, "我的素材库").unwrap();
    let root = space_root.join("我的素材库");

    // 库目录 = 纯数据 + backup/cache/vectors/temp（assets/ 与 inbox/ 均在空间级）
    for d in [BACKUP_DIR, CACHE_DIR, VECTORS_DIR, TEMP_DIR] {
      assert!(root.join(d).is_dir(), "缺目录 {d}");
    }
    assert!(!root.join("asstes").exists(), "库目录不应再有 asstes/");
    assert!(!root.join("inbox").exists(), "库目录不应再有 inbox/（空间级共用）");
    assert!(!root.join("settings.json").exists(), "库目录不应再有 settings.json（autoImport 已上移空间级）");
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
  fn create_rejects_non_empty_folder() {
    let dir = tempfile::tempdir().unwrap();
    let (mut space, space_root) = make_space(dir.path());
    let lib_dir = space_root.join("库");
    fs::create_dir_all(&lib_dir).unwrap();
    fs::write(lib_dir.join("随便一个文件.txt"), "hi").unwrap();
    assert!(matches!(create(&mut space, "库"), Err(DataError::NotEmpty)));
  }

  #[test]
  fn create_rejects_invalid_names() {
    let dir = tempfile::tempdir().unwrap();
    let (mut space, space_root) = make_space(dir.path());
    // 特殊符号 / 结尾点 / 系统保留名（Windows 文件夹命名规则）/ 空间级保留目录名
    for name in ["bad:name", "bad|name", "lib.", "lib ", "CON", "com1", "Lpt9", "assets", "workspace", "自动导入"] {
      let result = create(&mut space, name);
      assert!(matches!(result, Err(DataError::NameInvalid(_))), "名称 {name} 应被拒绝");
      assert!(!space_root.join(name).join(FOLDERS_FILE).exists(), "名称 {name} 不应建出库");
    }
    // 合法中文名可用
    assert!(create(&mut space, "我的素材").is_ok());
  }

  #[test]
  fn create_rejects_existing_library() {
    let dir = tempfile::tempdir().unwrap();
    let (mut space, _) = make_space(dir.path());
    create(&mut space, "库").unwrap();
    assert!(matches!(create(&mut space, "库"), Err(DataError::AlreadyLibrary)));
  }

  /// 新库排尾（PRD 3.2）：创建成功 append 进 library_order 并经 save_space 落盘
  #[test]
  fn create_appends_to_library_order() {
    let dir = tempfile::tempdir().unwrap();
    let (mut space, space_root) = make_space(dir.path());
    create(&mut space, "库A").unwrap();
    create(&mut space, "库B").unwrap();
    assert_eq!(space.library_order, vec!["库A", "库B"]);
    let disk: SpaceFile = read_json(&space_root.join(space::SPACE_FILE)).unwrap();
    assert_eq!(disk.library_order, vec!["库A", "库B"], "libraryOrder 应随创建落盘");
    // 创建失败（重名）不入 order
    assert!(create(&mut space, "库A").is_err());
    assert_eq!(space.library_order, vec!["库A", "库B"]);
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

  /// 无主 info 扫描（2026-09-03 资产污染修复配套；2026-09-26 口径修复为空间级引用集——
  /// 单库空间语义不变）：空壳（无 metadata.json）开库自动物理删；
  /// 孤儿（metadata 在，各库 folders ∪ recycle 均无引用）保留不动（log 清单由用户处置）；
  /// 正常资产与回收站引用资产不误伤
  #[test]
  fn open_scans_orphan_infos() {
    let dir = tempfile::tempdir().unwrap();
    let lib = create_test(dir.path());
    let root = lib.root.clone();
    let assets_dir = lib.assets_dir.clone();
    let seed = |id: &str, name: &str| {
      let info = lib.info_dir(id);
      fs::create_dir_all(&info).unwrap();
      fs::write(
        info.join("metadata.json"),
        format!(
          r#"{{ "version": 1, "id": "{id}", "name": "{name}", "originName": "{name}.fla",
               "ext": "fla", "size": 1, "importedAt": "2026-09-02T01:00:00+08:00",
               "modifiedAt": "2026-09-02T01:00:00+08:00" }}"#
        ),
      )
      .unwrap();
      fs::write(info.join(format!("{name}.fla")), b"fake").unwrap();
    };
    seed("AN-KEEP", "留"); // 有 Placement 的正常资产
    seed("AN-REC", "站"); // 回收站引用的资产
    seed("AN-ORPHAN", "孤"); // 孤儿：metadata 在，folders ∪ recycle 无引用
    fs::create_dir_all(lib.info_dir("AN-SHELL")).unwrap(); // 空壳：无 metadata.json
    let folders: FoldersFile = serde_json::from_str(
      r#"{
        "version": 1, "folders": [],
        "items": [ { "asset": "AN-KEEP", "folder": "root", "name": "留.fla", "order": 0, "origin": true } ]
      }"#,
    )
    .unwrap();
    write_json(&root.join(FOLDERS_FILE), &folders).unwrap();
    let recycle: RecycleFile = serde_json::from_str(
      r#"{
        "version": 1,
        "items": [ { "asset": "AN-REC", "deletedAt": "2026-09-02T02:00:00+08:00",
          "placements": [ { "asset": "AN-REC", "folder": "root", "name": "站.fla", "order": 0, "origin": true } ] } ]
      }"#,
    )
    .unwrap();
    write_json(&root.join(RECYCLE_FILE), &recycle).unwrap();

    let lib = open(&lib.space_root, &root).unwrap();
    assert!(!assets_dir.join("AN-SHELL.info").exists(), "空壳自动物理删");
    assert!(assets_dir.join("AN-ORPHAN.info").is_dir(), "孤儿不自动删（用户处置）");
    assert!(lib.index.assets.contains_key("AN-ORPHAN"), "孤儿仍载入索引（数据层实况）");
    assert!(lib.index.placements_of("AN-ORPHAN").is_empty(), "孤儿无 Placement 不可见");
    assert!(assets_dir.join("AN-KEEP.info").is_dir() && assets_dir.join("AN-REC.info").is_dir(), "正常与回收站引用资产不误伤");
    assert_eq!(lib.index.placements_of("AN-KEEP").len(), 1);
  }

  /// 无 origin 扶正自检（2026-09-08 副本 origin 标记修复存量自愈）：有活位置无 origin 且回收站
  /// 无其 origin 快照（历史 duplicate 副本产物）→ items 序首条置 origin 并落盘；
  /// 回收站有其 origin 快照（"origin 在站、全链接存活"过渡态，PRD 01 规则 8）→ 不动；
  /// 已有 origin 的资产不多置（库内 origin 恒 ≤ 1）
  #[test]
  fn open_fixes_missing_origins() {
    let dir = tempfile::tempdir().unwrap();
    let lib = create_test(dir.path());
    let root = lib.root.clone();
    let seed = |id: &str, name: &str| {
      let info = lib.info_dir(id);
      fs::create_dir_all(&info).unwrap();
      fs::write(
        info.join("metadata.json"),
        format!(
          r#"{{ "version": 1, "id": "{id}", "name": "{name}", "originName": "{name}.fla",
               "ext": "fla", "size": 1, "importedAt": "2026-09-08T01:00:00+08:00",
               "modifiedAt": "2026-09-08T01:00:00+08:00" }}"#
        ),
      )
      .unwrap();
      fs::write(info.join(format!("{name}.fla")), b"fake").unwrap();
    };
    seed("AN-DUP", "副"); // 存量副本态：两位置全无 origin、回收站无快照 → 扶正首条
    seed("AN-LINK", "链"); // origin 在回收站、全链接存活 → 不动
    seed("AN-OK", "正"); // 已有 origin → 不动
    let folders: FoldersFile = serde_json::from_str(
      r#"{
        "version": 1, "folders": [],
        "items": [
          { "asset": "AN-DUP", "folder": "root", "name": "副-1.fla", "order": 0, "origin": false },
          { "asset": "AN-DUP", "folder": "root", "name": "副-2.fla", "order": 1, "origin": false },
          { "asset": "AN-LINK", "folder": "root", "name": "链.fla", "order": 2, "origin": false },
          { "asset": "AN-OK", "folder": "root", "name": "正.fla", "order": 3, "origin": true }
        ]
      }"#,
    )
    .unwrap();
    write_json(&root.join(FOLDERS_FILE), &folders).unwrap();
    let recycle: RecycleFile = serde_json::from_str(
      r#"{
        "version": 1,
        "items": [ { "asset": "AN-LINK", "deletedAt": "2026-09-08T02:00:00+08:00",
          "placements": [ { "asset": "AN-LINK", "folder": "root", "name": "链.fla", "order": 0, "origin": true } ] } ]
      }"#,
    )
    .unwrap();
    write_json(&root.join(RECYCLE_FILE), &recycle).unwrap();

    let lib = open(&lib.space_root, &root).unwrap();
    let dup = lib.index.placements_of("AN-DUP");
    assert_eq!(dup.iter().filter(|p| p.origin).count(), 1, "扶正且仅一条");
    assert!(dup[0].origin && dup[0].name == "副-1.fla", "items 序首条扶正");
    assert!(!lib.index.placements_of("AN-LINK").iter().any(|p| p.origin), "origin 在回收站的过渡态不动");
    assert_eq!(lib.index.placements_of("AN-OK").iter().filter(|p| p.origin).count(), 1, "已有 origin 不动");
    // 落盘一致（经自检备份门禁写回）
    let disk: FoldersFile = read_json(&root.join(FOLDERS_FILE)).unwrap();
    assert!(disk.items.iter().find(|p| p.asset == "AN-DUP" && p.name == "副-1.fla").unwrap().origin, "folders.json 落盘");
    assert!(!disk.items.iter().find(|p| p.asset == "AN-LINK").unwrap().origin, "落盘同样不动");
  }

  /// 无主 info 扫描（2026-09-02）：孤儿清单结构（id / name / info 路径）——
  /// 前端警告弹窗与后续「打开资源管理器查看」入口的数据源
  #[test]
  fn scan_orphan_infos_notice_carries_name_and_path() {
    let dir = tempfile::tempdir().unwrap();
    let lib = create_test(dir.path());
    let info = lib.info_dir("AN-ORPHAN");
    fs::create_dir_all(&info).unwrap();
    fs::write(
      info.join("metadata.json"),
      r#"{ "version": 1, "id": "AN-ORPHAN", "name": "孤", "originName": "孤.fla",
           "ext": "fla", "size": 1, "importedAt": "2026-09-02T01:00:00+08:00",
           "modifiedAt": "2026-09-02T01:00:00+08:00" }"#,
    )
    .unwrap();
    let assets = load_assets(&lib.assets_dir).unwrap();
    let referenced: HashSet<String> = lib
      .folders
      .items
      .iter()
      .map(|p| p.asset.clone())
      .chain(lib.recycle.items.iter().map(|i| i.asset.clone()))
      .collect();
    let orphans = scan_orphan_infos(&lib.assets_dir, Some(&referenced), &assets);
    assert_eq!(orphans.len(), 1);
    assert_eq!(orphans[0].id, "AN-ORPHAN");
    assert_eq!(orphans[0].name, "孤");
    assert!(orphans[0].path.replace('\\', "/").ends_with("assets/AN-ORPHAN.info"), "带 info 完整路径：{}", orphans[0].path);
  }

  /// 无主扫描口径 = 空间级（2026-09-26 修复：新建 / 切换库把他库资产全报无主）：
  /// 仅被邻库引用的资产不报；全空间无引用的才报
  #[test]
  fn open_orphan_scan_uses_space_wide_references() {
    let dir = tempfile::tempdir().unwrap();
    let lib = create_test(dir.path());
    let space_root = lib.space_root.clone();
    let root = lib.root.clone();
    let seed = |id: &str, name: &str| {
      let info = lib.info_dir(id);
      fs::create_dir_all(&info).unwrap();
      fs::write(
        info.join("metadata.json"),
        format!(
          r#"{{ "version": 1, "id": "{id}", "name": "{name}", "originName": "{name}.fla",
               "ext": "fla", "size": 1, "importedAt": "2026-09-26T01:00:00+08:00",
               "modifiedAt": "2026-09-26T01:00:00+08:00" }}"#
        ),
      )
      .unwrap();
    };
    seed("AN-SIBLING", "邻"); // 仅邻库引用
    seed("AN-TRUE-ORPHAN", "孤"); // 全空间无引用
    // 同空间第二个库（磁盘态）：folders.json 引用 AN-SIBLING
    let mut space = space::open(&space_root).unwrap();
    create(&mut space, "邻库").unwrap();
    fs::write(
      space_root.join("邻库").join(FOLDERS_FILE),
      r#"{ "version": 1, "folders": [],
        "items": [ { "asset": "AN-SIBLING", "folder": "root", "name": "邻.fla", "order": 0, "origin": true } ] }"#,
    )
    .unwrap();
    drop(lib);

    let lib2 = open(&space_root, &root).unwrap();
    assert!(!lib2.orphan_infos.iter().any(|o| o.id == "AN-SIBLING"), "邻库引用的资产不是无主");
    assert!(lib2.orphan_infos.iter().any(|o| o.id == "AN-TRUE-ORPHAN"), "全空间无引用才是无主");
    assert_eq!(lib2.orphan_infos.len(), 1);
  }

  /// 打开自检不剔 root 链接（2026-08-13 用户定案：候车区不做内容限制，
  /// 原 prune_root_links 自检废除——root 下的链接 Placement 是合法数据，照常进索引）
  #[test]
  fn open_keeps_root_links() {
    let dir = tempfile::tempdir().unwrap();
    let lib = create_test(dir.path());
    let root = lib.root.clone();

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
          { "asset": "AN-REAL", "folder": "root", "name": "在.fla", "order": 0, "origin": false }
        ]
      }"#,
    )
    .unwrap();
    write_json(&root.join(FOLDERS_FILE), &folders).unwrap();

    let lib = open(&lib.space_root, &root).unwrap();
    assert_eq!(lib.index.placements_by_folder["root"].len(), 1, "root 链接照常进索引");
    assert_eq!(lib.index.placements_by_folder["F01"].len(), 1, "文件夹内 origin 不动");
    let persisted: FoldersFile = read_json(&root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(persisted.items.len(), 2, "root 链接保留不落盘清理");
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
    // 会话级缓存（2026-07-25 用户定）：tmp / previews / frames / waveforms / note-thumbs
    // / seq-trash（序列帧删帧暂存，2026-08-21）/ covers-trash（封面覆盖烘培撤销暂存，2026-09-08）打开库时整目录清空
    for sub in ["tmp", "previews", "frames", "waveforms", "note-thumbs", "seq-trash", "covers-trash"] {
      let d = root.join("cache").join(sub);
      fs::create_dir_all(&d).unwrap();
      fs::write(d.join("a.webp"), b"x").unwrap();
    }
    // 格式图标缓存跨会话保留
    let thumbs = root.join("cache/thumbs");
    fs::create_dir_all(&thumbs).unwrap();
    fs::write(thumbs.join("fla.webp"), b"x").unwrap();
    open(&lib.space_root, &root).unwrap();
    for sub in ["tmp", "previews", "frames", "waveforms", "note-thumbs", "seq-trash", "covers-trash"] {
      assert!(!root.join("cache").join(sub).exists(), "打开库应清空 cache/{sub}");
    }
    assert!(thumbs.join("fla.webp").exists(), "格式图标缓存应保留");
  }

  /// 档案封面自检（2026-09-08 覆盖式烘培定案配套）：旧版库根 covers/ 迁入 cache/covers/（引用不动），
  /// 无 archives 记录引用的封面文件开库即清（过期封面不累积）；covers/ 本体跨会话保留
  #[test]
  fn open_migrates_legacy_covers_and_purges_stale() {
    let dir = tempfile::tempdir().unwrap();
    let lib = create_test(dir.path());
    let space_root = lib.space_root.clone();
    let root = lib.root.clone();
    fs::write(
      root.join(FOLDERS_FILE),
      r#"{
        "version": 1,
        "folders": [ { "id": "F01", "name": "雪牧", "parent": null, "kind": "archive", "order": 0 } ],
        "items": []
      }"#,
    )
    .unwrap();
    fs::write(
      root.join(ARCHIVES_FILE),
      r#"{
        "version": 1,
        "archives": [ { "folderId": "F01", "avatar": { "file": "F01-20260823093000000.webp" } } ]
      }"#,
    )
    .unwrap();
    // 旧版库根 covers/：被引用 + 无引用（过期）各一
    let legacy = root.join("covers");
    fs::create_dir_all(&legacy).unwrap();
    fs::write(legacy.join("F01-20260823093000000.webp"), b"referenced").unwrap();
    fs::write(legacy.join("F01-20260822093000000.webp"), b"stale").unwrap();
    // 已在 cache/covers/ 的过期文件（上次会话换封面的遗留场景由暂存承担，这里测纯清理）
    let cache_covers = root.join("cache").join("covers");
    fs::create_dir_all(&cache_covers).unwrap();
    fs::write(cache_covers.join("F99.webp"), b"stale-in-cache").unwrap();

    let lib2 = open(&space_root, &root).unwrap();
    drop(lib2);
    assert!(
      cache_covers.join("F01-20260823093000000.webp").is_file(),
      "被引用封面迁入 cache/covers/"
    );
    assert!(!legacy.join("F01-20260822093000000.webp").exists(), "旧目录过期文件已清");
    assert!(!cache_covers.join("F99.webp").exists(), "cache/covers/ 无引用文件已清");
    assert!(!legacy.exists(), "旧版库根 covers/ 已移除");
    // 再开一次幂等（无 covers 相关报错 / 二次清理无副作用）
    open(&space_root, &root).unwrap();
  }

  /// 区块自检（2026-08-14 区块子系统）：旧版 folders.json（无 blocks 字段、Folder 无 block/icon）
  /// 开库注入默认区块「目录」并按惯例落盘（备份后写）
  #[test]
  fn open_injects_default_block_into_legacy_folders_file() {
    let dir = tempfile::tempdir().unwrap();
    let lib = create_test(dir.path());
    let root = lib.root.clone();

    fs::write(
      root.join(FOLDERS_FILE),
      r#"{
        "version": 1,
        "folders": [ { "id": "F01", "name": "人物", "parent": null, "order": 0 } ],
        "items": []
      }"#,
    )
    .unwrap();

    let lib = open(&lib.space_root, &root).unwrap();
    assert_eq!(lib.folders.blocks.len(), 1, "blocks 为空注入默认区块");
    assert_eq!(
      (lib.folders.blocks[0].id.as_str(), lib.folders.blocks[0].name.as_str(), lib.folders.blocks[0].order),
      (DEFAULT_BLOCK_ID, "文件夹", 0)
    );
    assert_eq!((lib.folders.folders[0].block.clone(), lib.folders.folders[0].icon.clone()), (None, None));
    let disk: FoldersFile = read_json(&root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(disk.blocks.len(), 1, "自检改动落盘");
  }

  /// 默认区块名迁移（2026-09-17 实机走查「目录语义改成文件夹」）：存量「目录」更名「文件夹」并落盘；
  /// 用户改过名的默认区块不动
  #[test]
  fn open_renames_legacy_default_block_name() {
    let dir = tempfile::tempdir().unwrap();
    let lib = create_test(dir.path());
    let root = lib.root.clone();

    fs::write(
      root.join(FOLDERS_FILE),
      r#"{
        "version": 1,
        "blocks": [ { "id": "dir", "name": "目录", "order": 0 } ],
        "folders": [],
        "items": []
      }"#,
    )
    .unwrap();
    let lib = open(&lib.space_root, &root).unwrap();
    assert_eq!(lib.folders.blocks[0].name, "文件夹", "旧名「目录」更名");
    let disk: FoldersFile = read_json(&root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(disk.blocks[0].name, "文件夹", "迁移落盘");

    fs::write(
      root.join(FOLDERS_FILE),
      r#"{
        "version": 1,
        "blocks": [ { "id": "dir", "name": "素材区", "order": 0 } ],
        "folders": [],
        "items": []
      }"#,
    )
    .unwrap();
    let lib = open(&lib.space_root, &root).unwrap();
    assert_eq!(lib.folders.blocks[0].name, "素材区", "用户改过名的不动");
  }

  /// 自定义图标迁移（2026-09-01，7.1 二十五轮）：旧 05_custom-icon 文件干就近映射（animal→dog）、
  /// 对不上的旧名与未知名回退 None；新 29 名原样保留（archive 档案专属出清单，存量值同回退）；
  /// Folder.icon 与 Block.icon 同规则并落盘
  #[test]
  fn open_migrates_legacy_custom_icon_values() {
    let dir = tempfile::tempdir().unwrap();
    let lib = create_test(dir.path());
    let root = lib.root.clone();

    fs::write(
      root.join(FOLDERS_FILE),
      r#"{
        "version": 1,
        "blocks": [
          { "id": "dir", "name": "目录", "icon": null, "order": 0 },
          { "id": "B01", "name": "参考", "icon": "weapon", "order": 1 }
        ],
        "folders": [
          { "id": "F01", "name": "动物", "parent": null, "order": 0, "icon": "animal" },
          { "id": "F02", "name": "狗狗", "parent": null, "order": 1, "icon": "dog" },
          { "id": "F03", "name": "杂物", "parent": null, "order": 2, "icon": "weapon" },
          { "id": "F04", "name": "手改", "parent": null, "order": 3, "icon": "not-a-icon" },
          { "id": "F05", "name": "旧档", "parent": null, "order": 4, "icon": "archive" }
        ],
        "items": []
      }"#,
    )
    .unwrap();

    let lib = open(&lib.space_root, &root).unwrap();
    let at = |id: &str| lib.folders.folders.iter().find(|f| f.id == id).unwrap().icon.clone();
    assert_eq!(at("F01").as_deref(), Some("dog"), "animal 就近映射 dog");
    assert_eq!(at("F02").as_deref(), Some("dog"), "新 30 名原样保留");
    assert_eq!(at("F03"), None, "对不上的旧名回退默认");
    assert_eq!(at("F04"), None, "任意未知名回退默认（防渲染报错）");
    assert_eq!(at("F05"), None, "archive 档案专属出清单（2026-09-01 走查）→ 存量值回退默认");
    assert_eq!(lib.folders.blocks[1].icon, None, "Block.icon 同规则回退");
    let disk: FoldersFile = read_json(&root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(disk.folders.iter().find(|f| f.id == "F01").unwrap().icon.as_deref(), Some("dog"), "迁移落盘");
  }

  /// 区块自检：默认区块缺失（已有其他区块）→ 补注队首；Folder.block 指向不存在的区块 → 置 None 自愈
  #[test]
  fn open_heals_missing_default_block_and_dangling_refs() {
    let dir = tempfile::tempdir().unwrap();
    let lib = create_test(dir.path());
    let root = lib.root.clone();

    fs::write(
      root.join(FOLDERS_FILE),
      r#"{
        "version": 1,
        "blocks": [ { "id": "B01", "name": "角色", "order": 5 } ],
        "folders": [
          { "id": "F01", "name": "甲", "parent": null, "block": "B01", "order": 0 },
          { "id": "F02", "name": "乙", "parent": null, "block": "B-不存在", "order": 1 }
        ],
        "items": []
      }"#,
    )
    .unwrap();

    let lib = open(&lib.space_root, &root).unwrap();
    assert_eq!(lib.folders.blocks.len(), 2, "补注默认区块");
    assert_eq!(lib.folders.blocks[0].id, DEFAULT_BLOCK_ID, "默认区块注在队首");
    let f01 = lib.folders.folders.iter().find(|f| f.id == "F01").unwrap();
    assert_eq!(f01.block.as_deref(), Some("B01"), "有效引用保留");
    let f02 = lib.folders.folders.iter().find(|f| f.id == "F02").unwrap();
    assert_eq!(f02.block, None, "失效引用置 None");
    let disk: FoldersFile = read_json(&root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(disk.folders.iter().find(|f| f.id == "F02").unwrap().block, None, "自愈落盘");
  }

  /// 档案自检（2026-08-15 档案子系统）：指向不存在 folder / folder 非档案形态的记录剔除并落盘
  #[test]
  fn open_heals_archives_file() {
    let dir = tempfile::tempdir().unwrap();
    let lib = create_test(dir.path());
    let root = lib.root.clone();

    fs::write(
      root.join(FOLDERS_FILE),
      r#"{
        "version": 1,
        "folders": [
          { "id": "F01", "name": "雪牧", "parent": null, "kind": "archive", "order": 0 },
          { "id": "F02", "name": "普通", "parent": null, "order": 1 }
        ],
        "items": []
      }"#,
    )
    .unwrap();
    fs::write(
      root.join(ARCHIVES_FILE),
      r#"{
        "version": 1,
        "archives": [
          { "folderId": "F01", "props": ["主角"] },
          { "folderId": "F-不存在" },
          { "folderId": "F02" }
        ]
      }"#,
    )
    .unwrap();

    let lib = open(&lib.space_root, &root).unwrap();
    assert_eq!(lib.archives.archives.len(), 1, "失效记录剔除");
    assert_eq!(lib.archives.archives[0].folder_id, "F01");
    assert_eq!(lib.archives.archives[0].props, vec!["主角"]);
    let disk: ArchivesFile = read_json(&root.join(ARCHIVES_FILE)).unwrap();
    assert_eq!(disk.archives.len(), 1, "自愈落盘");
  }

  /// 档案自检去重（2026-09-01 封面设置懒建 BUG 存量自愈）：同 folderId 重复记录保留首条
  ///（真数据恒在首条——更新永远落在 find 命中的首条，副本是后追加的空壳）并落盘
  #[test]
  fn open_dedups_archive_records() {
    let dir = tempfile::tempdir().unwrap();
    let lib = create_test(dir.path());
    let root = lib.root.clone();

    fs::write(
      root.join(FOLDERS_FILE),
      r#"{
        "version": 1,
        "folders": [
          { "id": "F01", "name": "雪牧", "parent": null, "kind": "archive", "order": 0 }
        ],
        "items": []
      }"#,
    )
    .unwrap();
    fs::write(
      root.join(ARCHIVES_FILE),
      r##"{
        "version": 1,
        "archives": [
          { "folderId": "F01", "badge": { "text": "女猪脚", "color": "#402122" }, "props": ["主角"] },
          { "folderId": "F01" },
          { "folderId": "F01" }
        ]
      }"##,
    )
    .unwrap();

    let lib = open(&lib.space_root, &root).unwrap();
    assert_eq!(lib.archives.archives.len(), 1, "重复记录去重");
    let rec = &lib.archives.archives[0];
    assert_eq!(rec.badge.as_ref().map(|b| b.text.as_str()), Some("女猪脚"), "首条真数据保留");
    assert_eq!(rec.props, vec!["主角"]);
    let disk: ArchivesFile = read_json(&root.join(ARCHIVES_FILE)).unwrap();
    assert_eq!(disk.archives.len(), 1, "自愈落盘");
  }

  /// 旧库无 archives.json（3.2 新增文件）：开库按空表加载不报错；无改动不补建文件
  #[test]
  fn open_without_archives_file_loads_empty() {
    let dir = tempfile::tempdir().unwrap();
    let lib = create_test(dir.path());
    let root = lib.root.clone();
    fs::remove_file(root.join(ARCHIVES_FILE)).unwrap();

    let lib = open(&lib.space_root, &root).unwrap();
    assert!(lib.archives.archives.is_empty());
    assert!(!root.join(ARCHIVES_FILE).exists(), "无改动不补建文件");
  }

  #[test]
  fn save_folders_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let mut lib = create_test(dir.path());
    lib.folders.folders.push(Folder { id: "F01".into(), name: "人物".into(), parent: None, block: None, icon: None, kind: None, color: None, groups: None, order: 0 });
    save_folders(&lib).unwrap();
    let back: FoldersFile = read_json(&lib.root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(back.folders.len(), 1);
    assert_eq!(back.folders[0].name, "人物");
  }

  #[test]
  fn unique_name_appends_counter_before_ext() {
    let existing: HashSet<String> =
      ["男人.fla".to_string(), "男人_1.fla".to_string(), "参考".to_string()].into_iter().collect();
    // 无冲突原样返回
    assert_eq!(unique_name(&existing, "女人.fla"), "女人.fla");
    // 冲突在扩展名前追加 _序号，已占用的序号跳过
    assert_eq!(unique_name(&existing, "男人.fla"), "男人_2.fla");
    // 无扩展名在末尾追加
    assert_eq!(unique_name(&existing, "参考"), "参考_1");
    // 点开头不视为扩展名
    let hidden: HashSet<String> = [".gitignore".to_string()].into_iter().collect();
    assert_eq!(unique_name(&hidden, ".gitignore"), ".gitignore_1");
  }
}
