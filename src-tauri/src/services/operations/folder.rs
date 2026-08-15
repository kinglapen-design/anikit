//! 文件夹操作（6.1）：新建（支持插到指定兄弟之后）、同级排序与重设父级。
//! Folder 展示按 order 排序（commands::import::folder_tree），排序只重写同级 order 为连续序号。

use std::collections::HashSet;

use crate::core::library::{self, Library};
use crate::core::models::{Folder, ARCHIVE_KIND};
use crate::services::import as ops;
use crate::services::import::ROOT_FOLDER_ID;

use super::{check_folder, parent_key};

/// 新建文件夹：after_folder_id 给了就插到该兄弟之后（同级后续 order 顺移 +1），否则末尾。
/// 名称走库名同一套 Windows 命名规则（前后端同规则，命令接口统一把关）。
/// block_id 仅顶级新建（parent = None / "root"）生效（2026-08-14 区块子系统）：写入 Folder.block，
/// 且所在区块有默认图标时继承到 Folder.icon；非顶级新建忽略该参数
pub fn create_folder(
  lib: &mut Library,
  parent: Option<&str>,
  name: &str,
  after_folder_id: Option<&str>,
  block_id: Option<&str>,
) -> Result<Folder, String> {
  let name = name.trim();
  library::validate_library_name(name).map_err(|e| e.to_string())?;
  let parent_key = parent_key(parent);
  if let Some(p) = &parent_key {
    match lib.index.folders.get(p) {
      None => return Err("父文件夹不存在，可能已被删除".to_string()),
      // 档案层级约束（2026-08-15 档案子系统，PRD 02_文件整理 03）：档案内不允许子级文件夹
      Some(f) if f.kind.as_deref() == Some(ARCHIVE_KIND) => return Err("档案下不允许出现子级文件夹".to_string()),
      _ => {}
    }
  }
  // 区块归属与默认图标继承（仅顶级；block_id 指向不存在区块报错）
  let (block, icon) = match (&parent_key, block_id) {
    (None, Some(b)) => {
      let blk = lib
        .folders
        .blocks
        .iter()
        .find(|x| x.id == b)
        .ok_or_else(|| "目标区块不存在，可能已被删除".to_string())?;
      (Some(b.to_string()), blk.icon.clone())
    }
    _ => (None, None),
  };
  let mut folder = match after_folder_id {
    None => {
      let id = ops::create_folder(lib, parent.unwrap_or(ROOT_FOLDER_ID), name);
      lib.index.folders[&id].clone()
    }
    Some(after_id) => {
      let anchor = lib
        .folders
        .folders
        .iter()
        .find(|f| f.id == after_id && f.parent == parent_key)
        .ok_or_else(|| "插入位置无效：目标兄弟文件夹不存在".to_string())?;
      let insert_order = anchor.order + 1;
      for f in &mut lib.folders.folders {
        if f.parent == parent_key && f.order >= insert_order {
          f.order += 1;
        }
      }
      let folder =
        Folder { id: ops::new_folder_id(), name: name.to_string(), parent: parent_key, block: None, icon: None, kind: None, order: insert_order };
      lib.folders.folders.push(folder.clone());
      lib.index.folders.insert(folder.id.clone(), folder.clone());
      folder
    }
  };
  // 区块归属写回（两条创建路径统一在此落 block/icon；均为 None 时无操作）
  if block.is_some() || icon.is_some() {
    folder.block = block;
    folder.icon = icon;
    if let Some(f) = lib.folders.folders.iter_mut().find(|f| f.id == folder.id) {
      f.block = folder.block.clone();
      f.icon = folder.icon.clone();
    }
    lib.index.folders.insert(folder.id.clone(), folder.clone());
  }
  sync_folder_index(lib); // 顺移过 order 的兄弟同步进索引
  library::save_folders(lib).map_err(|e| e.to_string())?;
  log::info!("新建文件夹：{}（{:?} 下）", folder.name, folder.parent);
  Ok(folder)
}

