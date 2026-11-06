//! 开库自检链（7.3 T5 收编，2026-11-03，自 library.rs open() 抽离）：
//! 每段 = 幂等自愈（剔除失效引用 / 扶正缺失标记 / 失效值迁移回退）——与一次性版本迁移
//!（migrate.rs）的分界：外部损坏随时发生 = 持续自检；JSON 结构变更才走 MIGRATIONS 升版。
//! run_self_checks 只改内存 + 返回改动清单，落盘统一走 open() 的备份门禁段
//!（关系 / 封面两个空间级与缓存级段除外，各自原地写回不随库备份门禁）。

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::core::library::{covers_dir, info_dirs, OrphanNotice, COVERS_DIR};
use crate::core::models::*;
use crate::core::relations;
use crate::core::space;

/// 自检结果：改动标记与清单（open() 备份门禁段据此决定哪些数据文件落盘）
pub(crate) struct SelfCheckOutcome {
  pub folders_changed: bool,
  pub archives_changed: bool,
  pub thumb_refs_fixed: Vec<String>,
  pub prompt_refs_fixed: Vec<String>,
  pub orphan_infos: Vec<OrphanNotice>,
}

/// 自检链主入口（open() 236-329 原序原样收编）：Placement 剔除 → origin 扶正 → 无主扫描 →
/// 区块表 → 自定义图标 → 伴生缩略图引用 → 提示词引用（资产 + 档案）→ 档案记录 → 封面 → 关系
pub(crate) fn run_self_checks(
  root: &Path,
  space_root: &Path,
  assets_dir: &Path,
  folders: &mut FoldersFile,
  recycle: &RecycleFile,
  assets: &mut [AssetMeta],
  archives: &mut ArchivesFile,
) -> SelfCheckOutcome {
  // 打开自检（2026-07-25 用户反馈：手动删 assets/{ID}.info 后数量成永久错误）：
  // 剔除指向缺失 info 的 Placement（内存与磁盘同清，数量统计随索引重建归零）。
  // 落盘前先强制备份一份含脏数据的快照；备份失败则不写盘（只内存清理，防误删无可回滚）。
  let removed = prune_missing_placements(folders, assets);
  if removed > 0 {
    log::warn!("打开自检剔除 {removed} 条指向缺失 info 的 Placement");
  }
  // 无 origin 扶正（2026-09-08 副本 origin 标记修复存量自愈）：有活位置无 origin 且回收站
  // 无其 origin 快照的资产，首条活位置置 origin（历史副本误标链接语义致回收站漏物理删）
  let origins_fixed = fix_missing_origins(folders, recycle);
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
  let orphan_infos = scan_orphan_infos(assets_dir, referenced.as_ref(), assets);
  if !orphan_infos.is_empty() {
    let detail = orphan_infos.iter().map(|o| format!("{}（{}）", o.id, o.name)).collect::<Vec<_>>().join("、");
    log::warn!("打开自检发现 {} 个无主资产（数据层无记录，info 残留磁盘）：{detail}", orphan_infos.len());
  }
  // 区块表自检（2026-08-14 区块子系统）：blocks 缺默认区块 → 注入「目录」；
  // Folder.block 指向不存在的区块 → 置 None（存量自愈）
  let blocks_fixed = fix_blocks(folders);
  if blocks_fixed {
    log::info!("打开自检修复区块表（注入默认区块 / 失效区块引用置空）");
  }
  // 自定义图标值迁移（2026-09-01，7.1 二十五轮）：02自定义 新 30 图标集——旧文件干
  // 就近映射（animal→dog 等 5 项），失效值回退默认 None（用户定）
  let icons_fixed = fix_custom_icons(folders);
  if icons_fixed {
    log::info!("打开自检迁移自定义图标值（旧文件干就近映射 / 失效值回退默认）");
  }
  // 伴生缩略图引用自检（8.1，PRD 02-15）：thumb_asset_id 指向已不存在资产 → 剔除（失效引用）。
  // 反向（hidden 资产无引用）合法不动——换绑遗留由用户手动「解除缩略图」或删除
  let thumb_refs_fixed = fix_thumb_refs(assets);
  if !thumb_refs_fixed.is_empty() {
    log::info!("打开自检剔除 {} 条失效伴生缩略图引用（thumb_asset_id）", thumb_refs_fixed.len());
  }
  // 提示词引用自检（2026-09-26 提示词轮，PRD 03_数据链接 02_文件）：prompt.asset_id 指向
  // 已不存在资产 → 引用置 None、快照保留降级为已扫描态（资产侧逐 metadata 落盘 + 档案侧随 archives）
  let prompt_refs_fixed = fix_prompt_refs(assets);
  if !prompt_refs_fixed.is_empty() {
    log::info!("打开自检降级 {} 条失效提示词引用（prompt.assetId 置 None、快照保留）", prompt_refs_fixed.len());
  }
  let archive_prompt_fixed = fix_archive_prompt_refs(archives, assets);
  if archive_prompt_fixed {
    log::info!("打开自检降级档案侧失效提示词引用（快照保留）");
  }
  // 档案数据自检（2026-08-15 档案子系统）：剔除指向不存在 folder / folder 非档案形态的记录；
  // 同 folder_id 重复记录去重（2026-09-01 封面设置懒建 BUG 存量自愈）
  let archives_fixed = fix_archives(archives, folders);
  if archives_fixed {
    log::info!("打开自检修复 archives.json（剔除失效档案记录 / 重复记录去重）");
  }
  // 档案封面自检（2026-09-08 覆盖式烘培定案配套，纯缓存操作失败只记日志不阻断开库）：
  // ① 旧版库根 covers/ 迁入 cache/covers/；② 清理无 archives 记录引用的封面文件
  //（覆盖式后一档案一文件，历史过期封面不再累积——用户实锤"covers 里几十个过期封面"）
  {
    let (migrated, purged) = migrate_and_purge_covers(root, archives);
    if migrated > 0 {
      log::info!("打开自检迁移旧版档案封面 {migrated} 个（库根 covers/ → cache/covers/）");
    }
    if purged > 0 {
      log::info!("打开自检清理过期档案封面 {purged} 个（无记录引用）");
    }
  }
  // 关系自检（6.2 选择模式一轮，PRD 03_数据链接 06）：空间级 relations.json——剔除 from/to
  // 端点指向本库已失效目标的关系（跨库端点无法校验，保留）。空间级文件不在库备份清单，
  // 剔除项均为失效引用，有改动直接原子写回（不随库文件备份门禁段）
  {
    let lib_name = root.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    match relations::load_relations(space_root) {
      Ok(mut rels) => {
        if relations::prune_relations(&mut rels, &lib_name, folders, assets) {
          match relations::save_relations(space_root, &rels) {
            Ok(()) => log::info!("打开自检修复 relations.json（剔除本库失效端点的关系）"),
            Err(e) => log::error!("关系自检落盘失败（数据文件保持原样）：{e}"),
          }
        }
      }
      Err(e) => log::error!("关系自检读取失败（跳过）：{e}"),
    }
  }
  SelfCheckOutcome {
    folders_changed: removed > 0 || blocks_fixed || icons_fixed || origins_fixed > 0,
    archives_changed: archives_fixed || archive_prompt_fixed,
    thumb_refs_fixed,
    prompt_refs_fixed,
    orphan_infos,
  }
}

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
/// 返回孤儿清单（测试断言用）；pub(crate) = library.rs 既有测试同径调用
pub(crate) fn scan_orphan_infos(assets_dir: &Path, referenced: Option<&HashSet<String>>, assets: &[AssetMeta]) -> Vec<OrphanNotice> {
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
