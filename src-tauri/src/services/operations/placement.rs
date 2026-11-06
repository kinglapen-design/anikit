//! Placement 操作（6.1）：移动（含摘标）、段内排序、关联（链接 Placement）。

use std::collections::{HashMap, HashSet};

use crate::core::library::{self, Library};
use crate::core::models::Placement;
use crate::core::paths;
use crate::services::import as ops;
use crate::services::import::ROOT_FOLDER_ID;

use super::{check_folder, find_placement, finish, rewrite_orders, PlacementRef, SyncLinkAdd, SyncLinkRemove, SyncLinkRename};

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
  paths::save_asset_metadata(&lib.assets_dir, &meta).map_err(|e| e.to_string())?;
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

/// 删除行定位（sync_asset_links）：name 随三元组入 PlacementRef（给了 = 精确到行；
/// find_placement 见名即用精确语义，2026-10-11 PlacementRef 补 name 维度）
fn locate_remove(lib: &Library, asset_id: &str, r: &SyncLinkRemove) -> Option<usize> {
  let item = PlacementRef { asset_id: asset_id.into(), folder_id: r.folder_id.clone(), origin: r.origin, name: r.name.clone() };
  find_placement(lib, &item)
}

/// 将此文件设为原始文件（6.2 四轮走查③，PRD 01 边界规则 8 手动 origin 转移——"原始文件标识
/// 的空缺由用户自主决定"）：目标 Placement 置 origin = true，原 origin 行（若存在）同时置 false
/// （库内同一时刻 origin 恒 ≤ 1）；**仅标记转移不动磁盘**（originName 磁盘文件名不随链接显示名改，
/// 之后重命名 origin 才按同步语义改磁盘）；name 给了 = 同文件夹重复链接精确到行；
/// 目标已是 origin = 无操作 false（命令层不进撤销栈）
pub fn set_placement_origin(
  lib: &mut Library,
  asset_id: &str,
  folder_id: &str,
  name: Option<&str>,
) -> Result<bool, String> {
  if !lib.index.assets.contains_key(asset_id) {
    return Err("文件不存在，可能已被删除".to_string());
  }
  let item = PlacementRef { asset_id: asset_id.into(), folder_id: folder_id.into(), origin: false, name: name.map(|n| n.to_string()) };
  // 先按 origin=false 找；找不到再按 true 找（幂等判定）
  let pos = find_placement(lib, &item).or_else(|| {
    let item_origin = PlacementRef { asset_id: asset_id.into(), folder_id: folder_id.into(), origin: true, name: name.map(|n| n.to_string()) };
    find_placement(lib, &item_origin)
  });
  let Some(pos) = pos else {
    return Err("项目不存在，可能已被移动或删除".to_string());
  };
  if lib.folders.items[pos].origin {
    return Ok(false); // 已是原始文件
  }
  for p in &mut lib.folders.items {
    if p.asset == asset_id && p.origin {
      p.origin = false;
    }
  }
  lib.folders.items[pos].origin = true;
  finish(lib)?;
  log::info!("设为原始文件 {asset_id} @ {folder_id}");
  Ok(true)
}

