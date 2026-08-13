//! Placement 操作（6.1）：移动（含摘标）、段内排序、关联（链接 Placement）。

use std::collections::{HashMap, HashSet};

use crate::core::json_io::write_json;
use crate::core::library::{self, Library};
use crate::core::models::Placement;
use crate::core::paths;
use crate::services::import as ops;
use crate::services::import::ROOT_FOLDER_ID;

use super::{check_folder, find_placement, finish, rewrite_orders, PlacementRef};

/// 移动 Placement 到目标文件夹，返回实际移动条数：
/// - 目标文件夹已有同资源 Placement 则去重跳过（resolve.rs use_existing 语义）；
/// - 目标内显示名 unique_name，order 追加段尾；
/// - 移出 root（"root" → 非 root）清该资源 metadata.importedFrom（摘标：移动即转正，
///   metadata 写盘 + 索引同步；对应 metadata.json 快照由命令层 record_file）；
/// - 原地移动与找不到的陈旧三元组跳过不计数
pub fn move_placements(lib: &mut Library, items: &[PlacementRef], target_folder: &str) -> Result<usize, String> {
  check_folder(lib, target_folder)?;
  let mut moved = 0usize;
  for item in items {
    if item.folder_id == target_folder {
      continue; // 原地移动 = 无操作
    }
    let Some(pos) = find_placement(lib, item) else { continue };
    let asset = lib.folders.items[pos].asset.clone();
    if lib.folders.items.iter().any(|p| p.asset == asset && p.folder == target_folder) {
      continue; // 去重：目标已有同资源 Placement
    }
    if item.folder_id == ROOT_FOLDER_ID && target_folder != ROOT_FOLDER_ID {
      clear_imported_from(lib, &asset)?;
    }
    let existing: HashSet<String> =
      lib.folders.items.iter().filter(|p| p.folder == target_folder).map(|p| p.name.clone()).collect();
    let order = existing.len() as i32;
    let old_name = lib.folders.items[pos].name.clone();
    let p = &mut lib.folders.items[pos];
    p.name = library::unique_name(&existing, &old_name);
    p.folder = target_folder.to_string();
    p.order = order;
    moved += 1;
  }
  if moved > 0 {
    finish(lib)?;
    log::info!("移动 {moved} 个项目至 {target_folder}");
  }
  Ok(moved)
}

/// 摘标：清 metadata.importedFrom（先落盘 metadata.json 再同步内存索引，规范第 7 条）
fn clear_imported_from(lib: &mut Library, asset_id: &str) -> Result<(), String> {
  let Some(mut meta) = lib.index.assets.get(asset_id).cloned() else { return Ok(()) };
  if meta.imported_from.is_none() {
    return Ok(());
  }
  meta.imported_from = None;
  let meta_path = paths::metadata_file(&lib.assets_dir, asset_id);
  write_json(&meta_path, &meta).map_err(|e| e.to_string())?;
  lib.index.assets.insert(asset_id.to_string(), meta);
  Ok(())
}

