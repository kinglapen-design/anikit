//! 回收站（6.1 第二轮）：视图 / 还原 / 彻底删除 / 清空。
//! - 还原：RecycleItem.placements 写回 folders.json（目标文件夹已不存在挂 root、
//!   同文件夹显示名 unique_name）+ recycle.json 移除条目；撤销三步由命令层接入。
//! - 彻底删除 / 清空：物理删 {ID}.info 整目录，不可逆——**不接撤销**，
//!   操作前 backup_library 兜底（失败仅警告不阻断，参照 delete.rs 大批量删除策略）；
//!   同步清理 index / recycle.json / recent.json / pins.json 引用。

use std::collections::{HashMap, HashSet};
use std::fs;

use serde::Serialize;

use crate::core::backup;
use crate::core::library::{self, Library};
use crate::core::models::{Placement, RecycleItem};
use crate::services::import::ROOT_FOLDER_ID;
use crate::services::operations::{finish, rewrite_orders};
use crate::services::recent::remove_recent;
use crate::services::views::{placement_view, PlacementView};

/// RecycleEntryView.folderId 伪值（回收站不是 folders.json 里的真实文件夹）
pub const RECYCLE_FOLDER_ID: &str = "recycle";

/// 命令 list_recycle 输出：PlacementView 同构（folderId 恒 "recycle"、origin 恒 false）
/// + deletedAt + placementCount
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecycleEntryView {
  #[serde(flatten)]
  pub view: PlacementView,
  pub deleted_at: String,
  pub placement_count: u32,
}

/// 回收站列表：deletedAt 倒序；资产实体已不在索引（脏数据）跳过
pub fn list_recycle(lib: &Library) -> Vec<RecycleEntryView> {
  let mut items = lib.recycle.items.clone();
  items.sort_by(|a, b| b.deleted_at.cmp(&a.deleted_at));
  items.iter().filter_map(|i| entry_view(lib, i)).collect()
}

fn entry_view(lib: &Library, item: &RecycleItem) -> Option<RecycleEntryView> {
  let meta = lib.index.assets.get(&item.asset)?;
  let name = item.placements.first().map(|p| p.name.clone()).unwrap_or_else(|| meta.name.clone());
  let pseudo =
    Placement { asset: item.asset.clone(), folder: RECYCLE_FOLDER_ID.to_string(), name, order: 0, origin: false };
  Some(RecycleEntryView {
    view: placement_view(lib, meta, &pseudo, &lib.format_icons),
    deleted_at: item.deleted_at.clone(),
    placement_count: item.placements.len() as u32,
  })
}

/// 还原：写回记录的全部 Placement 并从回收站移除，返回还原的资源数（陈旧 ID 跳过不计）；
/// 显示名原样写回不唯一化（2026-08-19 撞名放开）
pub fn restore_recycle(lib: &mut Library, asset_ids: &[String]) -> Result<usize, String> {
  let want: HashSet<&str> = asset_ids.iter().map(String::as_str).collect();
  let mut orders: HashMap<String, i32> = HashMap::new();
  let mut affected: HashSet<String> = HashSet::new();
  let mut remaining = Vec::new();
  let mut restored = 0usize;
  for item in std::mem::take(&mut lib.recycle.items) {
    if want.contains(item.asset.as_str()) && lib.index.assets.contains_key(&item.asset) {
      restore_one(lib, &item, &mut orders, &mut affected);
      restored += 1;
    } else {
      remaining.push(item);
    }
  }
  lib.recycle.items = remaining;
  if restored == 0 {
    return Ok(0);
  }
  for folder in &affected {
    rewrite_orders(lib, folder);
  }
  finish(lib)?;
  library::save_recycle(lib).map_err(|e| e.to_string())?;
  log::info!("还原 {restored} 个回收站项目");
  Ok(restored)
}

