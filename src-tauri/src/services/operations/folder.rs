//! 文件夹操作（6.1）：新建（支持插到指定兄弟之后）与同级排序。
//! Folder 展示按 order 排序（commands::import::folder_tree），排序只重写同级 order 为连续序号。

use std::collections::HashSet;

use crate::core::library::{self, Library};
use crate::core::models::Folder;
use crate::services::import as ops;
use crate::services::import::ROOT_FOLDER_ID;

use super::parent_key;

/// 新建文件夹：after_folder_id 给了就插到该兄弟之后（同级后续 order 顺移 +1），否则末尾。
/// 名称走库名同一套 Windows 命名规则（前后端同规则，命令接口统一把关）
pub fn create_folder(
  lib: &mut Library,
  parent: Option<&str>,
  name: &str,
  after_folder_id: Option<&str>,
) -> Result<Folder, String> {
  let name = name.trim();
  library::validate_library_name(name).map_err(|e| e.to_string())?;
  let parent_key = parent_key(parent);
  if let Some(p) = &parent_key {
    if !lib.index.folders.contains_key(p) {
      return Err("父文件夹不存在，可能已被删除".to_string());
    }
  }
  let folder = match after_folder_id {
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
      let folder = Folder { id: ops::new_folder_id(), name: name.to_string(), parent: parent_key, order: insert_order };
      lib.folders.folders.push(folder.clone());
      lib.index.folders.insert(folder.id.clone(), folder.clone());
      folder
    }
  };
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

/// Folder 数据改动后索引同步（folders.folders 为事实来源，全量重建映射）
fn sync_folder_index(lib: &mut Library) {
  lib.index.folders = lib.folders.folders.iter().map(|f| (f.id.clone(), f.clone())).collect();
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::json_io::read_json;
  use crate::core::library::{FOLDERS_FILE};
  use crate::core::models::FoldersFile;

  struct Harness {
    _dir: tempfile::TempDir,
    lib: Library,
  }

  fn make_lib() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("测试库");
    let lib = library::create(&root).unwrap();
    Harness { _dir: dir, lib }
  }

  fn add_folder(lib: &mut Library, id: &str, name: &str, parent: Option<&str>, order: i32) {
    let folder = Folder { id: id.into(), name: name.into(), parent: parent.map(|s| s.into()), order };
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
    let f = create_folder(&mut h.lib, None, "动画", None).unwrap();
    assert!(f.id.starts_with('F'));
    assert_eq!((f.parent, f.order), (None, 1), "末尾 order = 同级 max+1");
    assert!(h.lib.index.folders.contains_key(&f.id), "索引同步");
    // "root" 视为顶层；落盘验证
    let f2 = create_folder(&mut h.lib, Some("root"), "参考", None).unwrap();
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
    let f = create_folder(&mut h.lib, None, "新", Some("F01")).unwrap();
    assert_eq!(f.order, 1, "插到 F01 之后");
    assert_eq!(top_orders(&h.lib), vec![("F01", 0), (f.id.as_str(), 1), ("F02", 2), ("F03", 3)], "后续兄弟顺移 +1");
    // 子层级互不影响
    add_folder(&mut h.lib, "F01-1", "子", Some("F01"), 0);
    let child = create_folder(&mut h.lib, Some("F01"), "子2", Some("F01-1")).unwrap();
    assert_eq!(child.parent.as_deref(), Some("F01"));
    // 兄弟不存在 / 父不存在 / 非法名称
    assert!(create_folder(&mut h.lib, None, "x", Some("F-不存在")).is_err());
    assert!(create_folder(&mut h.lib, Some("F-不存在"), "x", None).is_err());
    assert!(create_folder(&mut h.lib, None, "坏:名", None).is_err());
    assert!(create_folder(&mut h.lib, None, "   ", None).is_err());
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
}