/// 段内排序：items（须同属一个文件夹）整体移到 target_folder 段内 before_item 之前
///（None = 段尾），vec 重排 + 受影响文件夹段 order 重写为连续 0..n。
/// 跨文件夹时改 folder + 目标内显示名 unique_name，源 / 目标两段 order 都重写；
/// 跨文件夹移出 root 清 metadata.importedFrom（摘标口径与 move_placements 一致：移动即转正）
pub fn reorder_placements(
  lib: &mut Library,
  items: &[PlacementRef],
  target_folder: &str,
  before_item: Option<&PlacementRef>,
) -> Result<(), String> {
  check_folder(lib, target_folder)?;
  if items.is_empty() {
    return Ok(());
  }
  if let Some(b) = before_item {
    if items.contains(b) {
      return Err("插入位置无效：目标项目在移动列表中".to_string());
    }
  }
  let mut positions = Vec::with_capacity(items.len());
  let mut seen = HashSet::new();
  for it in items {
    let pos = find_placement(lib, it).ok_or_else(|| "项目不存在，可能已被移动或删除".to_string())?;
    if !seen.insert(pos) {
      return Err("调整列表存在重复项目".to_string());
    }
    positions.push(pos);
  }
  let source = lib.folders.items[positions[0]].folder.clone();
  if positions.iter().any(|&i| lib.folders.items[i].folder != source) {
    return Err("只能调整同一文件夹内的项目顺序".to_string());
  }

  // 跨文件夹移出 root：先摘标（与 move_placements 同口径：移动即转正，metadata 写盘 + 索引同步）
  if source == ROOT_FOLDER_ID && target_folder != ROOT_FOLDER_ID {
    for it in items {
      clear_imported_from(lib, &it.asset_id)?;
    }
  }

  // 提取移动项（按传入顺序）并从 vec 移除（下标降序，防位移）
  let mut moving: Vec<Placement> = positions.iter().map(|&i| lib.folders.items[i].clone()).collect();
  let mut desc = positions.clone();
  desc.sort_unstable_by(|a, b| b.cmp(a));
  for i in desc {
    lib.folders.items.remove(i);
  }
  // 跨文件夹：改 folder + 目标内显示名唯一化
  if source != target_folder {
    let mut existing: HashSet<String> =
      lib.folders.items.iter().filter(|p| p.folder == target_folder).map(|p| p.name.clone()).collect();
    for p in &mut moving {
      p.folder = target_folder.to_string();
      p.name = library::unique_name(&existing, &p.name);
      existing.insert(p.name.clone());
    }
  }
  // 插入点：before_item 在目标段中的 vec 下标；None = 段尾
  let insert_at = match before_item {
    Some(b) => {
      let idx = find_placement(lib, b).ok_or_else(|| "插入位置无效：目标项目不存在".to_string())?;
      if lib.folders.items[idx].folder != target_folder {
        return Err("插入位置无效：目标项目不在目标文件夹".to_string());
      }
      idx
    }
    None => lib
      .folders
      .items
      .iter()
      .rposition(|p| p.folder == target_folder)
      .map(|i| i + 1)
      .unwrap_or(lib.folders.items.len()),
  };
  for (off, p) in moving.into_iter().enumerate() {
    lib.folders.items.insert(insert_at + off, p);
  }
  rewrite_orders(lib, target_folder);
  if source != target_folder {
    rewrite_orders(lib, &source);
  }
  finish(lib)
}