/// 单条 RecycleItem 的 Placement 写回：目标文件夹已不存在挂 root；**显示名原样写回不唯一化**
///（撞名放开，2026-08-19 用户定"护栏拆干净"——旧同文件夹 unique_name 废除）；
/// 追加到目标文件夹段尾（orders 按需懒建，跨条目累计保持段尾连续）
fn restore_one(
  lib: &mut Library,
  item: &RecycleItem,
  orders: &mut HashMap<String, i32>,
  affected: &mut HashSet<String>,
) {
  for p in &item.placements {
    let folder = if p.folder == ROOT_FOLDER_ID || lib.index.folders.contains_key(&p.folder) {
      p.folder.clone()
    } else {
      ROOT_FOLDER_ID.to_string()
    };
    let next = orders.entry(folder.clone()).or_insert_with(|| {
      lib.folders.items.iter().filter(|q| q.folder == folder).map(|q| q.order).max().map_or(0, |m| m + 1)
    });
    let order = *next;
    *next += 1;
    lib.folders.items.push(Placement { asset: item.asset.clone(), folder: folder.clone(), name: p.name.clone(), order, origin: p.origin });
    affected.insert(folder);
  }
}

/// 彻底删除结果：needConfirm = true 时未动任何数据，segmentRefs 为引用被删集合的虚拟片段数，
/// linkRefs 为库内**活链接**数（2026-08-19 位置级模型：本体在回收站时链接可以活着挂在目录树，
/// 物理删除本体即全部随清——用户定"彻底清除前提示关联链接一并清除"）
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteRecycleReport {
  pub need_confirm: bool,
  #[serde(default)]
  pub segment_refs: usize,
  #[serde(default)]
  pub link_refs: usize,
  #[serde(default)]
  pub deleted: usize,
}

/// 彻底删除（两段式；2026-08-19 位置级模型分级）：
/// - 快照含 origin 的条目 = 资产本体在回收站 → 物理删 info + 库内该资产全部活位置（含链接）清除
///   + index / recycle / recent / pins 同步；本体在库内的活链接必随之死，故计数进 linkRefs；
/// - 纯链接位置记录（无 origin 快照）= 只删回收站条目，物理不动、无需确认。
/// 未确认只返回 needConfirm + 计数不动数据（仅"本体在回收站且有引用/活链接"时才需要确认）
pub fn delete_recycle(lib: &mut Library, asset_ids: &[String], confirmed: bool) -> Result<DeleteRecycleReport, String> {
  let want: HashSet<&str> = asset_ids.iter().map(String::as_str).collect();
  let targets: Vec<RecycleItem> = lib.recycle.items.iter().filter(|i| want.contains(i.asset.as_str())).cloned().collect();
  if targets.is_empty() {
    return Ok(DeleteRecycleReport { need_confirm: false, segment_refs: 0, link_refs: 0, deleted: 0 });
  }
  let target_set: HashSet<&str> = targets.iter().map(|i| i.asset.as_str()).collect();
  // 本体在回收站的资产集（快照含 origin）；纯位置记录不进物理段
  let purge_set: HashSet<&str> = targets
    .iter()
    .filter(|i| i.placements.iter().any(|p| p.origin))
    .map(|i| i.asset.as_str())
    .collect();
  let segment_refs = count_segment_refs(lib, &purge_set);
  // 活链接 = 库内 folders.items 里本体将删资产的 origin=false 位置（物理删后必随清）
  let link_refs: usize = lib
    .folders
    .items
    .iter()
    .filter(|p| !p.origin && purge_set.contains(p.asset.as_str()))
    .count();
  if (segment_refs > 0 || link_refs > 0) && !confirmed {
    return Ok(DeleteRecycleReport { need_confirm: true, segment_refs, link_refs, deleted: 0 });
  }
  // 物理不可逆：操作前强制备份兜底（失败仅警告不阻断）
  if !purge_set.is_empty() {
    if let Err(e) = backup::backup_library(&lib.root, &lib.assets_dir) {
      log::warn!("彻底删除前备份失败（不阻断删除）：{e}");
    }
  }
  let mut removed: Vec<String> = Vec::new();
  for &id in &purge_set {
    let info = lib.info_dir(id);
    if info.exists() {
      if let Err(e) = fs::remove_dir_all(&info) {
        // 半途失败：已删的部分先同步数据文件保持一致，再报错
        partial_sync(lib, &target_set, &removed)?;
        let name = lib.index.assets.get(id).map(|m| m.name.clone()).unwrap_or_else(|| id.to_string());
        return Err(format!("删除「{name}」的文件失败：{e}"));
      }
    }
    removed.push(id.to_string());
  }
  partial_sync(lib, &target_set, &removed)?;
  log::info!("彻底删除 {} 个回收站项目（物理删除 {}，虚拟片段引用 {segment_refs}，活链接 {link_refs}）", targets.len(), removed.len());
  Ok(DeleteRecycleReport { need_confirm: false, segment_refs, link_refs, deleted: targets.len() })
}

