//! 资源库合并（PRD 01_储存空间 3.2「多个资源库允许合并为一个」，2026-09-26 功能轮定案 +
//! 二轮交互修订）：勾选集（≥2）并入目标库（= 勾选集中 libraryOrder 最靠前者，合并完成即打开），
//! 源库目录随后删除（命令层编排：目标库就位（开 / 取当前）→ 备份 → 本模块纯数据并入 →
//! 单写点落盘 → relations.json 端点改写 → 封面迁移 → space::delete_library 删源目录）。
//!
//! 合并口径：
//! - 素材实体不动（空间级 assets 池多库共享，并入 = 数据层归拢）；
//! - folders.json：区块 / 文件夹 / Placement 全量并入；id 撞车（F/B + 时间戳 + 随机，理论撞）
//!   才重生成并同步改全部引用（parent / block / Placement.folder / archives.folderId /
//!   pins.target / relations 档案端点 + 封面文件名）；默认区块「dir」两库同一个，跳过不并；
//! - 同名不去重（库本就允许同名——区块新建不做唯一化、Placement「保留两个」允许同名，
//!   2026-08-14 用户定同类口径）；仅完全重复（同资产 + 同文件夹 + 同名）的 Placement 跳过，
//!   origin 转移给已存在条；
//! - origin 治理：一资产库内 origin 恒 ≤ 1（fix_missing_origins 同口径）——目标库已有 origin
//!   （folders ∪ 目标回收站 origin 快照 ∪ 本批已并入）的资产，源 origin 降级为链接；
//! - 层级内排序：源顶级文件夹 order 续排目标顶级末尾（批内保序）；源 root Placement 的 order
//!   续排目标 root 末尾；子级文件夹与其内 Placement 的 order 原样（同级皆源库自家人）；
//! - recycle.json：同资产条目并成一条（placements 按 (folder,name) 取并集，deletedAt 取早），
//!   placements 内 folder 引用随 id 重映射、origin 同治理；
//! - recent.json：同资产保留最新 viewedAt，倒序截 RECENT_LIMIT（500）；
//! - pins.json：按 target 去重并入（文件夹 target 随 id 重映射），order 续排；
//! - archives.json：档案记录随 folderId 重映射并入；封面文件迁移（cache/covers/ 跨库拷贝 +
//!   文件名随新 folderId 改前缀）由命令层按 MergeOutcome.covers 执行；
//! - tags.json 不并（标签体系已取消、无写点——源库标签定义随目录删除废弃）。

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::core::json_io::{read_json, DataError};
use crate::core::library::{ARCHIVES_FILE, FOLDERS_FILE, PINS_FILE, RECENT_FILE, RECYCLE_FILE};
use crate::core::models::*;

/// 源库数据（合并输入）：源库未打开，直接读盘；archives.json 缺失按空表（3.2 新增文件，
/// 旧库无此文件不算损坏，同 library::open 口径），其余缺失 / 损坏视为读库失败报错
pub struct SourceLibrary {
  pub folders: FoldersFile,
  pub pins: PinsFile,
  pub recycle: RecycleFile,
  pub recent: RecentFile,
  pub archives: ArchivesFile,
}

/// 读入源库数据文件（版本门禁随 read_json——高于当前版本的源库拒绝并入）
pub fn read_source(dir: &Path) -> Result<SourceLibrary, DataError> {
  let archives_path = dir.join(ARCHIVES_FILE);
  let archives = if archives_path.is_file() { read_json(&archives_path)? } else { ArchivesFile::new() };
  Ok(SourceLibrary {
    folders: read_json(&dir.join(FOLDERS_FILE))?,
    pins: read_json(&dir.join(PINS_FILE))?,
    recycle: read_json(&dir.join(RECYCLE_FILE))?,
    recent: read_json(&dir.join(RECENT_FILE))?,
    archives,
  })
}

/// 合并目标（当前打开库的可变字段组）：命令层自 Library 组装；剥离 Library 壳便于纯数据单测
pub struct MergeTarget<'a> {
  pub folders: &'a mut FoldersFile,
  pub pins: &'a mut PinsFile,
  pub recycle: &'a mut RecycleFile,
  pub recent: &'a mut RecentFile,
  pub archives: &'a mut ArchivesFile,
}