/// 同级排序：folder_ids（须同属 parent 层级）整体移到 before_folder_id 之前（None = 末尾），
/// 随后同级 order 重写为连续 0..n。混排护栏：Folder 与 Placement 各自独立排序（前端分开调）
pub fn reorder_folders(
  lib: &mut Library,
  folder_ids: &[String],
  parent: Option<&str>,
  before_folder_id: Option<&str>,
) -> Result<(), String> {
  if folder_ids.is_empty() {
    return Ok(());
  }
  let parent_key = parent_key(parent);
  // 同级兄弟的 vec 下标，按 order 排序（当前显示顺序）
  let mut siblings: Vec<usize> = lib
    .folders
    .folders
    .iter()
    .enumerate()
    .filter(|(_, f)| f.parent == parent_key)
    .map(|(i, _)| i)
    .collect();
  siblings.sort_by_key(|&i| lib.folders.folders[i].order);

  let mut moving: Vec<usize> = Vec::with_capacity(folder_ids.len());
  let mut seen = HashSet::new();
  for id in folder_ids {
    if !seen.insert(id) {
      return Err("调整列表存在重复文件夹".to_string());
    }
    let pos = siblings
      .iter()
      .position(|&i| lib.folders.folders[i].id == *id)
      .ok_or_else(|| "文件夹不在该层级，可能已被移动".to_string())?;
    moving.push(siblings[pos]);
  }
  let moving_set: HashSet<usize> = moving.iter().copied().collect();
  let remaining: Vec<usize> = siblings.iter().copied().filter(|i| !moving_set.contains(i)).collect();

  let insert_at = match before_folder_id {
    None => remaining.len(),
    Some(before) => remaining
      .iter()
      .position(|&i| lib.folders.folders[i].id == *before)
      .ok_or_else(|| "插入位置无效：目标兄弟文件夹不存在".to_string())?,
  };
  let mut sequence = remaining[..insert_at].to_vec();
  sequence.extend(moving);
  sequence.extend(&remaining[insert_at..]);
  for (n, &i) in sequence.iter().enumerate() {
    lib.folders.folders[i].order = n as i32;
  }
  sync_folder_index(lib);
  library::save_folders(lib).map_err(|e| e.to_string())
}

/// 文件夹重命名（2026-07-29 复验三轮）：仅改数据不动磁盘（Folder 是数据层分组，无磁盘实体）。
/// 名称走库名同一套 Windows 命名规则（validate_library_name）；同名 = 幂等无操作
pub fn rename_folder(lib: &mut Library, folder_id: &str, new_name: &str) -> Result<(), String> {
  let name = new_name.trim();
  library::validate_library_name(name).map_err(|e| e.to_string())?;
  let folder = lib
    .folders
    .folders
    .iter_mut()
    .find(|f| f.id == folder_id)
    .ok_or_else(|| "目标文件夹不存在，可能已被删除".to_string())?;
  if folder.name == name {
    return Ok(());
  }
  folder.name = name.to_string();
  sync_folder_index(lib);
  library::save_folders(lib).map_err(|e| e.to_string())?;
  log::info!("重命名文件夹 {folder_id} 为 {name}");
  Ok(())
}

/// Folder 数据改动后索引同步（folders.folders 为事实来源，全量重建映射）
fn sync_folder_index(lib: &mut Library) {
  lib.index.folders = lib.folders.folders.iter().map(|f| (f.id.clone(), f.clone())).collect();
}

