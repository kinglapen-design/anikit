//! 删除 Placement（6.1，两段式）：
//! 第一次调用（lastAction = None）若涉及"资源最后一条 Placement"则只返回
//! needConfirm + lastAssets 不动数据；前端弹询问后带 lastAction 再调——
//! "recycle" = 全部 Placement 移除 + recycle.json 记录（info 原地不动，PRD 06-06）；
//! "uncategorized" = 这些资源的 Placement 改挂 root（未分类）；非最后一条的直接移除。
//! 删到 origin Placement 而资产仍有剩余 Placement 时，origin 转给剩余中顺序最前的一条
//! （PRD 01 边界规则 8）。

use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::core::backup;
use crate::core::library::{self, Library};
use crate::core::models::{Placement, RecycleItem};
use crate::services::import::ROOT_FOLDER_ID;

use super::{find_placement, finish, now, rewrite_orders, PlacementRef};

/// 大批量删除阈值：达到先 backup_library（失败仅警告不阻断，参照 library.rs 打开自检策略）
pub const DELETE_BACKUP_THRESHOLD: usize = 50;

/// 删除结果：needConfirm = true 时未动任何数据，lastAssets 为"删的是其最后一条 Placement"的资源
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteReport {
  pub need_confirm: bool,
  #[serde(default)]
  pub last_assets: Vec<String>,
}