/// 一条封面迁移：源库 cache/covers/ 内文件名 → 目标库定名（建议名，撞名由命令层唯一化并回写
/// 档案记录）；folder_id = 重映射后的档案 id（命令层回写 avatar.file 定位用）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverMove {
  pub from: String,
  pub to: String,
  pub folder_id: String,
}

/// 单库并入产物：id 重映射表（relations.json 档案端点改写用）+ 封面迁移清单 + 统计（日志）
#[derive(Debug, Default)]
pub struct MergeOutcome {
  /// 源文件夹 id → 新 id（仅撞车重组的条目；未撞不在表内——引用方按「命中才改」查表）
  pub folder_id_map: HashMap<String, String>,
  pub covers: Vec<CoverMove>,
  pub folders: usize,
  pub placements: usize,
  /// 完全重复跳过的 Placement 数
  pub skipped_dup: usize,
  /// 目标已有 origin 被降级为链接的条数（folders + 回收站快照合计）
  pub demoted_origins: usize,
  /// 源回收站并入条目数
  pub recycle: usize,
}

/// 封面文件定名：现行覆盖式命名恒 {folder_id}.webp/.png、旧版带时间戳 {folder_id}-<ts>.webp——
/// 均以旧 folderId 开头，前缀换新 id；对不上前缀的（手编数据）保留原名，撞名由命令层唯一化
fn cover_target_name(old_file: &str, old_id: &str, new_id: &str) -> String {
  match old_file.strip_prefix(old_id) {
    Some(rest) => format!("{new_id}{rest}"),
    None => old_file.to_string(),
  }
}