/// 文件夹重设父级（仅改数据不动磁盘）：target_parent None / "root" = 顶层。
/// 护栏：目标是自身或子孙 → 循环整条报错；同父 / 陈旧 ID 跳过不计数。
/// 落点：before_folder_id 给了 = 插到该兄弟（须为 target_parent 直接子级）之前——
///       同级 order >= 锚点的兄弟顺移 +移动数，移动组从锚点 order 起按传入顺序连续分配
///       （跨父级的上/下 1/3 落点定位，2026-07-29 复验：嵌套文件夹拖出）；
///       None = 排到目标父级现有子文件夹末尾（同级 max+1 起连续分配）
/// target_block（2026-08-14 区块子系统）三态：None = 保持原区块不动；Some(id) = 落根级时
/// 写入 Folder.block（移回「目录」传 Some("dir")）；落进文件夹内（target_parent 有值）时必须为 None
pub fn move_folders(
  lib: &mut Library,
  folder_ids: &[String],
  target_parent: Option<&str>,
  before_folder_id: Option<&str>,
  target_block: Option<&str>,
) -> Result<usize, String> {
  let target_key = parent_key(target_parent);
  if let Some(t) = &target_key {
    check_folder(lib, t)?;
    // 档案层级约束（2026-08-15 档案子系统）：档案不作为父级并入；档案自身当移动 / 排序对象不受限
    if lib.index.folders.get(t).map(|f| f.kind.as_deref()) == Some(Some(ARCHIVE_KIND)) {
      return Err("档案下不允许出现子级文件夹".to_string());
    }
    if target_block.is_some() {
      return Err("移入文件夹内不能同时指定目标区块".to_string());
    }
  }
  if let Some(b) = target_block {
    if !lib.folders.blocks.iter().any(|x| x.id == b) {
      return Err("目标区块不存在，可能已被删除".to_string());
    }
  }
  // 锚点校验先于落点（先全量验证后落点）：须为 target_parent 的直接子级
  let anchor_order = match before_folder_id {
    None => None,
    Some(before) => Some(
      lib
        .index
        .folders
        .get(before)
        .filter(|f| f.parent == target_key)
        .map(|f| f.order)
        .ok_or_else(|| "插入位置无效：目标兄弟文件夹不存在".to_string())?,
    ),
  };
  let mut seen = HashSet::new();
  let mut moving: Vec<String> = Vec::new();
  for id in folder_ids {
    if !seen.insert(id) {
      continue;
    }
    let folder = match lib.index.folders.get(id) {
      Some(f) => f,
      None => continue, // 陈旧 ID 跳过
    };
    if folder.parent == target_key {
      // 原地移动跳过；但落根级且指定了不同目标区块时不算原地——拖到区块标题 =
      // 移入该区块根级末尾（2026-08-14 区块子系统），照常走落点改区块归属
      let block_change = target_key.is_none() && target_block.is_some() && folder.block.as_deref() != target_block;
      if !block_change {
        continue;
      }
    }
    if is_self_or_descendant(lib, id, target_key.as_deref()) {
      return Err("不能移动到自身或其子文件夹内".to_string());
    }
    moving.push(id.clone());
  }
  if moving.is_empty() {
    return Ok(0);
  }
  place_moved(lib, &moving, &target_key, anchor_order);
  // 落根级且指定目标区块 → 写入 Folder.block（2026-08-14 区块子系统）
  if target_key.is_none() {
    if let Some(b) = target_block {
      for f in &mut lib.folders.folders {
        if moving.contains(&f.id) {
          f.block = Some(b.to_string());
        }
      }
    }
  }
  sync_folder_index(lib);
  library::save_folders(lib).map_err(|e| e.to_string())?;
  log::info!("移动 {} 个文件夹到 {:?}", moving.len(), target_key);
  Ok(moving.len())
}

/// 落点分配：anchor_order Some = 插到锚点前（同级 order >= 锚点的兄弟顺移 +移动数，
/// 移动组从锚点 order 起连续分配）；None = 末尾（同级 max+1 起，按传入顺序连续分配）
fn place_moved(lib: &mut Library, moving: &[String], target_key: &Option<String>, anchor_order: Option<i32>) {
  let base = match anchor_order {
    Some(base) => {
      let shift = moving.len() as i32;
      for f in &mut lib.folders.folders {
        if f.parent == *target_key && f.order >= base {
          f.order += shift;
        }
      }
      base
    }
    None => {
      lib.folders.folders.iter().filter(|f| f.parent == *target_key).map(|f| f.order).max().unwrap_or(-1) + 1
    }
  };
  for (i, id) in moving.iter().enumerate() {
    if let Some(f) = lib.folders.folders.iter_mut().find(|f| &f.id == id) {
      f.parent = target_key.clone();
      f.order = base + i as i32;
    }
  }
}

