//! 彻底删除服务（6.3 右键菜单）：素材直接彻底删除（不经过回收站）、删除文件夹两形态。
//! - delete_assets_permanently：两段式（虚拟片段引用拦截）→ **先物理段后数据段**——逐个删
//!   info（失败收集名单不中断）→ 只清成功项 Placement + 四处同步（index / recycle / recent /
//!   pins，复用 recycle.rs 同收尾）；失败项数据层原样保留可重试（2026-09-03 资产污染修复：
//!   旧序"先清数据后删物理"半途失败产孤儿）；**不接撤销**。
//! - delete_folder(permanent=false)："删除文件夹"——子树 origin Placement 改挂 root（素材进未分类，
//!   2026-07-29 用户定）+ 链接 Placement 随文件夹直接删除（不进回收站，2026-08-12 用户定案，
//!   PRD 02-15）+ 子树文件夹节点删除；仅改数据，**撤销三步由命令层接入**。
//! - delete_folder(permanent=true)："彻底删除文件夹"——两段式 → 子树资产物理全删（同上当先物理
//!   后数据，失败项挪未分类保留）+ 节点删除；**不接撤销**。两形态执行前均 backup_library 兜底
//!   （失败仅警告不阻断）。

use std::collections::HashSet;
use std::fs;

use serde::Serialize;

use crate::core::backup;
use crate::core::library::Library;
use crate::services::import::ROOT_FOLDER_ID;
use crate::services::operations::{finish, rewrite_orders, sync_folder_index};
use crate::services::recycle::{count_segment_refs, sync_after_purge, DeleteRecycleReport};

/// 素材直接彻底删除（两段式）：库内其他资产虚拟片段引用被删集合时未确认只拦截；
/// 确认后移除全部 Placement + 物理删 info + 四处同步清理。返回与回收站彻底删除同构报告
pub fn delete_assets_permanently(lib: &mut Library, asset_ids: &[String], confirmed: bool) -> Result<DeleteRecycleReport, String> {
  let targets: Vec<String> = asset_ids.iter().filter(|id| lib.index.assets.contains_key(id.as_str())).cloned().collect();
  if targets.is_empty() {
    return Ok(DeleteRecycleReport { need_confirm: false, segment_refs: 0, link_refs: 0, deleted: 0 });
  }
  let target_set: HashSet<&str> = targets.iter().map(String::as_str).collect();
  let segment_refs = count_segment_refs(lib, &target_set);
  if segment_refs > 0 && !confirmed {
    return Ok(DeleteRecycleReport { need_confirm: true, segment_refs, link_refs: 0, deleted: 0 });
  }
  if let Err(e) = backup::backup_library(&lib.root, &lib.assets_dir) {
    log::warn!("彻底删除前备份失败（不阻断删除）：{e}");
  }
  // 物理段先行（2026-09-03 资产污染修复：旧序"先清数据后删物理"半途失败会把未删资产的
  // Placement 一并冲掉产孤儿）：逐个删除，失败收集名单不中断；数据段只清成功项，
  // 失败项 Placement / index / recent 原样保留（库内可见、可重试）。
  // 崩溃窗口（info 已删 Placement 未清）由开库自检 prune_missing_placements 兜底。
  let mut removed: Vec<String> = Vec::new();
  let mut failed: Vec<String> = Vec::new();
  for id in &targets {
    let info = lib.info_dir(id);
    if info.exists() {
      let name = lib.index.assets.get(id).map(|m| m.name.clone()).unwrap_or_else(|| id.clone());
      if let Err(e) = fs::remove_dir_all(&info) {
        log::warn!("彻底删除「{name}」的文件失败（{e}），该项保留在库中");
        failed.push(name);
        continue;
      }
    }
    removed.push(id.clone());
  }
  // 数据段：只清已成功删除项（Placement / index / recycle / recent / pins / 伴生引用级联）
  if !removed.is_empty() {
    let removed_set: HashSet<&str> = removed.iter().map(String::as_str).collect();
    lib.folders.items.retain(|p| !removed_set.contains(p.asset.as_str()));
    finish(lib)?;
    sync_after_purge(lib, &removed)?;
  }
  if !failed.is_empty() {
    return Err(format!(
      "{} 个素材的文件删除失败（可能被 Animate 等程序占用）：{}。其余已删除，失败项保留在库中，可关闭占用程序后重试",
      failed.len(),
      failed.join("、")
    ));
  }
  log::info!("彻底删除 {} 个素材（虚拟片段引用 {segment_refs}）", removed.len());
  Ok(DeleteRecycleReport { need_confirm: false, segment_refs, link_refs: 0, deleted: removed.len() })
}