/// 源库并入目标库（纯数据，无磁盘 IO）：六文件中除 tags 外全量合并，口径见模块头
pub fn merge_into(t: &mut MergeTarget, source: SourceLibrary) -> MergeOutcome {
  let mut outcome = MergeOutcome::default();

  // ① 区块并入：默认区块「dir」两库同 id 同一个，跳过；其余撞 id 重生成，order 续排目标末尾。
  // 同名不去重（2026-08-14 用户定：新建区块名不做唯一化，库允许同名）
  let mut block_map: HashMap<String, String> = HashMap::new();
  let mut next_block_order = t.folders.blocks.iter().map(|b| b.order).max().unwrap_or(-1) + 1;
  for b in source.folders.blocks {
    if b.id == DEFAULT_BLOCK_ID {
      continue;
    }
    let id = if t.folders.blocks.iter().any(|x| x.id == b.id) { new_block_id() } else { b.id.clone() };
    if id != b.id {
      block_map.insert(b.id.clone(), id.clone());
    }
    t.folders.blocks.push(Block { id, name: b.name, icon: b.icon, order: next_block_order });
    next_block_order += 1;
  }
  let map_block = |id: &str| -> String { block_map.get(id).cloned().unwrap_or_else(|| id.to_string()) };

  // ② 文件夹 id 撞车重映射（F+时间戳+随机，撞 = 同分钟同随机；未撞原样保留——pins /
  // relations 等既有引用不动）
  let mut folder_map: HashMap<String, String> = HashMap::new();
  for f in &source.folders.folders {
    if t.folders.folders.iter().any(|x| x.id == f.id) {
      folder_map.insert(f.id.clone(), new_folder_id());
    }
  }
  let map_folder = |id: &str| -> String { folder_map.get(id).cloned().unwrap_or_else(|| id.to_string()) };
  outcome.folder_id_map = folder_map.clone();

  // ③ 文件夹并入：顶级（parent None）order 续排目标顶级末尾（按源 order 保序）；
  // 子级 order 原样（同级皆源库自家人，内部序不动）
  let root_order_base =
    t.folders.folders.iter().filter(|f| f.parent.is_none()).map(|f| f.order).max().map(|m| m + 1).unwrap_or(0);
  let mut tops: Vec<&Folder> = source.folders.folders.iter().filter(|f| f.parent.is_none()).collect();
  tops.sort_by_key(|f| f.order);
  let top_order: HashMap<String, i32> =
    tops.iter().enumerate().map(|(i, f)| (f.id.clone(), root_order_base + i as i32)).collect();
  drop(tops);
  for f in source.folders.folders {
    let order = if f.parent.is_none() { top_order[f.id.as_str()] } else { f.order };
    t.folders.folders.push(Folder {
      id: map_folder(&f.id),
      name: f.name,
      parent: f.parent.map(|p| map_folder(&p)),
      block: f.block.map(|b| map_block(&b)),
      icon: f.icon,
      kind: f.kind,
      color: f.color,
      groups: f.groups,
      order,
    });
    outcome.folders += 1;
  }

  // origin 治理全集：目标 folders origin ∪ 目标回收站 origin 快照（本批保留的 origin 随入随记，
  // 多源顺序并入时后源让先源）
  let mut has_origin: HashSet<String> = t
    .folders
    .items
    .iter()
    .filter(|p| p.origin)
    .map(|p| p.asset.clone())
    .chain(t.recycle.items.iter().flat_map(|i| i.placements.iter()).filter(|p| p.origin).map(|p| p.asset.clone()))
    .collect();

  // ④ Placement 并入（folders.items）：folder 随 id 重映射（root 不在表内原样）；完全重复
  //（同资产 + 同文件夹 + 同名）跳过、origin 转移给已存在条；root 的 order 续排目标 root 末尾
  let mut key_index: HashMap<(String, String, String), usize> = t
    .folders
    .items
    .iter()
    .enumerate()
    .map(|(i, p)| ((p.asset.clone(), p.folder.clone(), p.name.clone()), i))
    .collect();
  let mut next_root_item_order =
    t.folders.items.iter().filter(|p| p.folder == ROOT_FOLDER_ID).map(|p| p.order).max().map(|m| m + 1).unwrap_or(0);
  for p in source.folders.items {
    let folder = map_folder(&p.folder);
    let mut origin = p.origin;
    if origin && has_origin.contains(&p.asset) {
      origin = false;
      outcome.demoted_origins += 1;
    }
    let key = (p.asset.clone(), folder.clone(), p.name.clone());
    if let Some(&idx) = key_index.get(&key) {
      // 完全重复：incoming 过了 origin 治理而既有条无 origin → origin 转移给既有条
      if origin && !t.folders.items[idx].origin {
        t.folders.items[idx].origin = true;
        has_origin.insert(p.asset.clone());
      }
      outcome.skipped_dup += 1;
      continue;
    }
    if origin {
      has_origin.insert(p.asset.clone());
    }
    let order = if folder == ROOT_FOLDER_ID {
      let o = next_root_item_order;
      next_root_item_order += 1;
      o
    } else {
      p.order
    };
    key_index.insert(key, t.folders.items.len());
    t.folders.items.push(Placement { asset: p.asset, folder, name: p.name, order, origin });
    outcome.placements += 1;
  }

  // ⑤ 回收站并入：placements 的 folder 引用随 id 重映射 + origin 同治理；同资产条目并成一条
  //（placements 按 (folder,name) 取并集——同资产同位置同名快照只留一份，deletedAt 取早）
  for item in source.recycle.items {
    outcome.recycle += 1;
    let mut cleaned: Vec<Placement> = Vec::with_capacity(item.placements.len());
    for p in item.placements {
      let mut origin = p.origin;
      if origin && has_origin.contains(&item.asset) {
        origin = false;
        outcome.demoted_origins += 1;
      }
      if origin {
        has_origin.insert(item.asset.clone());
      }
      cleaned.push(Placement {
        asset: item.asset.clone(),
        folder: map_folder(&p.folder),
        name: p.name,
        order: p.order,
        origin,
      });
    }
    match t.recycle.items.iter_mut().find(|i| i.asset == item.asset) {
      Some(existing) => {
        let mut keys: HashSet<(String, String)> =
          existing.placements.iter().map(|p| (p.folder.clone(), p.name.clone())).collect();
        for p in cleaned {
          if keys.insert((p.folder.clone(), p.name.clone())) {
            existing.placements.push(p);
          }
        }
        if item.deleted_at < existing.deleted_at {
          existing.deleted_at = item.deleted_at;
        }
      }
      None => {
        t.recycle.items.push(RecycleItem { asset: item.asset, placements: cleaned, deleted_at: item.deleted_at });
      }
    }
  }

  // ⑥ 最近使用并入：同资产保留最新 viewedAt，倒序截 RECENT_LIMIT（500）
  if !source.recent.items.is_empty() {
    let mut latest: HashMap<String, String> =
      t.recent.items.iter().map(|i| (i.asset.clone(), i.viewed_at.clone())).collect();
    for i in source.recent.items {
      latest
        .entry(i.asset)
        .and_modify(|v| {
          if i.viewed_at > *v {
            *v = i.viewed_at.clone();
          }
        })
        .or_insert(i.viewed_at);
    }
    let mut items: Vec<RecentItem> =
      latest.into_iter().map(|(asset, viewed_at)| RecentItem { asset, viewed_at }).collect();
    items.sort_by(|a, b| b.viewed_at.cmp(&a.viewed_at));
    items.truncate(RECENT_LIMIT);
    t.recent.items = items;
  }

  // ⑦ 快捷入口并入：文件夹 target 随 id 重映射（资产 id / 其他原样），按 target 去重，order 续排
  {
    let mut existing: HashSet<String> = t.pins.pins.iter().map(|p| p.target.clone()).collect();
    let mut next_pin_order = t.pins.pins.iter().map(|p| p.order).max().map(|m| m + 1).unwrap_or(0);
    for p in source.pins.pins {
      let target = map_folder(&p.target);
      if !existing.insert(target.clone()) {
        continue;
      }
      t.pins.pins.push(Pin { target, order: next_pin_order });
      next_pin_order += 1;
    }
  }

  // ⑧ 档案并入：folderId 随 id 重映射（重映射后必唯一，目标侧同 id 记录 = 源内重复，防御跳过）；
  // 封面定名随新 folderId 改前缀，迁移清单交命令层跨库拷贝
  for a in source.archives.archives {
    let new_fid = map_folder(&a.folder_id);
    if t.archives.archives.iter().any(|x| x.folder_id == new_fid) {
      continue;
    }
    let avatar = a.avatar.map(|av| {
      let file = cover_target_name(&av.file, &a.folder_id, &new_fid);
      outcome.covers.push(CoverMove { from: av.file.clone(), to: file.clone(), folder_id: new_fid.clone() });
      Avatar { file, crop: av.crop }
    });
    t.archives.archives.push(ArchiveData {
      folder_id: new_fid,
      badge: a.badge,
      avatar,
      props: a.props,
      descriptions: a.descriptions,
      links: a.links,
      created_at: a.created_at,
      prompt: a.prompt,
    });
  }

  outcome
}

