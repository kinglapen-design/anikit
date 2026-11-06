//! 文件夹删除服务（7.3 T7 死命令处置，2026-11-03 收窄）：
//! - delete_folders：删除文件夹（子树全部 Placement 改挂 root = 素材进未分类 + 子树节点与
//!   档案记录删除，仅改数据，撤销由命令层接入）。单例 delete_folder 命令 = 本函数特例委托。
//! - 旧 delete_assets_permanently / delete_folder(permanent=true) 已随 T7 删除——回收站
//!   彻底清除走 services/recycle.rs（delete_recycle / clear_recycle），本模块不留物理删除。

use std::collections::HashSet;

use serde::Serialize;

use crate::core::library::{self, Library};
use crate::services::import::ROOT_FOLDER_ID;
use crate::services::operations::{finish, rewrite_orders, sync_folder_index};

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
  lib.archives.archives.retain(|a| !subtree_ref.contains(a.folder_id.as_str()));
  sync_folder_index(lib);
  rewrite_orders(lib, ROOT_FOLDER_ID);
  finish(lib)?;
  library::save_archives(lib).map_err(|e| e.to_string())?;
  log::info!("删除 {} 个文件夹（{moved} 个素材移入未分类，{folder_count} 个文件夹节点）", roots.len());
  Ok(DeleteFolderReport { moved_placements: moved, deleted_folders: folder_count, ..empty_report() })
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;
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
  fn delete_folder_recycle_mode_moves_to_root_and_undoable_shape() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F-TOP", None);
    add_folder(&mut h.lib, "F-SUB", Some("F-TOP"));
    add_asset(&mut h, "AN-A", "a.png", "F-SUB");
    add_asset(&mut h, "AN-B", "a.png", "root"); // root 已有同名 → 唯一化

    let rep = delete_folders(&mut h.lib, &["F-TOP".into()]).unwrap();
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
    assert!(delete_folders(&mut h.lib, &["F-不存在".into()]).is_err());
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

    let rep = delete_folders(&mut h.lib, &["F-TOP".into()]).unwrap();
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

}