/// target 是否为 folder_id 自身或其子孙（沿 target 父链上溯，命中即循环）
fn is_self_or_descendant(lib: &Library, folder_id: &str, target: Option<&str>) -> bool {
  let mut cur = target.map(|s| s.to_string());
  while let Some(id) = cur {
    if id == folder_id {
      return true;
    }
    cur = lib.index.folders.get(&id).and_then(|f| f.parent.clone());
  }
  false
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::json_io::read_json;
  use crate::core::library::{FOLDERS_FILE};
  use crate::core::models::{Block, FoldersFile, DEFAULT_BLOCK_ID};
  use crate::core::undo::UndoManager;

  struct Harness {
    _dir: tempfile::TempDir,
    lib: Library,
  }

  fn make_lib() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    Harness { _dir: dir, lib }
  }

  fn add_folder(lib: &mut Library, id: &str, name: &str, parent: Option<&str>, order: i32) {
    let folder = Folder { id: id.into(), name: name.into(), parent: parent.map(|s| s.into()), block: None, icon: None, kind: None, order };
    lib.folders.folders.push(folder.clone());
    lib.index.folders.insert(id.into(), folder);
  }

  fn top_orders(lib: &Library) -> Vec<(&str, i32)> {
    let mut v: Vec<(&str, i32)> = lib
      .folders
      .folders
      .iter()
      .filter(|f| f.parent.is_none())
      .map(|f| (f.id.as_str(), f.order))
      .collect();
    v.sort_by_key(|(_, o)| *o);
    v
  }

  #[test]
  fn create_folder_appends_at_end() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "人物", None, 0);
    let f = create_folder(&mut h.lib, None, "动画", None, None).unwrap();
    assert!(f.id.starts_with('F'));
    assert_eq!((f.parent, f.order), (None, 1), "末尾 order = 同级 max+1");
    assert!(h.lib.index.folders.contains_key(&f.id), "索引同步");
    // "root" 视为顶层；落盘验证
    let f2 = create_folder(&mut h.lib, Some("root"), "参考", None, None).unwrap();
    assert_eq!((f2.parent, f2.order), (None, 2));
    let disk: FoldersFile = read_json(&h.lib.root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(disk.folders.len(), 3);
  }

  #[test]
  fn create_folder_inserts_after_sibling() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "甲", None, 0);
    add_folder(&mut h.lib, "F02", "乙", None, 1);
    add_folder(&mut h.lib, "F03", "丙", None, 2);
    let f = create_folder(&mut h.lib, None, "新", Some("F01"), None).unwrap();
    assert_eq!(f.order, 1, "插到 F01 之后");
    assert_eq!(top_orders(&h.lib), vec![("F01", 0), (f.id.as_str(), 1), ("F02", 2), ("F03", 3)], "后续兄弟顺移 +1");
    // 子层级互不影响
    add_folder(&mut h.lib, "F01-1", "子", Some("F01"), 0);
    let child = create_folder(&mut h.lib, Some("F01"), "子2", Some("F01-1"), None).unwrap();
    assert_eq!(child.parent.as_deref(), Some("F01"));
    // 兄弟不存在 / 父不存在 / 非法名称
    assert!(create_folder(&mut h.lib, None, "x", Some("F-不存在"), None).is_err());
    assert!(create_folder(&mut h.lib, Some("F-不存在"), "x", None, None).is_err());
    assert!(create_folder(&mut h.lib, None, "坏:名", None, None).is_err());
    assert!(create_folder(&mut h.lib, None, "   ", None, None).is_err());
  }

  #[test]
  fn reorder_folders_moves_before_and_compacts_orders() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "甲", None, 0);
    add_folder(&mut h.lib, "F02", "乙", None, 1);
    add_folder(&mut h.lib, "F03", "丙", None, 2);

    // F03 移到 F01 之前
    reorder_folders(&mut h.lib, &["F03".into()], None, Some("F01")).unwrap();
    assert_eq!(top_orders(&h.lib), vec![("F03", 0), ("F01", 1), ("F02", 2)]);
    assert_eq!(h.lib.index.folders["F03"].order, 0, "索引同步");

    // 多选移到末尾（before = None）
    reorder_folders(&mut h.lib, &["F03".into(), "F01".into()], Some("root"), None).unwrap();
    assert_eq!(top_orders(&h.lib), vec![("F02", 0), ("F03", 1), ("F01", 2)]);

    // 护栏：不属于该层级 / 插入点无效 / 重复
    add_folder(&mut h.lib, "F01-1", "子", Some("F01"), 0);
    assert!(reorder_folders(&mut h.lib, &["F01-1".into()], None, None).is_err());
    assert!(reorder_folders(&mut h.lib, &["F01".into()], None, Some("F-不存在")).is_err());
    assert!(reorder_folders(&mut h.lib, &["F01".into(), "F01".into()], None, None).is_err());
  }

  #[test]
  fn move_folders_basic_moves() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "甲", None, 0);
    add_folder(&mut h.lib, "F02", "乙", None, 1);
    add_folder(&mut h.lib, "F01-1", "子", Some("F01"), 0);

    // 顶层 → 子级
    let n = move_folders(&mut h.lib, &["F02".into()], Some("F01"), None, None).unwrap();
    assert_eq!(n, 1);
    assert_eq!(h.lib.index.folders["F02"].parent.as_deref(), Some("F01"), "索引同步");
    assert_eq!(h.lib.index.folders["F02"].order, 1, "排到目标子级末尾");

    // 子级 → 顶层（None = 顶层）
    let n = move_folders(&mut h.lib, &["F02".into()], None, None, None).unwrap();
    assert_eq!(n, 1);
    assert_eq!((h.lib.index.folders["F02"].parent.clone(), h.lib.index.folders["F02"].order), (None, 1));

    // 跨父级 + "root" 视为顶层
    let n = move_folders(&mut h.lib, &["F01-1".into()], Some("root"), None, None).unwrap();
    assert_eq!(n, 1);
    assert_eq!(h.lib.index.folders["F01-1"].parent, None);
    // 落盘验证
    let disk: FoldersFile = read_json(&h.lib.root.join(FOLDERS_FILE)).unwrap();
    let f = disk.folders.iter().find(|f| f.id == "F01-1").unwrap();
    assert_eq!(f.parent, None);
  }

  #[test]
  fn move_folders_appends_continuous_orders_in_input_order() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F-T", "目标", None, 0);
    add_folder(&mut h.lib, "F-A", "甲", None, 1);
    add_folder(&mut h.lib, "F-B", "乙", None, 2);
    add_folder(&mut h.lib, "F-C1", "子1", Some("F-T"), 0);
    add_folder(&mut h.lib, "F-C2", "子2", Some("F-T"), 1);

    let n = move_folders(&mut h.lib, &["F-B".into(), "F-A".into()], Some("F-T"), None, None).unwrap();
    assert_eq!(n, 2);
    assert_eq!(h.lib.index.folders["F-B"].order, 2, "同级 max+1 起");
    assert_eq!(h.lib.index.folders["F-A"].order, 3, "按传入顺序连续分配");
    assert_eq!(h.lib.index.folders["F-C2"].order, 1, "原有子级不动");
  }

  #[test]
  fn move_folders_inserts_before_anchor_and_shifts_siblings() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F-T", "目标", None, 0);
    add_folder(&mut h.lib, "F-X", "甲", None, 1);
    add_folder(&mut h.lib, "F-Y", "乙", None, 2);
    add_folder(&mut h.lib, "F-C1", "子1", Some("F-T"), 0);
    add_folder(&mut h.lib, "F-C2", "子2", Some("F-T"), 1);
    add_folder(&mut h.lib, "F-C3", "子3", Some("F-T"), 2);

    // F-X、F-Y 跨父级插到 F-C2 之前：锚点及后续兄弟顺移 +移动数，移动组从锚点 order 起连续分配
    let n = move_folders(&mut h.lib, &["F-X".into(), "F-Y".into()], Some("F-T"), Some("F-C2"), None).unwrap();
    assert_eq!(n, 2);
    assert_eq!(h.lib.index.folders["F-X"].order, 1, "从锚点 order 起");
    assert_eq!(h.lib.index.folders["F-Y"].order, 2, "按传入顺序连续分配");
    assert_eq!(h.lib.index.folders["F-C1"].order, 0, "锚点前的兄弟不动");
    assert_eq!(h.lib.index.folders["F-C2"].order, 3, "锚点及后续顺移 +移动数");
    assert_eq!(h.lib.index.folders["F-C3"].order, 4);
    // 落盘验证
    let disk: FoldersFile = read_json(&h.lib.root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(disk.folders.iter().find(|f| f.id == "F-X").unwrap().order, 1);
  }

  #[test]
  fn move_folders_anchor_must_be_direct_child_of_target() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "甲", None, 0);
    add_folder(&mut h.lib, "F01-1", "子", Some("F01"), 0);
    add_folder(&mut h.lib, "F02", "乙", None, 1);

    // 锚点不是 target_parent 直接子级：报错且不动数据
    let e = move_folders(&mut h.lib, &["F02".into()], None, Some("F01-1"), None).unwrap_err();
    assert!(e.contains("插入位置无效"), "{e}");
    assert_eq!(h.lib.index.folders["F02"].order, 1, "数据未被半截改动");
    // 锚点自身不在目标父级（F02 是顶层）/ 锚点不存在
    assert!(move_folders(&mut h.lib, &["F02".into()], Some("F01"), Some("F02"), None).is_err());
    assert!(move_folders(&mut h.lib, &["F02".into()], Some("F01"), Some("F-不存在"), None).is_err());
  }

  /// 2026-07-29 复验：嵌套文件夹拖出——跨父级落顶层某文件夹之前
  #[test]
  fn move_folders_pulls_nested_folder_out_to_top_before_anchor() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "甲", None, 0);
    add_folder(&mut h.lib, "F02", "乙", None, 1);
    add_folder(&mut h.lib, "F01-1", "子", Some("F01"), 0);

    let n = move_folders(&mut h.lib, &["F01-1".into()], None, Some("F02"), None).unwrap();
    assert_eq!(n, 1);
    assert_eq!(h.lib.index.folders["F01-1"].parent, None, "拖回顶层");
    assert_eq!(top_orders(&h.lib), vec![("F01", 0), ("F01-1", 1), ("F02", 2)], "插到 F02 之前、F02 顺移");
  }

  #[test]
  fn move_folders_rejects_cycles() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "甲", None, 0);
    add_folder(&mut h.lib, "F02", "乙", None, 1);
    add_folder(&mut h.lib, "F01-1", "子", Some("F01"), 0);
    add_folder(&mut h.lib, "F01-1-1", "孙", Some("F01-1"), 0);

    // 移入自身 / 子孙：整条报错且不动数据
    let e = move_folders(&mut h.lib, &["F01".into()], Some("F01"), None, None).unwrap_err();
    assert!(e.contains("不能移动到自身或其子文件夹内"), "{e}");
    let e = move_folders(&mut h.lib, &["F01".into()], Some("F01-1-1"), None, None).unwrap_err();
    assert!(e.contains("不能移动到自身或其子文件夹内"), "{e}");
    // 批量中一条循环 → 整条失败，先验证后落点（F02 不被半截移动）
    assert!(move_folders(&mut h.lib, &["F02".into(), "F01".into()], Some("F01-1"), None, None).is_err());
    assert_eq!(h.lib.index.folders["F02"].parent, None);
    assert_eq!(h.lib.index.folders["F01"].parent, None);
  }

  #[test]
  fn move_folders_skips_same_parent_and_stale_ids() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "甲", None, 0);
    add_folder(&mut h.lib, "F01-1", "子", Some("F01"), 0);

    // 同父跳过不计数，数据不变
    let n = move_folders(&mut h.lib, &["F01-1".into()], Some("F01"), None, None).unwrap();
    assert_eq!(n, 0);
    assert_eq!((h.lib.index.folders["F01-1"].parent.clone(), h.lib.index.folders["F01-1"].order), (Some("F01".into()), 0));
    // 陈旧 ID 跳过；重复 ID 只计一次
    let n = move_folders(&mut h.lib, &["F-不存在".into(), "F01-1".into(), "F01-1".into()], None, None, None).unwrap();
    assert_eq!(n, 1);
    assert_eq!(h.lib.index.folders["F01-1"].parent, None);
    // 目标父级不存在 → 报错
    assert!(move_folders(&mut h.lib, &["F01".into()], Some("F-不存在"), None, None).is_err());
  }

  /// 区块参数（2026-08-14 区块子系统）：顶级新建带 block_id 写入 Folder.block 并继承区块默认图标；
  /// 区块无默认图标则 icon 不写；block_id 指向不存在区块报错；非顶级新建忽略该参数
  #[test]
  fn create_folder_with_block_inherits_icon() {
    let mut h = make_lib();
    h.lib.folders.blocks.push(Block { id: "B01".into(), name: "角色".into(), icon: Some("icon-role".into()), order: 1 });
    h.lib.folders.blocks.push(Block { id: "B02".into(), name: "场景".into(), icon: None, order: 2 });
    add_folder(&mut h.lib, "F01", "甲", None, 0);

    let f = create_folder(&mut h.lib, None, "乙", None, Some("B01")).unwrap();
    assert_eq!(f.block.as_deref(), Some("B01"));
    assert_eq!(f.icon.as_deref(), Some("icon-role"), "继承区块默认图标");
    assert_eq!(h.lib.index.folders[&f.id].block.as_deref(), Some("B01"), "索引同步");
    // 区块无默认图标 → Folder.icon 保持 None
    let f2 = create_folder(&mut h.lib, None, "丙", None, Some("B02")).unwrap();
    assert_eq!((f2.block.as_deref(), f2.icon.clone()), (Some("B02"), None));
    // 落盘验证
    let disk: FoldersFile = read_json(&h.lib.root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(disk.folders.iter().find(|x| x.id == f.id).unwrap().icon.as_deref(), Some("icon-role"));
    // 不存在区块报错；非顶级忽略 block_id（即使 id 无效也不报）
    assert!(create_folder(&mut h.lib, None, "x", None, Some("B-不存在")).is_err());
    let child = create_folder(&mut h.lib, Some("F01"), "子", None, Some("B-不存在")).unwrap();
    assert_eq!((child.block.clone(), child.icon.clone()), (None, None), "非顶级忽略区块参数");
  }

  /// target_block 三态（2026-08-14 区块子系统）：落根级 Some(id) 写入 Folder.block；Some("dir") 移回
  /// 「目录」；None 保持原区块不动；落进文件夹内传 target_block 报错；指向不存在区块报错
  #[test]
  fn move_folders_target_block() {
    let mut h = make_lib();
    h.lib.folders.blocks.push(Block { id: "B01".into(), name: "角色".into(), icon: None, order: 1 });
    add_folder(&mut h.lib, "F01", "甲", None, 0);
    add_folder(&mut h.lib, "F02", "乙", None, 1);
    add_folder(&mut h.lib, "F01-1", "子", Some("F01"), 0);

    // 落根级设置区块（子级拖出到区块根级）
    let n = move_folders(&mut h.lib, &["F01-1".into()], None, None, Some("B01")).unwrap();
    assert_eq!(n, 1);
    assert_eq!(h.lib.index.folders["F01-1"].block.as_deref(), Some("B01"), "索引同步");
    // None = 不动原区块
    move_folders(&mut h.lib, &["F01-1".into()], Some("F02"), None, None).unwrap();
    let n = move_folders(&mut h.lib, &["F01-1".into()], None, None, None).unwrap();
    assert_eq!(n, 1);
    assert_eq!(h.lib.index.folders["F01-1"].block.as_deref(), Some("B01"), "不传 target_block 保持原区块");
    // Some("dir") = 移回「目录」
    move_folders(&mut h.lib, &["F01-1".into()], None, None, Some(DEFAULT_BLOCK_ID)).unwrap();
    assert_eq!(h.lib.index.folders["F01-1"].block.as_deref(), Some(DEFAULT_BLOCK_ID));
    // 落进文件夹内传 target_block → 报错；不存在区块 → 报错
    assert!(move_folders(&mut h.lib, &["F02".into()], Some("F01"), None, Some("B01")).is_err());
    assert!(move_folders(&mut h.lib, &["F02".into()], None, None, Some("B-不存在")).is_err());
  }

  /// 档案层级约束（2026-08-15 档案子系统，PRD 02_文件整理 03）：档案不作为父级——
  /// create_folder / move_folders 目标父级是 archive 报错；档案自身当移动 / 排序对象不受限
  #[test]
  fn archive_rejects_child_folders() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F-A", "档案", None, 0);
    add_folder(&mut h.lib, "F-B", "普通", None, 1);
    // 置 F-A 为档案形态
    h.lib.folders.folders.iter_mut().find(|f| f.id == "F-A").unwrap().kind = Some(ARCHIVE_KIND.into());
    h.lib.index.folders.get_mut("F-A").unwrap().kind = Some(ARCHIVE_KIND.into());

    let e = create_folder(&mut h.lib, Some("F-A"), "子", None, None).unwrap_err();
    assert!(e.contains("档案下不允许出现子级文件夹"), "{e}");
    let e = move_folders(&mut h.lib, &["F-B".into()], Some("F-A"), None, None).unwrap_err();
    assert!(e.contains("档案下不允许出现子级文件夹"), "{e}");
    assert_eq!(h.lib.index.folders["F-B"].parent, None, "拒绝后数据不动");

    // 档案自身当移动对象不受限（移入普通文件夹 / 拖回顶层 / 同级排序）
    let n = move_folders(&mut h.lib, &["F-A".into()], Some("F-B"), None, None).unwrap();
    assert_eq!(n, 1);
    assert_eq!(h.lib.index.folders["F-A"].parent.as_deref(), Some("F-B"));
    let n = move_folders(&mut h.lib, &["F-A".into()], None, None, None).unwrap();
    assert_eq!(n, 1);
    reorder_folders(&mut h.lib, &["F-A".into()], None, Some("F-B")).unwrap();
    assert_eq!(top_orders(&h.lib), vec![("F-A", 0), ("F-B", 1)]);
  }

  /// 撤销接入验证：begin → record_file → 业务 → commit 后 undo 栈有一条，undo 回滚 folders.json
  #[test]
  fn undo_rolls_back_move_folders() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "甲", None, 0);
    add_folder(&mut h.lib, "F02", "乙", None, 1);
    library::save_folders(&h.lib).unwrap(); // 先落盘，撤销快照才含测试数据
    let path = h.lib.root.join(FOLDERS_FILE);

    let mut undo = UndoManager::default();
    undo.begin("移动 1 个文件夹");
    undo.record_file(&path).unwrap();
    move_folders(&mut h.lib, &["F02".into()], Some("F01"), None, None).unwrap();
    undo.commit().unwrap();
    assert!(undo.state().can_undo);
    assert_eq!(undo.state().undo_label.as_deref(), Some("移动 1 个文件夹"));

    undo.undo().unwrap();
    crate::core::library::reload(&mut h.lib).unwrap();
    assert_eq!(h.lib.index.folders["F02"].parent, None, "undo 后回到顶层");
    assert_eq!(h.lib.index.folders["F02"].order, 1);
  }

  /// 带锚点定位的移动同样一步撤销（移动组与顺移的兄弟一并回滚）
  #[test]
  fn undo_rolls_back_move_folders_before_anchor() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "甲", None, 0);
    add_folder(&mut h.lib, "F02", "乙", None, 1);
    add_folder(&mut h.lib, "F01-1", "子", Some("F01"), 0);
    library::save_folders(&h.lib).unwrap();
    let path = h.lib.root.join(FOLDERS_FILE);

    let mut undo = UndoManager::default();
    undo.begin("移动 1 个文件夹");
    undo.record_file(&path).unwrap();
    move_folders(&mut h.lib, &["F01-1".into()], None, Some("F02"), None).unwrap();
    undo.commit().unwrap();

    undo.undo().unwrap();
    crate::core::library::reload(&mut h.lib).unwrap();
    assert_eq!(h.lib.index.folders["F01-1"].parent.as_deref(), Some("F01"), "undo 后回到原父级");
    assert_eq!(h.lib.index.folders["F01-1"].order, 0);
    assert_eq!(top_orders(&h.lib), vec![("F01", 0), ("F02", 1)], "顺移过的兄弟一并回滚");
  }

  /// 重命名（2026-07-29 复验三轮）：基本改名 + 落盘 + 索引同步；同名幂等
  #[test]
  fn rename_folder_basic_and_idempotent() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "人物", None, 0);
    add_folder(&mut h.lib, "F01-1", "子", Some("F01"), 0);

    rename_folder(&mut h.lib, "F01", "角色").unwrap();
    assert_eq!(h.lib.index.folders["F01"].name, "角色", "索引同步");
    assert_eq!(h.lib.index.folders["F01"].order, 0, "改名不动 order / 父级");
    assert_eq!(h.lib.index.folders["F01"].parent, None);
    // 落盘验证
    let disk: FoldersFile = read_json(&h.lib.root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(disk.folders.iter().find(|f| f.id == "F01").unwrap().name, "角色");
    // 子层级互不影响；前后空白 trim；同名幂等无操作
    assert_eq!(h.lib.index.folders["F01-1"].name, "子");
    rename_folder(&mut h.lib, "F01-1", "  站姿  ").unwrap();
    assert_eq!(h.lib.index.folders["F01-1"].name, "站姿");
    rename_folder(&mut h.lib, "F01", "角色").unwrap();
    assert_eq!(h.lib.index.folders["F01"].name, "角色");
  }

  /// 非法名 / 空名 / 不存在目标拒绝（数据不动）
  #[test]
  fn rename_folder_rejects_invalid_names() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "人物", None, 0);

    // 注意："结尾 " trim 后为合法名（与 create_folder 同口径，先 trim 再校验）
    for bad in ["", "   ", "坏:名", "坏/名", "结尾.", "con"] {
      assert!(rename_folder(&mut h.lib, "F01", bad).is_err(), "应拒绝：{bad:?}");
    }
    assert!(rename_folder(&mut h.lib, "F-不存在", "新名").is_err());
    assert_eq!(h.lib.index.folders["F01"].name, "人物", "拒绝后数据不动");
  }

  /// 撤销接入验证（2026-07-29 复验三轮）：begin → record_file → 业务 → commit，undo 回滚 folders.json
  #[test]
  fn undo_rolls_back_rename_folder() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01", "人物", None, 0);
    library::save_folders(&h.lib).unwrap(); // 先落盘，撤销快照才含测试数据
    let path = h.lib.root.join(FOLDERS_FILE);

    let mut undo = UndoManager::default();
    undo.begin("重命名文件夹为 角色");
    undo.record_file(&path).unwrap();
    rename_folder(&mut h.lib, "F01", "角色").unwrap();
    undo.commit().unwrap();
    assert!(undo.state().can_undo);
    assert_eq!(undo.state().undo_label.as_deref(), Some("重命名文件夹为 角色"));

    undo.undo().unwrap();
    crate::core::library::reload(&mut h.lib).unwrap();
    assert_eq!(h.lib.index.folders["F01"].name, "人物", "undo 后恢复原名");
  }
}