#[cfg(test)]
mod tests {
  use super::*;

  fn folder(id: &str, name: &str, parent: Option<&str>, order: i32) -> Folder {
    Folder {
      id: id.into(),
      name: name.into(),
      parent: parent.map(|s| s.to_string()),
      block: None,
      icon: None,
      kind: None,
      color: None,
      groups: None,
      order,
    }
  }

  fn placement(asset: &str, folder: &str, name: &str, order: i32, origin: bool) -> Placement {
    Placement { asset: asset.into(), folder: folder.into(), name: name.into(), order, origin }
  }

  fn recycle_item(asset: &str, deleted_at: &str, placements: Vec<Placement>) -> RecycleItem {
    RecycleItem { asset: asset.into(), placements, deleted_at: deleted_at.into() }
  }

  /// 五文件持有者：可变借用组装 MergeTarget 的样板收进 merge()
  struct T {
    folders: FoldersFile,
    pins: PinsFile,
    recycle: RecycleFile,
    recent: RecentFile,
    archives: ArchivesFile,
  }

  impl T {
    fn new() -> Self {
      T {
        folders: FoldersFile::new(),
        pins: PinsFile::new(),
        recycle: RecycleFile::new(),
        recent: RecentFile::new(),
        archives: ArchivesFile::new(),
      }
    }
    fn merge(&mut self, source: SourceLibrary) -> MergeOutcome {
      let mut t = MergeTarget {
        folders: &mut self.folders,
        pins: &mut self.pins,
        recycle: &mut self.recycle,
        recent: &mut self.recent,
        archives: &mut self.archives,
      };
      merge_into(&mut t, source)
    }
  }

  fn source_with(folders: FoldersFile) -> SourceLibrary {
    SourceLibrary {
      folders,
      pins: PinsFile::new(),
      recycle: RecycleFile::new(),
      recent: RecentFile::new(),
      archives: ArchivesFile::new(),
    }
  }