/// 删除文件夹报告（两分支各取所需字段）
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteFolderReport {
  pub need_confirm: bool,
  #[serde(default)]
  pub segment_refs: usize,
  #[serde(default)]
  pub asset_count: usize,
  #[serde(default)]
  pub folder_count: usize,
  #[serde(default)]
  pub moved_placements: usize,
  #[serde(default)]
  pub deleted_folders: usize,
  #[serde(default)]
  pub deleted: usize,
}

fn empty_report() -> DeleteFolderReport {
  DeleteFolderReport {
    need_confirm: false,
    segment_refs: 0,
    asset_count: 0,
    folder_count: 0,
    moved_placements: 0,
    deleted_folders: 0,
    deleted: 0,
  }
}

/// 子树文件夹 ID 集（含自身，BFS）
fn subtree_ids(lib: &Library, folder_id: &str) -> Vec<String> {
  let mut out = vec![folder_id.to_string()];
  let mut i = 0;
  while i < out.len() {
    for f in &lib.folders.folders {
      if f.parent.as_deref() == Some(out[i].as_str()) && !out.contains(&f.id) {
        out.push(f.id.clone());
      }
    }
    i += 1;
  }
  out
}

/// 删除文件夹两形态（见文件头注释）；非 permanent 单例 = delete_folders 特例（委托）
pub fn delete_folder(lib: &mut Library, folder_id: &str, permanent: bool, confirmed: bool) -> Result<DeleteFolderReport, String> {
  if !lib.index.folders.contains_key(folder_id) {
    return Err("目标文件夹不存在，可能已被删除".to_string());
  }
  if !permanent {
    return delete_folders(lib, std::slice::from_ref(&folder_id.to_string()));
  }

  // "彻底删除文件夹"：子树资产物理全删（两段式）
  let subtree = subtree_ids(lib, folder_id);
  let subtree_set: HashSet<&str> = subtree.iter().map(String::as_str).collect();
  let folder_count = subtree.len();

  let mut asset_set: HashSet<String> = HashSet::new();
  for p in &lib.folders.items {
    if subtree_set.contains(p.folder.as_str()) {
      asset_set.insert(p.asset.clone());
    }
  }
  let asset_refs: HashSet<&str> = asset_set.iter().map(String::as_str).collect();
  let segment_refs = count_segment_refs(lib, &asset_refs);
  if segment_refs > 0 && !confirmed {
    return Ok(DeleteFolderReport { need_confirm: true, segment_refs, asset_count: asset_set.len(), folder_count, ..empty_report() });
  }
  if let Err(e) = backup::backup_library(&lib.root, &lib.assets_dir) {
    log::warn!("彻底删除文件夹前备份失败（不阻断删除）：{e}");
  }
  // 物理段先行（2026-09-03 资产污染修复，同 delete_assets_permanently 口径）：
  // 成功项进数据段全清，失败项保留（子树内 Placement 随文件夹删除挪未分类、子树外链接不动——本体还在）
  let mut removed: Vec<String> = Vec::new();
  let mut failed: Vec<String> = Vec::new();
  let mut failed_ids: HashSet<String> = HashSet::new();
  for id in &asset_set {
    let info = lib.info_dir(id);
    if info.exists() {
      let name = lib.index.assets.get(id).map(|m| m.name.clone()).unwrap_or_else(|| id.clone());
      if let Err(e) = fs::remove_dir_all(&info) {
        log::warn!("彻底删除「{name}」的文件失败（{e}），该项移入未分类保留");
        failed.push(name);
        failed_ids.insert(id.clone());
        continue;
      }
    }
    removed.push(id.clone());
  }
  // 数据段：成功项全部 Placement 清除（含他处链接——本体已死）+ 失败项子树内 Placement 挪未分类
  // + 子树文件夹节点删除
  let removed_set: HashSet<&str> = removed.iter().map(String::as_str).collect();
  let mut kept = Vec::with_capacity(lib.folders.items.len());
  for mut p in std::mem::take(&mut lib.folders.items) {
    if removed_set.contains(p.asset.as_str()) {
      continue;
    }
    if failed_ids.contains(p.asset.as_str()) && subtree_set.contains(p.folder.as_str()) {
      p.folder = ROOT_FOLDER_ID.to_string();
    }
    kept.push(p);
  }
  lib.folders.items = kept;
  lib.folders.folders.retain(|f| !subtree_set.contains(f.id.as_str()));
  sync_folder_index(lib);
  if !failed_ids.is_empty() {
    rewrite_orders(lib, ROOT_FOLDER_ID);
  }
  finish(lib)?;
  sync_after_purge(lib, &removed)?;
  if !failed.is_empty() {
    return Err(format!(
      "{} 个素材的文件删除失败（可能被 Animate 等程序占用）：{}。其余已删除，失败项已移入未分类保留，可关闭占用程序后重试",
      failed.len(),
      failed.join("、")
    ));
  }
  log::info!("彻底删除文件夹 {folder_id}（{} 个素材物理删除，{folder_count} 个节点）", removed.len());
  Ok(DeleteFolderReport {
    segment_refs,
    asset_count: asset_set.len(),
    folder_count,
    deleted_folders: folder_count,
    deleted: removed.len(),
    ..empty_report()
  })
}