/// 链接归并（扫描文件/档案链接 屏 2 确认，PRD 03_查找文件-04，2026-10-16 两段式改版）：
/// 每个保留者位置**成为/保持唯一 origin**，该资产其余全部位置（链接 + 被替换的旧 origin）
/// 移入回收站——资产只剩保留者一个位置；物理不动（记录级，recycle.json 聚合可还原）。
/// 逐条删链接归链接管理（sync_asset_links），本函数只做整组归并。
/// 实现 = 既有原语编排（delete_placements recycle + set_placement_origin 幂等），finish 随原语；
/// 保留者定位先按传入 origin、落空换另一态再找（同 set_placement_origin 口径），陈旧跳过不计；
/// 返回清除的位置总数（0 = 命令层不进撤销栈）
pub fn consolidate_asset_links(lib: &mut Library, keepers: &[PlacementRef]) -> Result<usize, String> {
  let mut cleared = 0usize;
  for keeper in keepers {
    let Some(pos) = find_placement(lib, keeper).or_else(|| {
      let mut alt = keeper.clone();
      alt.origin = !keeper.origin;
      find_placement(lib, &alt)
    }) else {
      continue; // 陈旧（刚被移动/删除）跳过
    };
    let asset_id = keeper.asset_id.clone();
    // 其余全部位置进回收站：按行精确（name 恒携带，撞键行不错伤）；下标集在删除前取定
    let others: Vec<PlacementRef> = lib
      .folders
      .items
      .iter()
      .enumerate()
      .filter(|(i, p)| *i != pos && p.asset == asset_id)
      .map(|(_, p)| PlacementRef {
        asset_id: asset_id.clone(),
        folder_id: p.folder.clone(),
        origin: p.origin,
        name: Some(p.name.clone()),
      })
      .collect();
    super::delete_placements(lib, &others, Some("recycle"))?;
    cleared += others.len();
    // 保留者成为 origin（已是 = 幂等无操作；原 origin 已在回收站，恒 ≤ 1 不变式天然成立）
    set_placement_origin(lib, &asset_id, &keeper.folder_id, keeper.name.as_deref())?;
  }
  log::info!("链接归并 {} 组，清除 {} 个位置", keepers.len(), cleared);
  Ok(cleared)
}