  /// 基础并入：文件夹 / Placement 全量进目标；顶级文件夹 order 续排目标末尾且批内按源 order 保序；
  /// 子级 order 原样；无 id 撞车原样保留；默认区块不重复并入
  #[test]
  fn merge_appends_folders_and_placements() {
    let mut t = T::new();
    t.folders.folders.push(folder("F-T1", "目标夹", None, 0));
    t.folders.folders.push(folder("F-T2", "目标子", Some("F-T1"), 0));
    t.folders.items.push(placement("AN-T", "F-T1", "在.fla", 0, true));

    let mut sf = FoldersFile::new();
    sf.blocks.push(Block { id: "B-S1".into(), name: "参考".into(), icon: None, order: 1 });
    sf.folders.push(folder("F-S1", "源夹乙", None, 5));
    sf.folders.push(folder("F-S2", "源夹甲", None, 3)); // 源 order 甲 < 乙 → 并入后甲排乙前
    sf.folders.push(folder("F-S3", "源子", Some("F-S1"), 7));
    sf.items.push(placement("AN-S", "F-S1", "来.fla", 9, true));
    sf.items.push(placement("AN-S", ROOT_FOLDER_ID, "来.fla", 4, false));

    let outcome = t.merge(source_with(sf));
    assert_eq!(outcome.folders, 3);
    assert_eq!(outcome.placements, 2);
    assert!(outcome.folder_id_map.is_empty(), "无撞车不重映射");
    assert_eq!(t.folders.blocks.len(), 2, "默认区块跳过不并，自定义区块并入");
    assert_eq!(t.folders.blocks[1].id, "B-S1");
    assert_eq!(t.folders.blocks[1].order, 1, "区块 order 续排目标末尾（目标仅默认区块 order 0）");

    let top = |id: &str| t.folders.folders.iter().find(|f| f.id == id).unwrap().clone();
    assert_eq!(top("F-S2").order, 1, "顶级续排目标末尾（目标顶级 max=0），甲 order 小排前");
    assert_eq!(top("F-S1").order, 2);
    assert_eq!(top("F-S3").order, 7, "子级 order 原样");
    assert_eq!(top("F-S3").parent.as_deref(), Some("F-S1"));

    assert_eq!(t.folders.items.len(), 3);
    let root_p = t.folders.items.iter().find(|p| p.folder == ROOT_FOLDER_ID).unwrap();
    assert!(root_p.order >= 0 && !root_p.origin, "root 链接并入、链接态保留");
  }

  /// id 撞车重映射：文件夹 / 区块撞 id 重生成，引用方（parent / block / Placement.folder /
  /// archives.folderId / pins.target / 封面文件名前缀）同步改
  #[test]
  fn merge_remaps_colliding_ids() {
    let mut t = T::new();
    t.folders.blocks.push(Block { id: "B-DUP".into(), name: "目标区块".into(), icon: None, order: 1 });
    t.folders.folders.push(folder("F-DUP", "目标夹", None, 0));

    let mut sf = FoldersFile::new();
    sf.blocks.push(Block { id: "B-DUP".into(), name: "源区块".into(), icon: None, order: 1 });
    sf.folders.push({
      let mut f = folder("F-DUP", "源夹", None, 0);
      f.block = Some("B-DUP".into());
      f.kind = Some(ARCHIVE_KIND.into());
      f
    });
    sf.folders.push(folder("F-SUB", "源子", Some("F-DUP"), 0));
    sf.items.push(placement("AN-S", "F-DUP", "来.fla", 0, true));

    let mut source = source_with(sf);
    source.pins.pins.push(Pin { target: "F-DUP".into(), order: 0 });
    source.archives.archives.push(ArchiveData {
      folder_id: "F-DUP".into(),
      badge: None,
      avatar: Some(Avatar { file: "F-DUP.webp".into(), crop: None }),
      props: vec![],
      descriptions: vec![],
      links: vec![],
      created_at: String::new(),
      prompt: None,
    });

    let outcome = t.merge(source);
    let new_fid = outcome.folder_id_map.get("F-DUP").expect("撞车应重映射").clone();
    assert_ne!(new_fid, "F-DUP");
    assert!(new_fid.starts_with('F'));

    let moved = t.folders.folders.iter().find(|f| f.id == new_fid).unwrap();
    assert_eq!(moved.name, "源夹");
    let new_block = moved.block.as_deref().expect("区块引用保留");
    assert_ne!(new_block, "B-DUP", "撞车区块同步重映射");
    assert!(t.folders.blocks.iter().any(|b| b.id == new_block && b.name == "源区块"));
    assert_eq!(t.folders.folders.iter().find(|f| f.id == "F-SUB").unwrap().parent.as_deref(), Some(new_fid.as_str()));
    assert_eq!(t.folders.items.iter().find(|p| p.asset == "AN-S").unwrap().folder, new_fid);
    assert_eq!(t.pins.pins[0].target, new_fid, "pins target 随重映射");
    let rec = t.archives.archives.iter().find(|r| r.folder_id == new_fid).unwrap();
    assert_eq!(rec.avatar.as_ref().unwrap().file, format!("{new_fid}.webp"), "封面文件名随新 id 改前缀");
    assert_eq!(outcome.covers, vec![CoverMove { from: "F-DUP.webp".into(), to: format!("{new_fid}.webp"), folder_id: new_fid }]);
  }

