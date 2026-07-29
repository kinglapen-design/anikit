//! 彻底删除服务（6.3 右键菜单）：素材直接彻底删除（不经过回收站）、删除文件夹两形态。
//! - delete_assets_permanently：两段式（虚拟片段引用拦截）→ 移除全部 Placement + 物理删 info
//!   + 四处同步清理（index / recycle / recent / pins，复用 recycle.rs 同收尾）；**不接撤销**。
//! - delete_folder(permanent=false)："删除文件夹"——子树 Placement 全部改挂 root（素材进未分类，
//!   2026-07-29 用户定）+ 子树文件夹节点删除；仅改数据，**撤销三步由命令层接入**。
//! - delete_folder(permanent=true)："彻底删除文件夹"——两段式 → 子树资产物理全删 + 节点删除；
//!   **不接撤销**。两形态执行前均 backup_library 兜底（失败仅警告不阻断）。

use std::collections::HashSet;
use std::fs;

use serde::Serialize;

use crate::core::backup;
use crate::core::library::{self, Library, ASSTES_DIR};
use crate::services::import::ROOT_FOLDER_ID;
use crate::services::operations::{finish, rewrite_orders, sync_folder_index};
use crate::services::recycle::{count_segment_refs, sync_after_purge, DeleteRecycleReport};