/// 删除后同步：库内活位置清除（本体被物理删的资产，其链接随清）+ 回收站条目移除 +
/// index / recent / pins 收尾（sync_after_purge）
fn partial_sync(lib: &mut Library, target_set: &HashSet<&str>, removed: &[String]) -> Result<(), String> {
  if !removed.is_empty() {
    let purge_set: HashSet<&str> = removed.iter().map(String::as_str).collect();
    lib.folders.items.retain(|p| !purge_set.contains(p.asset.as_str()));
    finish(lib)?;
  }
  lib.recycle.items.retain(|i| !target_set.contains(i.asset.as_str()));
  library::save_recycle(lib).map_err(|e| e.to_string())?;
  sync_after_purge(lib, removed)
}

/// 清空回收站：等价于对全部条目执行 delete_recycle（segmentRefs 对全集汇总）
pub fn clear_recycle(lib: &mut Library, confirmed: bool) -> Result<DeleteRecycleReport, String> {
  let ids: Vec<String> = lib.recycle.items.iter().map(|i| i.asset.clone()).collect();
  delete_recycle(lib, &ids, confirmed)
}

/// 虚拟片段引用计数：库内不在删除集合内的资产 type="segment" 且 source 指向删除集合
/// pub(crate)：purge.rs（素材/文件夹彻底删除）复用同一判定
pub(crate) fn count_segment_refs(lib: &Library, target_set: &HashSet<&str>) -> usize {
  lib
    .index
    .assets
    .values()
    .filter(|m| {
      m.r#type.as_deref() == Some("segment")
        && !target_set.contains(m.id.as_str())
        && m.source.as_ref().is_some_and(|s| target_set.contains(s.as_str()))
    })
    .count()
}