  /// origin 治理：目标已有 origin（folders 或回收站快照）的资产，源 origin 降级为链接；
  /// 仅源有的资产 origin 保留；一资产多 origin（源内腐数据）只保首条
  #[test]
  fn merge_demotes_conflicting_origins() {
    let mut t = T::new();
    t.folders.items.push(placement("AN-BOTH", "root", "双.fla", 0, true));
    t.recycle.items.push(recycle_item("AN-REC", "2026-09-01T00:00:00+08:00", vec![placement("AN-REC", "root", "站.fla", 0, true)]));

    let mut sf = FoldersFile::new();
    sf.items.push(placement("AN-BOTH", "root", "双.fla", 0, true)); // 目标 folders 已有 origin → 降级
    sf.items.push(placement("AN-ONLY", "root", "独.fla", 1, true)); // 目标无 → 保留
    sf.items.push(placement("AN-REC", "root", "站-活.fla", 2, true)); // 目标回收站有其 origin 快照 → 降级
    let mut source = source_with(sf);
    source.recycle.items.push(recycle_item("AN-BOTH", "2026-09-02T00:00:00+08:00", vec![placement("AN-BOTH", "root", "双-站.fla", 0, true)]));

    let outcome = t.merge(source);
    assert_eq!(outcome.demoted_origins, 3, "folders 两条 + 回收站快照一条降级");
    assert_eq!(t.folders.items.iter().filter(|p| p.asset == "AN-BOTH" && p.origin).count(), 1, "库内 origin 恒 ≤ 1");
    assert!(t.folders.items.iter().any(|p| p.asset == "AN-ONLY" && p.origin), "仅源有的 origin 保留");
    assert!(!t.folders.items.iter().any(|p| p.asset == "AN-REC" && p.origin));
    // 同资产回收站条目并成一条，快照 origin 降级、deletedAt 不取晚
    let item = t.recycle.items.iter().find(|i| i.asset == "AN-BOTH").unwrap();
    assert!(item.placements.iter().all(|p| !p.origin));
    assert_eq!(item.deleted_at, "2026-09-02T00:00:00+08:00");
  }

  /// 完全重复（同资产 + 同文件夹 + 同名）跳过；incoming 带 origin 而既有条没有 → origin 转移
  #[test]
  fn merge_skips_exact_dup_and_transfers_origin() {
    let mut t = T::new();
    t.folders.items.push(placement("AN-X", "root", "同.fla", 0, false)); // 目标链接态（origin 在回收站等过渡）
    t.folders.items.push(placement("AN-Y", "root", "留.fla", 1, true));

    let mut sf = FoldersFile::new();
    sf.items.push(placement("AN-X", "root", "同.fla", 0, true)); // 完全重复但带 origin → 跳过 + 转移
    sf.items.push(placement("AN-X", "root", "同-异名.fla", 1, false)); // 名不同 = 另一条链接，并入
    let outcome = t.merge(source_with(sf));

    assert_eq!(outcome.skipped_dup, 1);
    assert_eq!(outcome.placements, 1);
    assert_eq!(t.folders.items.iter().filter(|p| p.asset == "AN-X").count(), 2);
    let kept = t.folders.items.iter().find(|p| p.asset == "AN-X" && p.name == "同.fla").unwrap();
    assert!(kept.origin, "origin 转移给已存在条");
  }