/// 批量删除文件夹（3.4，2026-08-18，仅非 permanent 形态）：一次确认单趟执行——
/// 选中集先剔除"祖先同批入选"的后代（删祖先已连带子树，再删后代必报不存在），
/// 并集子树一趟改完只 finish 一次（命令层单撤销步的对应物）；
/// 陈旧 id（选中后已被删）剔除容错，全部不存在才报错（单例委托本函数，语义与原版一致）。
/// 业务口径：子树全部 Placement 改挂 root（素材进未分类，**origin / 链接一视同仁、名称与
/// origin 标记不动**——2026-08-19 位置级模型，"链接随文件夹删"旧口径废除）+ 节点删除
pub fn delete_folders(lib: &mut Library, folder_ids: &[String]) -> Result<DeleteFolderReport, String> {
  // 存在性过滤 + 保序去重
  let mut ids: Vec<&str> = Vec::new();
  for id in folder_ids {
    if lib.index.folders.contains_key(id.as_str()) && !ids.contains(&id.as_str()) {
      ids.push(id.as_str());
    }
  }
  if ids.is_empty() {
    return Err("目标文件夹不存在，可能已被删除".to_string());
  }
  // 根级去重：任意祖先同批入选 → 剔除（沿 parent 链上溯命中集合即后代）
  let parent_of = |id: &str| -> Option<String> {
    lib.folders.folders.iter().find(|f| f.id == id).and_then(|f| f.parent.clone())
  };
  let id_set: HashSet<&str> = ids.iter().copied().collect();
  let roots: Vec<&str> = ids
    .iter()
    .copied()
    .filter(|id| {
      let mut cur = parent_of(id);
      while let Some(p) = cur {
        if id_set.contains(p.as_str()) {
          return false;
        }
        cur = parent_of(&p);
      }
      true
    })
    .collect();
  // 并集子树（含各根级自身）
  let mut subtree_set: HashSet<String> = HashSet::new();
  for id in &roots {
    subtree_set.extend(subtree_ids(lib, id));
  }
  let folder_count = subtree_set.len();
  let subtree_ref: HashSet<&str> = subtree_set.iter().map(String::as_str).collect();

  let mut moved = 0usize;
  let mut kept = Vec::with_capacity(lib.folders.items.len());
  for mut p in std::mem::take(&mut lib.folders.items) {
    if subtree_ref.contains(p.folder.as_str()) {
      // origin / 链接同路径回未分类（2026-08-19 位置级模型）；名称 / origin 标记不动
      p.folder = ROOT_FOLDER_ID.to_string();
      moved += 1;
    }
    kept.push(p);
  }
  lib.folders.items = kept;
  lib.folders.folders.retain(|f| !subtree_ref.contains(f.id.as_str()));
  sync_folder_index(lib);
  rewrite_orders(lib, ROOT_FOLDER_ID);
  finish(lib)?;
  log::info!("删除 {} 个文件夹（{moved} 个素材移入未分类，{folder_count} 个文件夹节点）", roots.len());
  Ok(DeleteFolderReport { moved_placements: moved, deleted_folders: folder_count, ..empty_report() })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::json_io::read_json;
  use crate::core::library;
  use crate::core::library::{FOLDERS_FILE, PINS_FILE, RECENT_FILE};
  use crate::core::models::{Folder, FoldersFile, Pin, PinsFile, RecentFile};
  use crate::services::import::{self as ops, PendingFile, Source};
  use crate::services::recent::touch_recent;
  use std::collections::HashMap;

  struct Harness {
    dir: tempfile::TempDir,
    lib: Library,
  }

  fn make_lib() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    Harness { dir, lib }
  }

  fn add_folder(lib: &mut Library, id: &str, parent: Option<&str>) {
    let folder = Folder { id: id.into(), name: id.into(), parent: parent.map(|p| p.to_string()), block: None, icon: None, kind: None, color: None, groups: None, order: 0 };
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

  fn info_dir(h: &Harness, id: &str) -> std::path::PathBuf {
    h.lib.info_dir(id)
  }

  #[test]
  fn delete_assets_permanently_two_phase_and_cleanup() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", None);
    add_asset(&mut h, "AN-V", "视频.mp4", "F01");
    add_asset(&mut h, "AN-SEG", "片段.mp4", "root");
    let seg = h.lib.index.assets.get_mut("AN-SEG").unwrap();
    seg.r#type = Some("segment".into());
    seg.source = Some("AN-V".into());
    touch_recent(&mut h.lib, "AN-V").unwrap();
    h.lib.pins.pins.push(Pin { target: "AN-V".into(), order: 0 });

    // 第一段：有引用未确认 → 拦截不动数据
    let rep = delete_assets_permanently(&mut h.lib, &["AN-V".into()], false).unwrap();
    assert!(rep.need_confirm && rep.segment_refs == 1 && rep.deleted == 0);
    assert!(info_dir(&h, "AN-V").is_dir());
    assert_eq!(h.lib.folders.items.len(), 2);

    // 第二段：确认 → Placement 移除 + 物理删除 + 四处同步
    let rep = delete_assets_permanently(&mut h.lib, &["AN-V".into()], true).unwrap();
    assert_eq!(rep.deleted, 1);
    assert!(!info_dir(&h, "AN-V").exists(), "info 物理删除");
    assert!(h.lib.folders.items.iter().all(|p| p.asset != "AN-V"), "Placement 移除");
    let folders: FoldersFile = read_json(&h.lib.root.join(FOLDERS_FILE)).unwrap();
    assert!(folders.items.iter().all(|p| p.asset != "AN-V"), "folders.json 落盘");
    assert!(!h.lib.index.assets.contains_key("AN-V"));
    assert!(h.lib.recent.items.is_empty(), "recent 剔除");
    let recent: RecentFile = read_json(&h.lib.root.join(RECENT_FILE)).unwrap();
    assert!(recent.items.is_empty(), "recent.json 落盘");
    let pins: PinsFile = read_json(&h.lib.root.join(PINS_FILE)).unwrap();
    assert!(pins.pins.is_empty(), "pins 剔除落盘");
    assert!(h.lib.index.assets.contains_key("AN-SEG"), "片段自身保留");
    // 无引用免确认 + 陈旧 ID 幂等
    let rep = delete_assets_permanently(&mut h.lib, &["AN-V".into()], false).unwrap();
    assert_eq!(rep.deleted, 0);
  }

  #[test]
  fn delete_folder_recycle_mode_moves_to_root_and_undoable_shape() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F-TOP", None);
    add_folder(&mut h.lib, "F-SUB", Some("F-TOP"));
    add_asset(&mut h, "AN-A", "a.png", "F-SUB");
    add_asset(&mut h, "AN-B", "a.png", "root"); // root 已有同名 → 唯一化

    let rep = delete_folder(&mut h.lib, "F-TOP", false, false).unwrap();
    assert_eq!((rep.moved_placements, rep.deleted_folders), (1, 2));
    let p = h.lib.folders.items.iter().find(|p| p.asset == "AN-A").unwrap();
    assert_eq!(p.folder, ROOT_FOLDER_ID, "素材移未分类");
    assert_eq!(p.name, "a.png", "撞名放开不唯一化（2026-08-19）");
    assert!(h.lib.folders.folders.is_empty(), "子树节点删除");
    assert!(h.lib.index.folders.is_empty(), "索引同步");
    assert!(info_dir(&h, "AN-A").is_dir(), "素材物理不动");
    let orders: Vec<i32> = h.lib.folders.items.iter().filter(|p| p.folder == "root").map(|p| p.order).collect();
    assert_eq!(orders, vec![0, 1], "root 段 order 重写连续");
    // 目标不存在报错
    assert!(delete_folder(&mut h.lib, "F-不存在", false, false).is_err());
  }

  /// 位置级模型（2026-08-19）：删除文件夹 = 子树 origin 与链接**全部回 root**（未分类）——
  /// "链接随文件夹直接删除"旧口径废除（护栏拆干净，链接当真实文件对待）
  #[test]
  fn delete_folder_recycle_mode_moves_links_to_root_too() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F-TOP", None);
    add_folder(&mut h.lib, "F-SUB", Some("F-TOP"));
    add_folder(&mut h.lib, "F-OUT", None);
    add_asset(&mut h, "AN-A", "a.png", "F-SUB"); // 子树内 origin → 回 root
    add_asset(&mut h, "AN-B", "b.png", "F-OUT"); // origin 在子树外
    add_asset(&mut h, "AN-C", "c.png", "F-SUB"); // origin 与链接同在子树内
    // 链接：AN-B（origin 在别处）与 AN-C（origin 同批回 root）各挂一条进子树
    ops::add_placement(&mut h.lib, "AN-B".into(), "F-SUB".into(), "b.png".into(), false, &mut HashMap::new());
    ops::add_placement(&mut h.lib, "AN-C".into(), "F-SUB".into(), "c.png".into(), false, &mut HashMap::new());
    finish(&mut h.lib).unwrap();
    assert_eq!(h.lib.folders.items.len(), 5, "3 origin + 2 链接");

    let rep = delete_folder(&mut h.lib, "F-TOP", false, false).unwrap();
    assert_eq!((rep.moved_placements, rep.deleted_folders), (4, 2), "origin + 链接全部计入移入未分类");
    let a = h.lib.folders.items.iter().find(|p| p.asset == "AN-A").unwrap();
    assert_eq!((a.folder.as_str(), a.origin), (ROOT_FOLDER_ID, true), "AN-A origin 回 root");
    let b: Vec<_> = h.lib.folders.items.iter().filter(|p| p.asset == "AN-B").collect();
    assert_eq!(b.len(), 2, "AN-B 子树链接回 root（不随删）");
    assert!(b.iter().any(|p| p.folder == ROOT_FOLDER_ID && !p.origin), "链接标记不动");
    let c: Vec<_> = h.lib.folders.items.iter().filter(|p| p.asset == "AN-C").collect();
    assert_eq!(c.len(), 2, "AN-C origin 与链接都回 root");
    assert!(c.iter().all(|p| p.folder == ROOT_FOLDER_ID));
    // 落盘一致：5 条全保留（4 回 root + AN-B 别处的 origin 不动）
    let disk: FoldersFile = read_json(&h.lib.root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(disk.items.len(), 5);
  }

  /// 3.4 批量删除（2026-08-18）：多选兄弟一趟删，素材分别回 root，链接随删，报告聚合
  #[test]
  fn delete_folders_batch_siblings_single_pass() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F-A", None);
    add_folder(&mut h.lib, "F-B", None);
    add_folder(&mut h.lib, "F-B-SUB", Some("F-B"));
    add_folder(&mut h.lib, "F-KEEP", None);
    add_asset(&mut h, "AN-A", "a.png", "F-A");
    add_asset(&mut h, "AN-B", "b.png", "F-B-SUB");
    add_asset(&mut h, "AN-K", "k.png", "F-KEEP");
    // F-A 内一条链接（origin 在 F-KEEP）随文件夹删
    ops::add_placement(&mut h.lib, "AN-K".into(), "F-A".into(), "k.png".into(), false, &mut HashMap::new());
    finish(&mut h.lib).unwrap();

    let rep = delete_folders(&mut h.lib, &["F-A".into(), "F-B".into()]).unwrap();
    assert_eq!((rep.moved_placements, rep.deleted_folders), (3, 3), "2 origin + 1 链接全回 root + 3 节点");
    let keep: Vec<_> = h.lib.folders.items.iter().filter(|p| p.asset == "AN-K").collect();
    assert_eq!(keep.len(), 2, "链接回 root 不随删（位置级模型）");
    assert!(keep.iter().any(|p| p.folder == ROOT_FOLDER_ID && !p.origin), "链接挪未分类原样");
    assert_eq!(h.lib.folders.folders.len(), 1, "只剩 F-KEEP 节点");
    assert_eq!(h.lib.folders.folders[0].id, "F-KEEP");
    for (asset, name) in [("AN-A", "a.png"), ("AN-B", "b.png")] {
      let p = h.lib.folders.items.iter().find(|p| p.asset == asset).unwrap();
      assert_eq!((p.folder.as_str(), p.name.as_str()), (ROOT_FOLDER_ID, name), "{asset} 回 root");
    }
    let disk: FoldersFile = read_json(&h.lib.root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(disk.items.len(), 4, "落盘一致（3 回 root + 1 保留）");
  }

  /// 3.4 批量删除：祖先与后代同批入选时后代去重（删祖先已连带子树，不再对后代报不存在）；
  /// 部分陈旧 id（选中后已被删）剔除容错，全部不存在才报错
  #[test]
  fn delete_folders_skips_descendant_when_ancestor_in_batch() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F-TOP", None);
    add_folder(&mut h.lib, "F-SUB", Some("F-TOP"));
    add_asset(&mut h, "AN-A", "a.png", "F-SUB");

    let rep = delete_folders(&mut h.lib, &["F-SUB".into(), "F-不存在".into(), "F-TOP".into()]).unwrap();
    assert_eq!((rep.moved_placements, rep.deleted_folders), (1, 2), "子树只删一趟不重复计数，陈旧 id 容错");
    assert!(h.lib.folders.folders.is_empty());
    assert!(delete_folders(&mut h.lib, &["F-不存在".into()]).is_err(), "全部不存在报错");
  }

  #[test]
  fn delete_folder_permanent_two_phase_and_purge() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F-TOP", None);
    add_folder(&mut h.lib, "F-SUB", Some("F-TOP"));
    add_asset(&mut h, "AN-V", "视频.mp4", "F-SUB");
    add_asset(&mut h, "AN-SEG", "片段.mp4", "root");
    let seg = h.lib.index.assets.get_mut("AN-SEG").unwrap();
    seg.r#type = Some("segment".into());
    seg.source = Some("AN-V".into());

    // 第一段：子树外片段引用 → 拦截并带计数
    let rep = delete_folder(&mut h.lib, "F-TOP", true, false).unwrap();
    assert!(rep.need_confirm);
    assert_eq!((rep.segment_refs, rep.asset_count, rep.folder_count), (1, 1, 2));
    assert!(info_dir(&h, "AN-V").is_dir());

    // 第二段：确认 → 物理全删 + 节点删除
    let rep = delete_folder(&mut h.lib, "F-TOP", true, true).unwrap();
    assert_eq!((rep.deleted, rep.deleted_folders), (1, 2));
    assert!(!info_dir(&h, "AN-V").exists());
    assert!(h.lib.folders.folders.is_empty());
    assert!(h.lib.folders.items.iter().all(|p| p.asset != "AN-V"));
    assert!(h.lib.index.assets.contains_key("AN-SEG"), "子树外片段保留");
  }

  /// 2026-09-03 半途失败加固（先物理后数据）：占用失败项数据层原样保留可重试
  ///（Placement / index / recent 不动 + info 保留），成功项照常全清，报错含名单
  #[cfg(windows)]
  #[test]
  fn delete_assets_permanently_partial_failure_keeps_failed_asset() {
    use std::os::windows::fs::OpenOptionsExt;
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", None);
    add_asset(&mut h, "AN-A", "a.png", "F01");
    add_asset(&mut h, "AN-B", "b.fla", "root");
    touch_recent(&mut h.lib, "AN-B").unwrap();
    // 独占打开 AN-B 实体 → remove_dir_all 必失败
    let _lock = std::fs::OpenOptions::new().read(true).share_mode(0).open(info_dir(&h, "AN-B").join("b.fla")).unwrap();

    let err = delete_assets_permanently(&mut h.lib, &["AN-A".into(), "AN-B".into()], true).unwrap_err();
    assert!(err.contains("b") && err.contains("保留在库中"), "报错含名单与重试提示：{err}");
    assert!(!info_dir(&h, "AN-A").exists(), "成功项物理删除");
    assert!(h.lib.folders.items.iter().all(|p| p.asset != "AN-A"), "成功项 Placement 清除");
    assert!(!h.lib.index.assets.contains_key("AN-A"), "成功项 index 移除");
    assert!(info_dir(&h, "AN-B").is_dir(), "失败项 info 保留");
    assert!(h.lib.folders.items.iter().any(|p| p.asset == "AN-B"), "失败项 Placement 保留（旧版此处被误清产孤儿）");
    assert!(h.lib.index.assets.contains_key("AN-B"), "失败项 index 保留");
    assert_eq!(h.lib.recent.items.len(), 1, "失败项 recent 保留");
    // 落盘一致：folders.json 只剩失败项
    let disk: FoldersFile = read_json(&h.lib.root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(disk.items.len(), 1);
    assert_eq!(disk.items[0].asset, "AN-B");
    // 重试路径：放手后再删成功
    drop(_lock);
    let rep = delete_assets_permanently(&mut h.lib, &["AN-B".into()], true).unwrap();
    assert_eq!(rep.deleted, 1);
    assert!(!info_dir(&h, "AN-B").exists(), "重试物理删除成功");
  }

  /// 2026-09-03 半途失败加固（彻底删除文件夹）：失败项子树内 Placement 挪未分类保留
  ///（文件夹节点照删），成功项全清
  #[cfg(windows)]
  #[test]
  fn delete_folder_permanent_partial_failure_moves_failed_to_root() {
    use std::os::windows::fs::OpenOptionsExt;
    let mut h = make_lib();
    add_folder(&mut h.lib, "F-TOP", None);
    add_asset(&mut h, "AN-A", "a.png", "F-TOP");
    add_asset(&mut h, "AN-B", "b.fla", "F-TOP");
    let _lock = std::fs::OpenOptions::new().read(true).share_mode(0).open(info_dir(&h, "AN-B").join("b.fla")).unwrap();

    let err = delete_folder(&mut h.lib, "F-TOP", true, true).unwrap_err();
    assert!(err.contains("移入未分类"), "{err}");
    assert!(!info_dir(&h, "AN-A").exists(), "成功项物理删除");
    assert!(h.lib.folders.folders.is_empty(), "文件夹节点照删");
    let b = h.lib.folders.items.iter().find(|p| p.asset == "AN-B").unwrap();
    assert_eq!(b.folder, ROOT_FOLDER_ID, "失败项挪未分类（不指向已删文件夹）");
    assert!(h.lib.index.assets.contains_key("AN-B"), "失败项 index 保留");
  }
}