/// 链接管理（17 稿，6.2 三轮）：一个资产的位置增 / 删 / 改三组一次提交（命令层单撤销步）。
/// - renames 先行（rename_placement 语义：链接仅改显示名、origin 同步磁盘 originName 段，
///   返回磁盘改名清单供命令层登记 DiskOp）；oldName 给了 = 同文件夹重复链接精确到行；
/// - removes = 直接删 Placement 记录（用户定：管理面板取消勾选不进回收站）；
///   name 给了精确到行，缺省三元组第一条；陈旧项跳过；
/// - adds = 链接 Placement（origin = false，add_placement 前插 + 同文件夹唯一化），
///   name trim 空 = 资产名；
/// - 护栏：执行后该资产位置数不能归 0（防孤儿资产——全部位置删除请走「丢到回收站」）；
/// - 三组全空 = 无操作直接返回（命令层不进撤销栈）。
pub fn sync_asset_links(
  lib: &mut Library,
  asset_id: &str,
  adds: &[SyncLinkAdd],
  removes: &[SyncLinkRemove],
  renames: &[SyncLinkRename],
) -> Result<Vec<(std::path::PathBuf, std::path::PathBuf)>, String> {
  if adds.is_empty() && removes.is_empty() && renames.is_empty() {
    return Ok(Vec::new());
  }
  let source_name =
    lib.index.assets.get(asset_id).map(|m| m.name.clone()).ok_or_else(|| "文件不存在，可能已被删除".to_string())?;
  // 校验全前置：落点文件夹存在 + 改名定位有效（删除定位 = 陈旧跳过不报错）
  for a in adds {
    check_folder(lib, &a.folder_id)?;
  }
  for r in renames {
    let item = PlacementRef { asset_id: asset_id.into(), folder_id: r.folder_id.clone(), origin: r.origin, name: r.old_name.clone() };
    if find_placement(lib, &item).is_none() {
      return Err("项目不存在，可能已被移动或删除".to_string());
    }
  }
  // 护栏：位置归 0 拦截（现有数 − 有效删除数 + 新增数）
  let current = lib.folders.items.iter().filter(|p| p.asset == asset_id).count();
  let valid_removes = removes.iter().filter(|r| locate_remove(lib, asset_id, r).is_some()).count();
  if current + adds.len() <= valid_removes {
    return Err("不能移除该文件的全部位置（需要删除文件请用「丢到回收站」）".to_string());
  }
  let mut disk_renames = Vec::new();
  for r in renames {
    let item = PlacementRef { asset_id: asset_id.into(), folder_id: r.folder_id.clone(), origin: r.origin, name: r.old_name.clone() };
    let pos = find_placement(lib, &item).ok_or_else(|| "项目不存在，可能已被移动或删除".to_string())?;
    if r.origin {
      // origin 每资产恒一条（无重复歧义）：rename_placement 全语义（校验 + 磁盘 originName 段）
      let mut pairs = super::duplicate::rename_placement(lib, asset_id, &r.folder_id, r.origin, &r.new_name, r.old_name.as_deref())?;
      disk_renames.append(&mut pairs);
    } else {
      // 链接行：仅改显示名（撞名不拦截——显示名放开），按精确定位改防重复链接命中错行
      let trimmed = r.new_name.trim();
      library::validate_library_name(trimmed).map_err(|e| e.to_string())?;
      lib.folders.items[pos].name = trimmed.to_string();
    }
  }
  for r in removes {
    if let Some(pos) = locate_remove(lib, asset_id, r) {
      lib.folders.items.remove(pos);
    }
  }
  let mut anchor = HashMap::new();
  for a in adds {
    let name = {
      let t = a.name.trim();
      if t.is_empty() { source_name.clone() } else { t.to_string() }
    };
    ops::add_placement(lib, asset_id.into(), a.folder_id.clone(), name, false, &mut anchor);
  }
  finish(lib)?;
  log::info!("链接管理 {asset_id}：增 {} / 删 {} / 改 {}", adds.len(), valid_removes, renames.len());
  Ok(disk_renames)
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
    let folder = Folder { id: id.into(), name: id.into(), parent: None, block: None, icon: None, kind: None, color: None, groups: None, order: 0 };
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
    PlacementRef { asset_id: asset.into(), folder_id: folder.into(), origin, name: None }
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
    assert_eq!(a.name, "图_1.png", "目标内显示名唯一化");
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
    assert!(names.contains("片_1.png"), "重名追加 _序号：{names:?}");
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
    assert!(names.contains("候_1.png"), "同文件夹重复链接唯一化：{names:?}");

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

  /// 链接管理（6.2 三轮）：增 / 删 / 改一次提交——链接改名仅显示名、链接删除直接删记录、
  /// 新增唯一化前插；陈旧删除项跳过；三组全空 = 无操作
  #[test]
  fn sync_asset_links_add_remove_rename() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01");
    add_folder(&mut h.lib, "F02");
    add_folder(&mut h.lib, "F03");
    add_asset(&mut h, "AN-A", "图.png", "F01", None);
    // 已有一条链接在 F02
    ops::add_placement(&mut h.lib, "AN-A".into(), "F02".into(), "图-链.png".into(), false, &mut HashMap::new());
    finish(&mut h.lib).unwrap();

    let renames = vec![SyncLinkRename { folder_id: "F02".into(), origin: false, old_name: Some("图-链.png".into()), new_name: " 改名.png ".into() }];
    let removes = vec![
      SyncLinkRemove { folder_id: "F01".into(), origin: true, name: Some("图.png".into()) },
      SyncLinkRemove { folder_id: "F99".into(), origin: false, name: None }, // 陈旧跳过
    ];
    let adds = vec![
      SyncLinkAdd { folder_id: "F03".into(), name: "  ".into() }, // 空名 = 资产名
      SyncLinkAdd { folder_id: "F03".into(), name: "图.png".into() }, // 撞名唯一化
    ];
    let disk = sync_asset_links(&mut h.lib, "AN-A", &adds, &removes, &renames).unwrap();
    assert!(disk.is_empty(), "链接改名无磁盘段");
    assert!(placement(&h.lib, "AN-A", "F01").is_none(), "origin 记录被移除（位置级模型允许）");
    let f03: Vec<&str> = h.lib.folders.items.iter().filter(|p| p.folder == "F03").map(|p| p.name.as_str()).collect();
    assert_eq!(f03, vec!["图", "图.png"], "空名回退资产名（meta 词干）+ 不撞名原样（前插批内保序）");
    assert_eq!(placement(&h.lib, "AN-A", "F02").unwrap().name, "改名.png", "链接改名 trim 生效");

    // 三组全空 = 无操作
    assert!(sync_asset_links(&mut h.lib, "AN-A", &[], &[], &[]).unwrap().is_empty());
    // 位置归 0 护栏：F02/F03 共 3 条全删报错
    let all = vec![
      SyncLinkRemove { folder_id: "F02".into(), origin: false, name: None },
      SyncLinkRemove { folder_id: "F03".into(), origin: false, name: Some("图".into()) },
      SyncLinkRemove { folder_id: "F03".into(), origin: false, name: Some("图.png".into()) },
    ];
    let e = sync_asset_links(&mut h.lib, "AN-A", &[], &all, &[]).unwrap_err();
    assert!(e.contains("全部位置"), "{e}");
    // 误删不存在资产
    assert!(sync_asset_links(&mut h.lib, "AN-X", &[], &[], &[]).is_ok(), "空组先返回不校验资产");
    assert!(sync_asset_links(&mut h.lib, "AN-X", &[SyncLinkAdd { folder_id: "F03".into(), name: "x".into() }], &[], &[]).is_err());
    // 落点文件夹不存在 / 改名定位不存在 = 校验前置报错
    assert!(sync_asset_links(&mut h.lib, "AN-A", &[SyncLinkAdd { folder_id: "F-X".into(), name: "x".into() }], &[], &[]).is_err());
    let bad_rename = vec![SyncLinkRename { folder_id: "F-X".into(), origin: false, old_name: None, new_name: "x.png".into() }];
    assert!(sync_asset_links(&mut h.lib, "AN-A", &[], &[], &bad_rename).is_err());
    // oldName 不匹配 = 定位失败
    let bad_rename2 = vec![SyncLinkRename { folder_id: "F02".into(), origin: false, old_name: Some("不存在.png".into()), new_name: "x.png".into() }];
    assert!(sync_asset_links(&mut h.lib, "AN-A", &[], &[], &bad_rename2).is_err());
  }

  /// 重复链接精确到行（2026-08-13 重复放开后弹窗平铺多行）：同文件夹同 origin 两条链接，
  /// 删 / 改带 name 只命中目标行
  #[test]
  fn sync_asset_links_duplicate_rows_precise() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01");
    add_asset(&mut h, "AN-A", "图.png", "F01", None);
    // 同文件夹再来两条重复链接
    ops::add_placement(&mut h.lib, "AN-A".into(), "F01".into(), "图-链A.png".into(), false, &mut HashMap::new());
    ops::add_placement(&mut h.lib, "AN-A".into(), "F01".into(), "图-链B.png".into(), false, &mut HashMap::new());
    finish(&mut h.lib).unwrap();

    // 删 链A（不动 链B）；改 链B → 新名（不动 origin 图.png）
    let removes = vec![SyncLinkRemove { folder_id: "F01".into(), origin: false, name: Some("图-链A.png".into()) }];
    let renames = vec![SyncLinkRename { folder_id: "F01".into(), origin: false, old_name: Some("图-链B.png".into()), new_name: "新名.png".into() }];
    sync_asset_links(&mut h.lib, "AN-A", &[], &removes, &renames).unwrap();
    let names: Vec<&str> = h.lib.folders.items.iter().filter(|p| p.folder == "F01").map(|p| p.name.as_str()).collect();
    assert!(names.contains(&"图.png"), "origin 行不动");
    assert!(!names.contains(&"图-链A.png"), "链A 被精确移除");
    assert!(names.contains(&"新名.png"), "链B 精确改名");
    assert!(!names.contains(&"图-链B.png"));
  }

  /// 将此文件设为原始文件（6.2 四轮走查③）：链接行置 origin，原 origin 摘标（origin 恒 ≤ 1）；
  /// 仅标记不动磁盘；幂等（已是 origin = false）；重复链接带 name 精确到行
  #[test]
  fn set_placement_origin_transfers_flag() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01");
    add_folder(&mut h.lib, "F02");
    add_asset(&mut h, "AN-A", "图.png", "F01", None);
    ops::add_placement(&mut h.lib, "AN-A".into(), "F02".into(), "图-链.png".into(), false, &mut HashMap::new());
    ops::add_placement(&mut h.lib, "AN-A".into(), "F02".into(), "图-链2.png".into(), false, &mut HashMap::new());
    finish(&mut h.lib).unwrap();

    // 重复链接不带 name 也能定位（三元组第一条），带 name 精确到行
    assert!(set_placement_origin(&mut h.lib, "AN-A", "F02", Some("图-链2.png")).unwrap());
    let origins: Vec<&str> = h.lib.folders.items.iter().filter(|p| p.asset == "AN-A" && p.origin).map(|p| p.name.as_str()).collect();
    assert_eq!(origins, vec!["图-链2.png"], "origin 恒 ≤ 1 且为指定行");
    assert!(placement(&h.lib, "AN-A", "F01").is_some_and(|p| !p.origin), "原 origin 行摘标保留");
    // 幂等：目标行已是 origin = false（不带 name 时三元组优先命中其余 false 行，会再转移——
    // 调用方（链接卡右键）恒对 origin=false 卡片操作，无此路径）
    assert!(!set_placement_origin(&mut h.lib, "AN-A", "F02", Some("图-链2.png")).unwrap());
    // 目标不存在报错
    assert!(set_placement_origin(&mut h.lib, "AN-A", "F99", None).is_err());
    assert!(set_placement_origin(&mut h.lib, "AN-X", "F02", None).is_err());
  }

  /// 链接归并（PRD 03_查找文件-04 屏 2 确认）：保留者 = origin → 链接全清进回收站；
  /// 保留者 = 链接 → 提升 origin，旧 origin 与其余链接全清——资产只剩保留者一个位置
  #[test]
  fn consolidate_keeps_single_origin() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01");
    add_folder(&mut h.lib, "F02");
    add_asset(&mut h, "AN-A", "图.png", "F01", None);
    ops::add_placement(&mut h.lib, "AN-A".into(), "F02".into(), "图-链A.png".into(), false, &mut HashMap::new());
    ops::add_placement(&mut h.lib, "AN-A".into(), "F02".into(), "图-链B.png".into(), false, &mut HashMap::new());
    finish(&mut h.lib).unwrap();

    // 保留 origin：两条链接进回收站，origin 不动
    let cleared = consolidate_asset_links(
      &mut h.lib,
      &[PlacementRef { asset_id: "AN-A".into(), folder_id: "F01".into(), origin: true, name: Some("图.png".into()) }],
    )
    .unwrap();
    assert_eq!(cleared, 2);
    let live: Vec<&Placement> = h.lib.folders.items.iter().filter(|p| p.asset == "AN-A").collect();
    assert_eq!(live.len(), 1, "只剩保留者");
    assert!(live[0].origin && live[0].folder == "F01");
    assert_eq!(h.lib.recycle.items.iter().find(|r| r.asset == "AN-A").map(|r| r.placements.len()), Some(2), "链接进回收站");

    // 还原后改保留链接：链接提升 origin，旧 origin 与其余链接全清
    crate::services::recycle::restore_recycle(&mut h.lib, &["AN-A".to_string()]).unwrap();
    let cleared = consolidate_asset_links(
      &mut h.lib,
      &[PlacementRef { asset_id: "AN-A".into(), folder_id: "F02".into(), origin: false, name: Some("图-链B.png".into()) }],
    )
    .unwrap();
    assert_eq!(cleared, 2, "旧 origin + 另一链接同清");
    let live: Vec<&Placement> = h.lib.folders.items.iter().filter(|p| p.asset == "AN-A").collect();
    assert_eq!(live.len(), 1);
    assert!(live[0].origin && live[0].name == "图-链B.png" && live[0].folder == "F02", "保留链接已提升 origin");
    // 陈旧保留者跳过
    let cleared = consolidate_asset_links(
      &mut h.lib,
      &[PlacementRef { asset_id: "AN-A".into(), folder_id: "F99".into(), origin: false, name: None }],
    )
    .unwrap();
    assert_eq!(cleared, 0);
  }
}