  /// 回收站并入：同资产 placements 按 (folder,name) 取并集、deletedAt 取早；
  /// 快照内 folder 引用随 id 重映射
  #[test]
  fn merge_recycle_unions_placements() {
    let mut t = T::new();
    t.folders.folders.push(folder("F-DUP", "目标夹", None, 0));
    t.recycle.items.push(recycle_item(
      "AN-R",
      "2026-09-10T00:00:00+08:00",
      vec![placement("AN-R", "F-DUP", "甲.fla", 0, true), placement("AN-R", "root", "乙.fla", 1, false)],
    ));

    let mut sf = FoldersFile::new();
    sf.folders.push(folder("F-DUP", "源夹", None, 0)); // 撞 id → 重映射
    let mut source = source_with(sf);
    source.recycle.items.push(recycle_item(
      "AN-R",
      "2026-09-05T00:00:00+08:00", // 更早
      vec![placement("AN-R", "F-DUP", "甲.fla", 0, true), placement("AN-R", "F-DUP", "丙.fla", 1, false)],
    ));

    let outcome = t.merge(source);
    let new_fid = outcome.folder_id_map["F-DUP"].clone();
    let item = t.recycle.items.iter().find(|i| i.asset == "AN-R").unwrap();
    assert_eq!(item.deleted_at, "2026-09-05T00:00:00+08:00", "deletedAt 取早");
    assert_eq!(
      item.placements.len(),
      4,
      "(folder,name) 并集——目标 2（F-DUP/甲 + root/乙）+ 源 2（重映射后新 id/甲 + 新 id/丙，folder 不同不与目标 甲.fla 撞）"
    );
    // 目标侧 (F-DUP,甲.fla) origin=true；源两条 origin 快照降级（目标已有 origin）
    assert_eq!(item.placements.iter().filter(|p| p.origin).count(), 1);
    assert!(item.placements.iter().any(|p| p.folder == new_fid && p.name == "丙.fla"), "快照 folder 引用随重映射");
    assert!(item.placements.iter().any(|p| p.folder == "F-DUP" && p.name == "甲.fla"));
  }

  /// 最近使用并入：同资产保留最新、倒序、截 500
  #[test]
  fn merge_recent_keeps_latest_and_caps() {
    let mut t = T::new();
    for i in 0..RECENT_LIMIT {
      t.recent.items.push(RecentItem { asset: format!("AN-T{i}"), viewed_at: format!("2026-09-01T00:{i:04}:00+08:00") });
    }
    t.recent.items[0].viewed_at = "2026-09-01T00:00:00+08:00".into(); // AN-T0 目标侧较旧
    let mut source = source_with(FoldersFile::new());
    source.recent.items.push(RecentItem { asset: "AN-T0".into(), viewed_at: "2026-09-20T00:00:00+08:00".into() });
    source.recent.items.push(RecentItem { asset: "AN-S".into(), viewed_at: "2026-09-21T00:00:00+08:00".into() });

    t.merge(source);
    assert_eq!(t.recent.items.len(), RECENT_LIMIT, "截 RECENT_LIMIT");
    assert_eq!(t.recent.items[0].asset, "AN-S", "最新排首");
    assert_eq!(t.recent.items[1].asset, "AN-T0");
    let t0 = t.recent.items.iter().find(|i| i.asset == "AN-T0").unwrap();
    assert_eq!(t0.viewed_at, "2026-09-20T00:00:00+08:00", "同资产保留最新");
  }

  /// pins / archives 并入：按 target / folderId 去重，order 续排；无 avatar 不出封面迁移项
  #[test]
  fn merge_pins_and_archives_dedup() {
    let mut t = T::new();
    t.pins.pins.push(Pin { target: "AN-T".into(), order: 0 });
    t.archives.archives.push(ArchiveData::new("F-A"));

    let mut source = source_with(FoldersFile::new());
    source.pins.pins.push(Pin { target: "AN-T".into(), order: 0 }); // 撞 → 跳
    source.pins.pins.push(Pin { target: "F-S".into(), order: 1 });
    source.archives.archives.push(ArchiveData::new("F-A")); // 撞 → 跳（防御：实际不撞 id）
    source.archives.archives.push(ArchiveData::new("F-B"));

    let outcome = t.merge(source);
    assert_eq!(t.pins.pins.len(), 2);
    assert_eq!(t.pins.pins[1].order, 1, "order 续排");
    assert_eq!(t.archives.archives.len(), 2);
    assert!(outcome.covers.is_empty(), "无 avatar 不出封面迁移项");
  }

  /// 封面定名：覆盖式 {id}.webp 与旧版 {id}-<ts>.webp 前缀替换；不对前缀的保留原名
  #[test]
  fn cover_name_rewrites_id_prefix() {
    assert_eq!(cover_target_name("F01.webp", "F01", "F99"), "F99.webp");
    assert_eq!(cover_target_name("F01-20260823093000000.webp", "F01", "F99"), "F99-20260823093000000.webp");
    assert_eq!(cover_target_name("随手.webp", "F01", "F99"), "随手.webp");
  }
}