pub fn delete_placements(lib: &mut Library, items: &[PlacementRef], last_action: Option<&str>) -> Result<DeleteReport, String> {
  match last_action {
    None | Some("recycle") | Some("uncategorized") => {}
    Some(other) => return Err(format!("未知的删除方式：{other}（可选 recycle / uncategorized）")),
  }
  // 匹配下标（陈旧三元组跳过，重复选择去重）
  let mut matched: Vec<usize> = Vec::new();
  let mut seen = HashSet::new();
  for it in items {
    if let Some(pos) = find_placement(lib, it) {
      if seen.insert(pos) {
        matched.push(pos);
      }
    }
  }
  if matched.is_empty() {
    return Ok(DeleteReport { need_confirm: false, last_assets: Vec::new() });
  }
  // "本次删除的是其最后一条 Placement"的资源集合
  let mut total: HashMap<&str, usize> = HashMap::new();
  for p in &lib.folders.items {
    *total.entry(p.asset.as_str()).or_default() += 1;
  }
  let mut deleting: HashMap<&str, usize> = HashMap::new();
  for &i in &matched {
    *deleting.entry(lib.folders.items[i].asset.as_str()).or_default() += 1;
  }
  let mut last_assets: Vec<String> = deleting
    .iter()
    .filter(|(a, n)| total.get(**a).copied().unwrap_or(0) == **n)
    .map(|(a, _)| a.to_string())
    .collect();
  last_assets.sort();
  if last_action.is_none() && !last_assets.is_empty() {
    return Ok(DeleteReport { need_confirm: true, last_assets });
  }

  // 大批量写前备份（防灾；备份失败仅警告不阻断）
  if matched.len() >= DELETE_BACKUP_THRESHOLD {
    if let Err(e) = backup::backup_library(&lib.root, &lib.assets_dir) {
      log::warn!("删除前备份失败（不阻断删除）：{e}");
    }
  }

  let matched_set: HashSet<usize> = matched.iter().copied().collect();
  let last_set: HashSet<&str> = last_assets.iter().map(String::as_str).collect();
  // origin 转移候选：本次涉及但非"最后一条"的资产（last_set 资产删完进回收站，无剩余不涉及）。
  // 所有权化——retain 之后 items 变动，借用不可再持有
  let transfer_candidates: HashSet<String> =
    matched.iter().map(|&i| lib.folders.items[i].asset.clone()).filter(|a| !last_set.contains(a.as_str())).collect();
  let mut keep_in_root: HashSet<usize> = HashSet::new();
  let mut recycled = false;
  match last_action {
    Some("recycle") => {
      for asset in &last_assets {
        let placements: Vec<Placement> =
          lib.folders.items.iter().filter(|p| p.asset == *asset).cloned().collect();
        lib.recycle.items.push(RecycleItem { asset: asset.clone(), placements, deleted_at: now() });
      }
      recycled = !last_assets.is_empty();
    }
    Some("uncategorized") => {
      // 最后一条 Placement 不删，改挂 root（未分类）；root 内显示名唯一化
      let mut existing: HashSet<String> = lib
        .folders
        .items
        .iter()
        .enumerate()
        .filter(|(i, p)| !matched_set.contains(i) && p.folder == ROOT_FOLDER_ID)
        .map(|(_, p)| p.name.clone())
        .collect();
      for &i in &matched {
        if last_set.contains(lib.folders.items[i].asset.as_str()) {
          keep_in_root.insert(i);
          let p = &mut lib.folders.items[i];
          p.folder = ROOT_FOLDER_ID.to_string();
          p.name = library::unique_name(&existing, &p.name);
          existing.insert(p.name.clone());
        }
      }
    }
    _ => {}
  }
  let mut i = 0;
  lib.folders.items.retain(|_| {
    let keep = !matched_set.contains(&i) || keep_in_root.contains(&i);
    i += 1;
    keep
  });
  // origin 转移（PRD 01 边界规则 8，2026-08-04 定 / 2026-08-13 落地）：本次删掉了某资产的
  // origin Placement 且该资产仍有剩余 Placement 时，origin 转给剩余中顺序最前的一条。
  // "顺序最前"定义：同文件夹内 items vec 顺序即 order 升序（add_placement 追加 /
  // rewrite_orders 重写维护该不变量），跨文件夹无全局顺序，故统一取 items vec 顺序最前者
  if !transfer_candidates.is_empty() {
    let has_origin: HashSet<String> = lib
      .folders
      .items
      .iter()
      .filter(|p| p.origin && transfer_candidates.contains(p.asset.as_str()))
      .map(|p| p.asset.clone())
      .collect();
    let mut transferred: HashSet<String> = HashSet::new();
    for p in &mut lib.folders.items {
      if !p.origin
        && transfer_candidates.contains(p.asset.as_str())
        && !has_origin.contains(p.asset.as_str())
        && transferred.insert(p.asset.clone())
      {
        p.origin = true;
      }
    }
  }
  if !keep_in_root.is_empty() {
    rewrite_orders(lib, ROOT_FOLDER_ID);
  }
  if recycled {
    library::save_recycle(lib).map_err(|e| e.to_string())?;
  }
  finish(lib)?;
  log::info!("删除 {} 个项目（回收站 {recycled}）", matched_set.len());
  Ok(DeleteReport { need_confirm: false, last_assets: Vec::new() })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::json_io::read_json;
  use crate::core::library::{FOLDERS_FILE, RECYCLE_FILE};
  use crate::core::models::{AssetMeta, Folder, FoldersFile, RecycleFile};
  use crate::services::import as ops;
  use crate::services::import::{PendingFile, Source};

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
    let folder = Folder { id: id.into(), name: id.into(), parent: None, order: 0 };
    lib.folders.folders.push(folder.clone());
    lib.index.folders.insert(id.into(), folder);
  }

  fn add_asset(h: &mut Harness, id: &str, file: &str, folder: &str) {
    let src = h.dir.path().join(file);
    std::fs::write(&src, b"fake").unwrap();
    let pf = PendingFile { path: src, chain: Vec::new() };
    let meta = ops::build_asset_with_id(&h.lib.assets_dir, &pf, "sha256:t", id, Source::Copy, None).unwrap();
    h.lib.index.assets.insert(id.into(), meta);
    ops::add_placement(&mut h.lib, id.into(), folder.into(), file.into(), true, &mut HashMap::new());
    finish(&mut h.lib).unwrap();
  }

  fn pref(asset: &str, folder: &str, origin: bool) -> PlacementRef {
    PlacementRef { asset_id: asset.into(), folder_id: folder.into(), origin }
  }

  fn count(lib: &Library, asset: &str) -> usize {
    lib.folders.items.iter().filter(|p| p.asset == asset).count()
  }

  /// 两段式：无 lastAction 且涉及最后一条 → needConfirm 不动数据
  #[test]
  fn delete_two_phase_need_confirm() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01");
    add_asset(&mut h, "AN-A", "a.png", "F01"); // 仅一条 Placement
    add_asset(&mut h, "AN-B", "b.png", "root");
    ops::add_placement(&mut h.lib, "AN-B".into(), "F01".into(), "b链接.png".into(), false, &mut HashMap::new());
    finish(&mut h.lib).unwrap();

    let rep = delete_placements(&mut h.lib, &[pref("AN-A", "F01", true), pref("AN-B", "root", true)], None).unwrap();
    assert!(rep.need_confirm);
    assert_eq!(rep.last_assets, vec!["AN-A"], "AN-B 在 F01 还有一条，不算最后");
    assert_eq!((count(&h.lib, "AN-A"), count(&h.lib, "AN-B")), (1, 2), "未动数据");

    // 非最后一条：不需确认直接删
    let rep2 = delete_placements(&mut h.lib, &[pref("AN-B", "F01", false)], None).unwrap();
    assert!(!rep2.need_confirm);
    assert_eq!(count(&h.lib, "AN-B"), 1);
    // 未知删除方式拒绝
    assert!(delete_placements(&mut h.lib, &[pref("AN-B", "root", true)], Some("销毁")).is_err());
  }

  /// origin 转移（PRD 01 边界规则 8）：删 origin 剩一条链接 → 链接转 origin
  #[test]
  fn delete_origin_transfers_to_single_remaining_link() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01");
    add_folder(&mut h.lib, "F02");
    add_asset(&mut h, "AN-A", "a.png", "F01"); // origin
    ops::add_placement(&mut h.lib, "AN-A".into(), "F02".into(), "链接.png".into(), false, &mut HashMap::new());
    finish(&mut h.lib).unwrap();

    // 删 origin（还有链接，非最后一条，无需确认）
    let rep = delete_placements(&mut h.lib, &[pref("AN-A", "F01", true)], None).unwrap();
    assert!(!rep.need_confirm);
    let remaining: Vec<&Placement> = h.lib.folders.items.iter().filter(|p| p.asset == "AN-A").collect();
    assert_eq!(remaining.len(), 1);
    assert!(remaining[0].origin, "唯一剩余链接转 origin");
    assert_eq!(remaining[0].folder, "F02");
    // 索引反查同步（finish 重建）
    assert_eq!(h.lib.index.placements_by_asset["AN-A"].iter().filter(|p| p.origin).count(), 1);
  }

  /// origin 转移：剩多条 → 顺序最前（items vec 顺序）者转 origin
  #[test]
  fn delete_origin_transfers_to_frontmost_of_multiple() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01");
    add_folder(&mut h.lib, "F02");
    add_asset(&mut h, "AN-A", "a.png", "F01"); // origin
    ops::add_placement(&mut h.lib, "AN-A".into(), "F02".into(), "链接1.png".into(), false, &mut HashMap::new());
    ops::add_placement(&mut h.lib, "AN-A".into(), "root".into(), "链接2.png".into(), false, &mut HashMap::new());
    finish(&mut h.lib).unwrap();

    let rep = delete_placements(&mut h.lib, &[pref("AN-A", "F01", true)], None).unwrap();
    assert!(!rep.need_confirm);
    let remaining: Vec<&Placement> = h.lib.folders.items.iter().filter(|p| p.asset == "AN-A").collect();
    assert_eq!(remaining.len(), 2);
    assert_eq!(remaining[0].folder, "F02");
    assert!(remaining[0].origin, "vec 顺序最前者转 origin");
    assert!(!remaining[1].origin, "origin 唯一");
    assert!(h.lib.recycle.items.is_empty(), "不进回收站");
  }

  /// 删除最后一条（origin）→ 进回收站，无剩余不转移；recycle.json 记 origin = true
  #[test]
  fn delete_last_origin_recycles_without_transfer() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01");
    add_asset(&mut h, "AN-A", "a.png", "F01");

    let rep = delete_placements(&mut h.lib, &[pref("AN-A", "F01", true)], Some("recycle")).unwrap();
    assert!(!rep.need_confirm);
    assert_eq!(count(&h.lib, "AN-A"), 0);
    assert_eq!(h.lib.recycle.items.len(), 1);
    assert!(h.lib.recycle.items[0].placements[0].origin, "回收站快照保留 origin 标记");
  }

  #[test]
  fn delete_recycle_moves_to_recycle_json() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01");
    add_asset(&mut h, "AN-A", "a.png", "F01");

    let rep = delete_placements(&mut h.lib, &[pref("AN-A", "F01", true)], Some("recycle")).unwrap();
    assert!(!rep.need_confirm);
    assert_eq!(count(&h.lib, "AN-A"), 0, "Placement 移除");
    // recycle.json 记录删除前全部 Placement（还原依据）；info 原地不动
    assert_eq!(h.lib.recycle.items.len(), 1);
    assert_eq!(h.lib.recycle.items[0].asset, "AN-A");
    assert_eq!(h.lib.recycle.items[0].placements.len(), 1);
    let disk: RecycleFile = read_json(&h.lib.root.join(RECYCLE_FILE)).unwrap();
    assert_eq!(disk.items.len(), 1, "recycle.json 落盘");
    assert!(h.lib.info_dir("AN-A").is_dir(), "info 原地不动");
    let folders: FoldersFile = read_json(&h.lib.root.join(FOLDERS_FILE)).unwrap();
    assert!(folders.items.is_empty(), "folders.json 落盘清理");
  }

  #[test]
  fn delete_uncategorized_keeps_as_root() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01");
    add_asset(&mut h, "AN-A", "a.png", "F01");
    // root 已有同名（不同资源）→ 移入时唯一化
    add_asset(&mut h, "AN-B", "a.png", "root");

    let rep = delete_placements(&mut h.lib, &[pref("AN-A", "F01", true)], Some("uncategorized")).unwrap();
    assert!(!rep.need_confirm);
    let p = h.lib.folders.items.iter().find(|p| p.asset == "AN-A").unwrap();
    assert_eq!(p.folder, "root", "保留为未分类");
    assert_eq!(p.name, "a-1.png", "root 内显示名唯一化");
    let orders: Vec<i32> = h.lib.folders.items.iter().filter(|p| p.folder == "root").map(|p| p.order).collect();
    assert_eq!(orders, vec![0, 1], "root 段 order 重写连续");
    assert!(h.lib.recycle.items.is_empty(), "不进回收站");
    let meta: AssetMeta = read_json(&h.lib.info_dir("AN-A").join("metadata.json")).unwrap();
    assert_eq!(meta.id, "AN-A", "资源实体不受影响");
  }
}