/// 素材直接彻底删除（两段式）：库内其他资产虚拟片段引用被删集合时未确认只拦截；
/// 确认后移除全部 Placement + 物理删 info + 四处同步清理。返回与回收站彻底删除同构报告
pub fn delete_assets_permanently(lib: &mut Library, asset_ids: &[String], confirmed: bool) -> Result<DeleteRecycleReport, String> {
  let targets: Vec<String> = asset_ids.iter().filter(|id| lib.index.assets.contains_key(id.as_str())).cloned().collect();
  if targets.is_empty() {
    return Ok(DeleteRecycleReport { need_confirm: false, segment_refs: 0, deleted: 0 });
  }
  let target_set: HashSet<&str> = targets.iter().map(String::as_str).collect();
  let segment_refs = count_segment_refs(lib, &target_set);
  if segment_refs > 0 && !confirmed {
    return Ok(DeleteRecycleReport { need_confirm: true, segment_refs, deleted: 0 });
  }
  if let Err(e) = backup::backup_library(&lib.root) {
    log::warn!("彻底删除前备份失败（不阻断删除）：{e}");
  }
  // 数据段：移除全部 Placement（资产随之从各文件夹消失，与回收站流入路径殊途同归）
  lib.folders.items.retain(|p| !target_set.contains(p.asset.as_str()));
  finish(lib)?;
  // 物理段：半途失败先把已删部分同步一致再报错（recycle.rs 同策略）
  let mut removed: Vec<String> = Vec::new();
  for id in &targets {
    let info = lib.root.join(ASSTES_DIR).join(format!("{id}.info"));
    if info.exists() {
      if let Err(e) = fs::remove_dir_all(&info) {
        sync_after_purge(lib, &removed)?;
        let name = lib.index.assets.get(id).map(|m| m.name.clone()).unwrap_or_else(|| id.clone());
        return Err(format!("删除「{name}」的文件失败：{e}"));
      }
    }
    removed.push(id.clone());
  }
  sync_after_purge(lib, &removed)?;
  log::info!("彻底删除 {} 个素材（虚拟片段引用 {segment_refs}）", removed.len());
  Ok(DeleteRecycleReport { need_confirm: false, segment_refs, deleted: removed.len() })
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

/// 删除文件夹两形态（见文件头注释）
pub fn delete_folder(lib: &mut Library, folder_id: &str, permanent: bool, confirmed: bool) -> Result<DeleteFolderReport, String> {
  if !lib.index.folders.contains_key(folder_id) {
    return Err("目标文件夹不存在，可能已被删除".to_string());
  }
  let subtree = subtree_ids(lib, folder_id);
  let subtree_set: HashSet<&str> = subtree.iter().map(String::as_str).collect();
  let folder_count = subtree.len();

  if !permanent {
    // "删除文件夹"：子树 Placement 全部改挂 root（素材进未分类，重名唯一化）+ 节点删除
    let mut root_names: HashSet<String> =
      lib.folders.items.iter().filter(|p| p.folder == ROOT_FOLDER_ID).map(|p| p.name.clone()).collect();
    let mut moved = 0usize;
    for p in &mut lib.folders.items {
      if subtree_set.contains(p.folder.as_str()) {
        let name = library::unique_name(&root_names, &p.name);
        root_names.insert(name.clone());
        p.name = name;
        p.folder = ROOT_FOLDER_ID.to_string();
        moved += 1;
      }
    }
    lib.folders.folders.retain(|f| !subtree_set.contains(f.id.as_str()));
    sync_folder_index(lib);
    rewrite_orders(lib, ROOT_FOLDER_ID);
    finish(lib)?;
    log::info!("删除文件夹 {folder_id}（{moved} 个素材移入未分类，{folder_count} 个文件夹节点）");
    return Ok(DeleteFolderReport { moved_placements: moved, deleted_folders: folder_count, ..empty_report() });
  }

  // "彻底删除文件夹"：子树资产物理全删（两段式）
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
  if let Err(e) = backup::backup_library(&lib.root) {
    log::warn!("彻底删除文件夹前备份失败（不阻断删除）：{e}");
  }
  // 数据段：子树 Placement 与文件夹节点移除
  lib.folders.items.retain(|p| !subtree_set.contains(p.folder.as_str()));
  lib.folders.folders.retain(|f| !subtree_set.contains(f.id.as_str()));
  sync_folder_index(lib);
  finish(lib)?;
  // 物理段
  let mut removed: Vec<String> = Vec::new();
  for id in &asset_set {
    let info = lib.root.join(ASSTES_DIR).join(format!("{id}.info"));
    if info.exists() {
      if let Err(e) = fs::remove_dir_all(&info) {
        sync_after_purge(lib, &removed)?;
        let name = lib.index.assets.get(id).map(|m| m.name.clone()).unwrap_or_else(|| id.clone());
        return Err(format!("删除「{name}」的文件失败：{e}"));
      }
    }
    removed.push(id.clone());
  }
  sync_after_purge(lib, &removed)?;
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

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::json_io::read_json;
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
    let root = dir.path().join("测试库");
    let lib = library::create(&root).unwrap();
    Harness { dir, lib }
  }

  fn add_folder(lib: &mut Library, id: &str, parent: Option<&str>) {
    let folder = Folder { id: id.into(), name: id.into(), parent: parent.map(|p| p.to_string()), order: 0 };
    lib.folders.folders.push(folder.clone());
    lib.index.folders.insert(id.into(), folder);
  }

  fn add_asset(h: &mut Harness, id: &str, file: &str, folder: &str) {
    let src = h.dir.path().join(file);
    fs::write(&src, b"fake").unwrap();
    let pf = PendingFile { path: src, chain: Vec::new() };
    let meta = ops::build_asset_with_id(&h.lib.root, &pf, "sha256:t", id, Source::Copy, None).unwrap();
    h.lib.index.assets.insert(id.into(), meta);
    ops::add_placement(&mut h.lib, id.into(), folder.into(), file.into(), true, &mut HashMap::new());
    finish(&mut h.lib).unwrap();
  }

  fn info_dir(h: &Harness, id: &str) -> std::path::PathBuf {
    h.lib.root.join(ASSTES_DIR).join(format!("{id}.info"))
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
    assert_eq!(p.name, "a-1.png", "root 重名唯一化");
    assert!(h.lib.folders.folders.is_empty(), "子树节点删除");
    assert!(h.lib.index.folders.is_empty(), "索引同步");
    assert!(info_dir(&h, "AN-A").is_dir(), "素材物理不动");
    let orders: Vec<i32> = h.lib.folders.items.iter().filter(|p| p.folder == "root").map(|p| p.order).collect();
    assert_eq!(orders, vec![0, 1], "root 段 order 重写连续");
    // 目标不存在报错
    assert!(delete_folder(&mut h.lib, "F-不存在", false, false).is_err());
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
}