/// 关联：目标文件夹逐条建链接 Placement（origin = false），返回关联条数；
/// 显示名 = names[i] 非空 ? 自定义 : 源 Placement 名，同文件夹 unique_name（add_placement 内部处理）。
/// 链接不设限（2026-08-13 用户定案：不替用户做整理决定——落点允许 root、任意资产可发起、
/// 同文件夹允许重复链接；原"候车区双护栏"废除）
pub fn link_placements(
  lib: &mut Library,
  items: &[PlacementRef],
  target_folder: &str,
  names: &[Option<String>],
) -> Result<usize, String> {
  check_folder(lib, target_folder)?;
  let mut anchor = HashMap::new();
  let mut linked = 0usize;
  for (i, item) in items.iter().enumerate() {
    let Some(pos) = find_placement(lib, item) else { continue };
    let source_name = lib.folders.items[pos].name.clone();
    let name = names
      .get(i)
      .and_then(|n| n.as_ref())
      .map(|n| n.trim())
      .filter(|n| !n.is_empty())
      .unwrap_or(&source_name)
      .to_string();
    ops::add_placement(lib, item.asset_id.clone(), target_folder.to_string(), name, false, &mut anchor);
    linked += 1;
  }
  if linked > 0 {
    finish(lib)?;
    log::info!("关联 {linked} 个项目至 {target_folder}");
  }
  Ok(linked)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::json_io::read_json;
  use crate::core::library::FOLDERS_FILE;
  use crate::core::models::{AssetMeta, Folder, FoldersFile};
  use crate::core::undo::UndoManager;
  use crate::services::import::{PendingFile, Source};

  struct Harness {
    dir: tempfile::TempDir,
    lib: Library,
    /// 前插锚点（add_placement 批内保序）：挂 Harness 上让多次 add_asset 保持到达顺序
    anchor: HashMap<String, usize>,
  }

  fn make_lib() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    Harness { dir, lib, anchor: HashMap::new() }
  }

  fn add_folder(lib: &mut Library, id: &str) {
    let folder = Folder { id: id.into(), name: id.into(), parent: None, order: 0 };
    lib.folders.folders.push(folder.clone());
    lib.index.folders.insert(id.into(), folder);
  }

  /// 造资源（info + metadata + 内存索引）并放一条 origin Placement 到指定文件夹
  fn add_asset(h: &mut Harness, id: &str, file: &str, folder: &str, imported_from: Option<&str>) {
    let src = h.dir.path().join(file);
    std::fs::write(&src, b"fake").unwrap();
    let pf = PendingFile { path: src, chain: Vec::new() };
    let meta = ops::build_asset_with_id(&h.lib.assets_dir, &pf, "sha256:t", id, Source::Copy, imported_from).unwrap();
    h.lib.index.assets.insert(id.into(), meta);
    let mut anchor = std::mem::take(&mut h.anchor);
    ops::add_placement(&mut h.lib, id.into(), folder.into(), file.into(), true, &mut anchor);
    h.anchor = anchor;
    finish(&mut h.lib).unwrap();
  }

  fn pref(asset: &str, folder: &str, origin: bool) -> PlacementRef {
    PlacementRef { asset_id: asset.into(), folder_id: folder.into(), origin }
  }

  fn placement(lib: &Library, asset: &str, folder: &str) -> Option<Placement> {
    lib.folders.items.iter().find(|p| p.asset == asset && p.folder == folder).cloned()
  }

  #[test]
  fn move_basic_unique_name_and_dedup() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01");
    add_asset(&mut h, "AN-A", "图.png", "root", None);
    add_asset(&mut h, "AN-B", "图.png", "F01", None); // 目标已有同名（不同资源）
    add_asset(&mut h, "AN-C", "另.png", "root", None);
    // 目标已有同资源链接 → 去重跳过
    ops::add_placement(&mut h.lib, "AN-C".into(), "F01".into(), "另.png".into(), false, &mut HashMap::new());
    finish(&mut h.lib).unwrap();

    let moved = move_placements(&mut h.lib, &[pref("AN-A", "root", true), pref("AN-C", "root", true)], "F01").unwrap();
    assert_eq!(moved, 1, "AN-C 目标已有同资源，去重跳过");
    let a = placement(&h.lib, "AN-A", "F01").unwrap();
    assert_eq!(a.name, "图-1.png", "目标内显示名唯一化");
    assert_eq!(a.order, 2, "追加段尾（目标段已有 2 条）");
    assert!(placement(&h.lib, "AN-A", "root").is_none(), "原位置不再保留");
    assert!(placement(&h.lib, "AN-C", "root").is_some(), "被跳过的保持原样");
    // 落盘验证
    let disk: FoldersFile = read_json(&h.lib.root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(disk.items.iter().filter(|p| p.folder == "F01").count(), 3);
    // 目标文件夹不存在
    assert!(move_placements(&mut h.lib, &[pref("AN-A", "F01", true)], "F-不存在").is_err());
  }

  #[test]
  fn move_out_of_root_clears_imported_from() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01");
    add_asset(&mut h, "AN-IN", "收件.png", "root", Some("inbox"));
    assert_eq!(h.lib.index.assets["AN-IN"].imported_from.as_deref(), Some("inbox"));

    move_placements(&mut h.lib, &[pref("AN-IN", "root", true)], "F01").unwrap();
    assert_eq!(h.lib.index.assets["AN-IN"].imported_from, None, "摘标：移动即转正");
    let meta_path = h.lib.info_dir("AN-IN").join("metadata.json");
    let disk: AssetMeta = read_json(&meta_path).unwrap();
    assert_eq!(disk.imported_from, None, "metadata.json 落盘同步");
  }

  #[test]
  fn reorder_same_folder_compacts_orders() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01");
    add_asset(&mut h, "AN-A", "a.png", "F01", None);
    add_asset(&mut h, "AN-B", "b.png", "F01", None);
    add_asset(&mut h, "AN-C", "c.png", "F01", None);

    // c 移到 a 之前
    reorder_placements(&mut h.lib, &[pref("AN-C", "F01", true)], "F01", Some(&pref("AN-A", "F01", true))).unwrap();
    let seg: Vec<(&str, i32)> =
      h.lib.folders.items.iter().filter(|p| p.folder == "F01").map(|p| (p.asset.as_str(), p.order)).collect();
    assert_eq!(seg, vec![("AN-C", 0), ("AN-A", 1), ("AN-B", 2)], "vec 重排 + order 连续 0..n");

    // before = None → 段尾
    reorder_placements(&mut h.lib, &[pref("AN-C", "F01", true)], "F01", None).unwrap();
    let seg: Vec<&str> =
      h.lib.folders.items.iter().filter(|p| p.folder == "F01").map(|p| p.asset.as_str()).collect();
    assert_eq!(seg, vec!["AN-A", "AN-B", "AN-C"]);
  }

  #[test]
  fn reorder_cross_folder_and_guards() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01");
    add_folder(&mut h.lib, "F02");
    add_asset(&mut h, "AN-A", "a.png", "F01", None);
    add_asset(&mut h, "AN-B", "b.png", "F01", None);
    add_asset(&mut h, "AN-C", "c.png", "F02", None);

    // AN-B 跨文件夹移到 F02 段首（c 之前）
    reorder_placements(&mut h.lib, &[pref("AN-B", "F01", true)], "F02", Some(&pref("AN-C", "F02", true))).unwrap();
    let f02: Vec<(&str, i32)> =
      h.lib.folders.items.iter().filter(|p| p.folder == "F02").map(|p| (p.asset.as_str(), p.order)).collect();
    assert_eq!(f02, vec![("AN-B", 0), ("AN-C", 1)]);
    assert_eq!(h.lib.folders.items.iter().find(|p| p.asset == "AN-A").unwrap().order, 0, "源段 order 同步重写");

    // 护栏：混合文件夹 / before 在移动列表 / before 不在目标段
    let mixed = reorder_placements(&mut h.lib, &[pref("AN-A", "F01", true), pref("AN-C", "F02", true)], "F01", None);
    assert!(mixed.unwrap_err().contains("同一文件夹"));
    let self_before =
      reorder_placements(&mut h.lib, &[pref("AN-A", "F01", true)], "F01", Some(&pref("AN-A", "F01", true)));
    assert!(self_before.is_err());
    let wrong_seg = reorder_placements(&mut h.lib, &[pref("AN-A", "F01", true)], "F01", Some(&pref("AN-C", "F02", true)));
    assert!(wrong_seg.is_err());
  }

  /// 跨文件夹移出 root 摘标（与 move_placements 同口径：移动即转正）；段内排序不摘标
  #[test]
  fn reorder_out_of_root_clears_imported_from() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01");
    add_asset(&mut h, "AN-IN", "收件.png", "root", Some("inbox"));
    add_asset(&mut h, "AN-PAD", "垫.png", "root", None);

    // 段内排序（root → root）不摘标
    reorder_placements(&mut h.lib, &[pref("AN-IN", "root", true)], "root", None).unwrap();
    assert_eq!(h.lib.index.assets["AN-IN"].imported_from.as_deref(), Some("inbox"), "段内排序不摘标");

    // 跨文件夹移出 root → 摘标 + metadata.json 落盘同步
    reorder_placements(&mut h.lib, &[pref("AN-IN", "root", true)], "F01", None).unwrap();
    assert_eq!(h.lib.index.assets["AN-IN"].imported_from, None, "摘标：跨文件夹移动即转正");
    let meta_path = h.lib.info_dir("AN-IN").join("metadata.json");
    let disk: AssetMeta = read_json(&meta_path).unwrap();
    assert_eq!(disk.imported_from, None, "metadata.json 落盘同步");
  }

  #[test]
  fn link_honors_custom_and_default_names() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01");
    add_folder(&mut h.lib, "F02");
    // 源 Placement 放 F02（链接落点 / 发起均不设限，2026-08-13 用户定案）
    add_asset(&mut h, "AN-A", "图.png", "F02", None);
    add_asset(&mut h, "AN-B", "片.png", "F02", None);

    let linked = link_placements(
      &mut h.lib,
      &[pref("AN-A", "F02", true), pref("AN-B", "F02", true)],
      "F01",
      &[Some("自定义.png".into()), None],
    )
    .unwrap();
    assert_eq!(linked, 2);
    let a = placement(&h.lib, "AN-A", "F01").unwrap();
    assert!(!a.origin, "链接 Placement 非 origin");
    assert_eq!(a.name, "自定义.png");
    assert_eq!(placement(&h.lib, "AN-B", "F01").unwrap().name, "片.png", "默认源 Placement 名");
    // 再关联一次同资源：同文件夹显示名唯一化
    link_placements(&mut h.lib, &[pref("AN-B", "F02", true)], "F01", &[]).unwrap();
    let names: HashSet<&str> =
      h.lib.folders.items.iter().filter(|p| p.folder == "F01").map(|p| p.name.as_str()).collect();
    assert!(names.contains("片-1.png"), "重名追加 -序号：{names:?}");
  }

  /// 链接不设限（2026-08-13 用户定案）：落点 root（未分类）允许建链接；
  /// origin 还在 root 的资产也可发起（含批量混合，不再预检拦截）
  #[test]
  fn link_allows_root_target_and_waiting_origin() {
    let mut h = make_lib();
    add_asset(&mut h, "AN-WAIT", "候.png", "root", None); // origin 滞留未分类

    // 落点 = root：允许（同文件夹重复链接显示名唯一化）
    let linked = link_placements(&mut h.lib, &[pref("AN-WAIT", "root", true)], "root", &[]).unwrap();
    assert_eq!(linked, 1);
    let names: HashSet<&str> =
      h.lib.folders.items.iter().filter(|p| p.folder == "root").map(|p| p.name.as_str()).collect();
    assert!(names.contains("候-1.png"), "同文件夹重复链接唯一化：{names:?}");

    // 发起端 = 候车区资产落文件夹：同样放行
    add_folder(&mut h.lib, "F01");
    let linked = link_placements(&mut h.lib, &[pref("AN-WAIT", "root", true)], "F01", &[]).unwrap();
    assert_eq!(linked, 1);
    assert!(placement(&h.lib, "AN-WAIT", "F01").is_some());
  }

  /// 撤销接入验证：begin → record_file → 业务 → commit 后 undo 栈有一条，undo 回滚 folders.json
  #[test]
  fn undo_rolls_back_move() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01");
    add_asset(&mut h, "AN-A", "图.png", "root", None);
    let path = h.lib.root.join(FOLDERS_FILE);

    let mut undo = UndoManager::default();
    undo.begin("移动 1 个项目");
    undo.record_file(&path).unwrap();
    move_placements(&mut h.lib, &[pref("AN-A", "root", true)], "F01").unwrap();
    undo.commit().unwrap();
    assert!(undo.state().can_undo);
    assert_eq!(undo.state().undo_label.as_deref(), Some("移动 1 个项目"));

    undo.undo().unwrap();
    crate::core::library::reload(&mut h.lib).unwrap();
    assert!(placement(&h.lib, "AN-A", "root").is_some(), "undo 后 Placement 回到 root");
    assert!(placement(&h.lib, "AN-A", "F01").is_none());
  }
}