/// 物理删除后的数据同步：index 移除 + recycle.json 移除条目 + recent.json 剔除 +
/// pins.json 引用清理（pins 未启用：有引用才剔除落盘，无则不动）
/// pub(crate)：purge.rs（素材/文件夹彻底删除）复用同一收尾
pub(crate) fn sync_after_purge(lib: &mut Library, removed: &[String]) -> Result<(), String> {
  if removed.is_empty() {
    return Ok(());
  }
  let set: HashSet<&str> = removed.iter().map(String::as_str).collect();
  for id in removed {
    lib.index.assets.remove(id);
  }
  lib.recycle.items.retain(|i| !set.contains(i.asset.as_str()));
  library::save_recycle(lib).map_err(|e| e.to_string())?;
  remove_recent(lib, &set)?;
  let before = lib.pins.pins.len();
  lib.pins.pins.retain(|p| !set.contains(p.target.as_str()));
  if lib.pins.pins.len() != before {
    library::save_pins(lib).map_err(|e| e.to_string())?;
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::json_io::read_json;
  use crate::core::library::{FOLDERS_FILE, PINS_FILE, RECENT_FILE, RECYCLE_FILE};
  use crate::core::models::{Folder, FoldersFile, Pin, RecycleFile, RecentFile};
  use crate::core::undo::UndoManager;
  use crate::services::import::{self as ops, PendingFile, Source};
  use crate::services::operations::{delete_placements, PlacementRef};
  use crate::services::recent::touch_recent;

  struct Harness {
    dir: tempfile::TempDir,
    lib: Library,
  }

  fn make_lib() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    Harness { dir, lib }
  }

  fn add_folder(lib: &mut Library, id: &str) {
    let folder = Folder { id: id.into(), name: id.into(), parent: None, block: None, icon: None, kind: None, color: None, order: 0 };
    lib.folders.folders.push(folder.clone());
    lib.index.folders.insert(id.into(), folder);
  }

  fn add_asset(h: &mut Harness, id: &str, file: &str, folder: &str) {
    let src = h.dir.path().join(file);
    fs::write(&src, b"fake").unwrap();
    let pf = PendingFile { path: src, chain: Vec::new() };
    let meta = ops::build_asset_with_id(&h.lib.assets_dir, &pf, "sha256:t", id, Source::Copy, None).unwrap();
    h.lib.index.assets.insert(id.into(), meta);
    ops::add_placement(&mut h.lib, id.into(), folder.into(), file.into(), true, &mut HashMap::new());
    finish(&mut h.lib).unwrap();
  }

  /// 把资源 origin Placement 删进回收站（走删除的 recycle 分支，数据与线上一致；
  /// 2026-08-19 位置级模型：无"最后一条"判定，origin 单独删即进回收站）
  fn recycle_asset(h: &mut Harness, id: &str, folder: &str) {
    let pref = PlacementRef { asset_id: id.into(), folder_id: folder.into(), origin: true };
    delete_placements(&mut h.lib, &[pref], Some("recycle")).unwrap();
  }

  fn info_dir(h: &Harness, id: &str) -> std::path::PathBuf {
    h.lib.info_dir(id)
  }

  #[test]
  fn list_recycle_desc_and_pseudo_fields() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01");
    add_asset(&mut h, "AN-A", "a.png", "F01");
    ops::add_placement(&mut h.lib, "AN-A".into(), "root".into(), "a链接.png".into(), false, &mut HashMap::new());
    finish(&mut h.lib).unwrap();
    // 两条 Placement 一次调用删除：位置级模型进同一条回收站记录（按资产聚合）
    let prefs = vec![
      PlacementRef { asset_id: "AN-A".into(), folder_id: "F01".into(), origin: true },
      PlacementRef { asset_id: "AN-A".into(), folder_id: "root".into(), origin: false },
    ];
    delete_placements(&mut h.lib, &prefs, Some("recycle")).unwrap();

    let list = list_recycle(&h.lib);
    assert_eq!(list.len(), 1);
    let v = &list[0];
    assert_eq!(v.view.folder_id, RECYCLE_FOLDER_ID, "folderId 恒伪值");
    assert!(!v.view.origin, "origin 恒 false");
    assert_eq!(v.view.name, "a.png", "name 取 placements[0].name");
    assert_eq!(v.placement_count, 2, "删除前全部 Placement 计数");
    assert!(!v.deleted_at.is_empty());
  }

  #[test]
  fn restore_recycle_basic_and_undo_rollback() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01");
    add_asset(&mut h, "AN-A", "a.png", "F01");
    recycle_asset(&mut h, "AN-A", "F01");

    // 撤销三步与命令层同约：begin → record_file 两个写点 → 业务 → commit
    let mut undo = UndoManager::default();
    undo.begin("还原 1 个项目");
    undo.record_file(&h.lib.root.join(FOLDERS_FILE)).unwrap();
    undo.record_file(&h.lib.root.join(RECYCLE_FILE)).unwrap();
    let n = restore_recycle(&mut h.lib, &["AN-A".to_string()]).unwrap();
    assert_eq!(n, 1);
    undo.commit().unwrap();

    let p = h.lib.folders.items.iter().find(|p| p.asset == "AN-A").unwrap();
    assert_eq!((p.folder.as_str(), p.name.as_str(), p.origin), ("F01", "a.png", true), "Placement 写回");
    assert!(h.lib.recycle.items.is_empty(), "recycle 条目移除");
    let folders: FoldersFile = read_json(&h.lib.root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(folders.items.len(), 1, "folders.json 落盘");
    let recycle: RecycleFile = read_json(&h.lib.root.join(RECYCLE_FILE)).unwrap();
    assert!(recycle.items.is_empty(), "recycle.json 落盘");

    // 撤销回滚：盘上两文件写回操作前快照，reload 后内存同步
    undo.undo().unwrap();
    library::reload(&mut h.lib).unwrap();
    assert!(h.lib.folders.items.is_empty(), "undo 后 Placement 消失");
    assert_eq!(h.lib.recycle.items.len(), 1, "undo 后回收站条目回来");
    // redo 还原再次生效
    undo.redo().unwrap();
    library::reload(&mut h.lib).unwrap();
    assert_eq!(h.lib.folders.items.len(), 1);
    assert!(h.lib.recycle.items.is_empty());
  }

  #[test]
  fn restore_recycle_missing_folder_falls_back_to_root_keeps_name() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01");
    add_asset(&mut h, "AN-A", "a.png", "F01");
    recycle_asset(&mut h, "AN-A", "F01");
    // 目标文件夹已不存在 → 挂 root
    h.lib.folders.folders.retain(|f| f.id != "F01");
    h.lib.index.folders.remove("F01");
    // root 已有同名（不同资源）→ 撞名放开，原名写回（2026-08-19 用户定"护栏拆干净"）
    add_asset(&mut h, "AN-B", "a.png", "root");

    let n = restore_recycle(&mut h.lib, &["AN-A".to_string()]).unwrap();
    assert_eq!(n, 1);
    let p = h.lib.folders.items.iter().find(|p| p.asset == "AN-A").unwrap();
    assert_eq!(p.folder, ROOT_FOLDER_ID, "文件夹缺失挂 root");
    assert_eq!(p.name, "a.png", "撞名不追加序号（显示名放开）");
    let orders: Vec<i32> = h.lib.folders.items.iter().filter(|p| p.folder == "root").map(|p| p.order).collect();
    assert_eq!(orders, vec![0, 1], "root 段 order 重写连续");
    // 陈旧 ID（不在回收站）跳过不计
    assert_eq!(restore_recycle(&mut h.lib, &["AN-不存在".to_string()]).unwrap(), 0);
  }

  #[test]
  fn delete_recycle_segment_refs_two_phase_then_purge() {
    let mut h = make_lib();
    add_asset(&mut h, "AN-V", "视频.mp4", "root");
    add_asset(&mut h, "AN-SEG", "片段.mp4", "root");
    // AN-SEG 是指向 AN-V 的虚拟片段（PRD 01-2.6）
    let seg = h.lib.index.assets.get_mut("AN-SEG").unwrap();
    seg.r#type = Some("segment".into());
    seg.source = Some("AN-V".into());
    touch_recent(&mut h.lib, "AN-V").unwrap();
    h.lib.pins.pins.push(Pin { target: "AN-V".into(), order: 0 });
    recycle_asset(&mut h, "AN-V", "root");

    // 第一段：有片段引用未确认 → 拦截不动数据
    let rep = delete_recycle(&mut h.lib, &["AN-V".to_string()], false).unwrap();
    assert!(rep.need_confirm);
    assert_eq!(rep.segment_refs, 1);
    assert_eq!(rep.deleted, 0);
    assert!(info_dir(&h, "AN-V").is_dir(), "未确认不动物理文件");
    assert_eq!(h.lib.recycle.items.len(), 1);

    // 第二段：确认 → 物理删除 + 全引用同步清理
    let rep = delete_recycle(&mut h.lib, &["AN-V".to_string()], true).unwrap();
    assert!(!rep.need_confirm);
    assert_eq!(rep.deleted, 1);
    assert!(!info_dir(&h, "AN-V").exists(), "info 整目录物理删除");
    assert!(!h.lib.index.assets.contains_key("AN-V"), "index 移除");
    assert!(h.lib.recycle.items.is_empty(), "recycle 条目移除");
    assert!(h.lib.recent.items.is_empty(), "recent 同步剔除");
    let recent: RecentFile = read_json(&h.lib.root.join(RECENT_FILE)).unwrap();
    assert!(recent.items.is_empty(), "recent.json 落盘");
    assert!(h.lib.pins.pins.is_empty(), "pins 引用剔除");
    let pins: crate::core::models::PinsFile = read_json(&h.lib.root.join(PINS_FILE)).unwrap();
    assert!(pins.pins.is_empty(), "pins.json 落盘");
    // 片段资产本身保留
    assert!(h.lib.index.assets.contains_key("AN-SEG"));
  }

  #[test]
  fn delete_recycle_without_refs_and_stale_ids() {
    let mut h = make_lib();
    add_asset(&mut h, "AN-A", "a.png", "root");
    recycle_asset(&mut h, "AN-A", "root");

    // 无片段引用：不需确认直接删
    let rep = delete_recycle(&mut h.lib, &["AN-A".to_string(), "AN-不存在".to_string()], false).unwrap();
    assert!(!rep.need_confirm);
    assert_eq!((rep.segment_refs, rep.deleted), (0, 1), "陈旧 ID 跳过");
    assert!(!info_dir(&h, "AN-A").exists());
    // 回收站已空：幂等返回 0
    let rep = delete_recycle(&mut h.lib, &["AN-A".to_string()], false).unwrap();
    assert_eq!(rep.deleted, 0);
  }

  /// 链接警示（2026-08-19 位置级模型）：本体（origin）进回收站、链接留库内活着——
  /// 彻底删除 = 物理删本体 + 活链接随清；未确认拦截带 linkRefs（活链接计数）
  #[test]
  fn delete_recycle_warns_live_link_refs() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01");
    add_asset(&mut h, "AN-A", "a.png", "F01"); // origin
    ops::add_placement(&mut h.lib, "AN-A".into(), "root".into(), "链接.png".into(), false, &mut HashMap::new());
    finish(&mut h.lib).unwrap();
    // origin 单独丢回收站（链接留库内活着，不转移）
    recycle_asset(&mut h, "AN-A", "F01");
    assert_eq!(h.lib.folders.items.len(), 1, "链接还活着");

    let rep = delete_recycle(&mut h.lib, &["AN-A".to_string()], false).unwrap();
    assert!(rep.need_confirm);
    assert_eq!((rep.segment_refs, rep.link_refs), (0, 1), "无片段引用、1 条活链接");
    assert!(info_dir(&h, "AN-A").is_dir(), "未确认不动物理文件");
    let rep = delete_recycle(&mut h.lib, &["AN-A".to_string()], true).unwrap();
    assert!(!rep.need_confirm);
    assert_eq!(rep.deleted, 1);
    assert!(!info_dir(&h, "AN-A").exists(), "物理删除");
    assert!(h.lib.folders.items.is_empty(), "活链接随清");
    assert!(!h.lib.index.assets.contains_key("AN-A"), "index 移除");
    assert!(h.lib.recycle.items.is_empty());
  }

  /// 纯链接位置记录（本体在库内活着）：彻底删除只删记录，物理不动、无需确认
  #[test]
  fn delete_recycle_link_only_entry_is_record_only() {
    let mut h = make_lib();
    add_asset(&mut h, "AN-A", "a.png", "root"); // origin 活着在库内
    ops::add_placement(&mut h.lib, "AN-A".into(), "root".into(), "链接.png".into(), false, &mut HashMap::new());
    finish(&mut h.lib).unwrap();
    // 链接单独丢回收站 = 纯位置记录（无 origin 快照）
    delete_placements(
      &mut h.lib,
      &[PlacementRef { asset_id: "AN-A".into(), folder_id: "root".into(), origin: false }],
      Some("recycle"),
    )
    .unwrap();
    assert_eq!(h.lib.recycle.items.len(), 1);

    let rep = delete_recycle(&mut h.lib, &["AN-A".to_string()], false).unwrap();
    assert!(!rep.need_confirm, "纯位置记录不需确认");
    assert_eq!((rep.link_refs, rep.deleted), (0, 1));
    assert!(info_dir(&h, "AN-A").is_dir(), "物理不动");
    assert_eq!(h.lib.folders.items.len(), 1, "库内 origin 不动");
    assert!(h.lib.recycle.items.is_empty(), "记录移除");
  }

  #[test]
  fn clear_recycle_aggregates_segment_refs() {
    let mut h = make_lib();
    add_asset(&mut h, "AN-A", "a.png", "root");
    add_asset(&mut h, "AN-B", "b.png", "root");
    add_asset(&mut h, "AN-SEG", "片段.mp4", "root");
    let seg = h.lib.index.assets.get_mut("AN-SEG").unwrap();
    seg.r#type = Some("segment".into());
    seg.source = Some("AN-B".into());
    recycle_asset(&mut h, "AN-A", "root");
    recycle_asset(&mut h, "AN-B", "root");

    // 汇总：全集里只有 AN-B 被片段引用
    let rep = clear_recycle(&mut h.lib, false).unwrap();
    assert!(rep.need_confirm);
    assert_eq!(rep.segment_refs, 1);
    assert_eq!(h.lib.recycle.items.len(), 2, "未确认不动数据");

    let rep = clear_recycle(&mut h.lib, true).unwrap();
    assert_eq!(rep.deleted, 2);
    assert!(!info_dir(&h, "AN-A").exists() && !info_dir(&h, "AN-B").exists());
    assert!(h.lib.recycle.items.is_empty());
    // 片段指向被删的 AN-B，但片段自身不在删除集合 → 保留
    assert!(h.lib.index.assets.contains_key("AN-SEG"));
  }
}
