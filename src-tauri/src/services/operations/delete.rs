//! 删除 Placement（6.1；2026-08-19 用户定"护栏拆干净"大简化）：
//! **位置级语义，无"最后一条"概念、无 origin 自动转移**——链接与原始文件同路径对待：
//! - "uncategorized"（从文件夹中移除，Shift+Del）：Placement 挪未分类（folder 改 root），
//!   不删记录、不改 origin 标记、显示名不动（显示名放开 PRD 01 规则 7）；
//! - "recycle"（丢到回收站，Del）：Placement 从 folders.json 移除并进 recycle.json
//!   （按资产聚合条目——同资产已有条目则追加 placements，无则新建）；origin 标记随记录走
//!   （origin 进回收站后资产可以只剩活链接，不主动转移——手动转移归阶段六「设为原始文件」）。
//!
//! 物理删除只在回收站彻底清除（recycle.rs）发生，本模块不动磁盘。
//! 原两段式（needConfirm / lastAssets）与 origin 转移随旧模型废除（2026-08-19）。

use std::collections::HashSet;

use crate::core::backup;
use crate::core::library::{self, Library};
use crate::core::models::{Placement, RecycleItem};
use crate::services::import::ROOT_FOLDER_ID;

use super::{find_placement, finish, now, rewrite_orders, PlacementRef};

/// 大批量删除阈值：达到先 backup_library（失败仅警告不阻断，参照 library.rs 打开自检策略）
pub const DELETE_BACKUP_THRESHOLD: usize = 50;

pub fn delete_placements(lib: &mut Library, items: &[PlacementRef], last_action: Option<&str>) -> Result<(), String> {
  let action = match last_action {
    Some(a @ ("recycle" | "uncategorized")) => a,
    other => return Err(format!("未知的删除方式：{}（可选 recycle / uncategorized）", other.unwrap_or("（空）"))),
  };
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
    return Ok(());
  }
  // 大批量写前备份（防灾；备份失败仅警告不阻断）
  if matched.len() >= DELETE_BACKUP_THRESHOLD {
    if let Err(e) = backup::backup_library(&lib.root, &lib.assets_dir) {
      log::warn!("删除前备份失败（不阻断删除）：{e}");
    }
  }

  match action {
    "uncategorized" => {
      // 挪未分类：folder 改 root，名称 / origin 标记不动；root 段 order 重写连续
      for &i in &matched {
        lib.folders.items[i].folder = ROOT_FOLDER_ID.to_string();
      }
      rewrite_orders(lib, ROOT_FOLDER_ID);
    }
    "recycle" => {
      // 进回收站：按资产聚合条目（已有同资产条目追加 placements）；matched 按 vec 正序逐个转移
      let matched_set: HashSet<usize> = matched.iter().copied().collect();
      let stamp = now();
      let mut moved: Vec<Placement> = Vec::new();
      let mut i = 0;
      lib.folders.items.retain(|p| {
        let hit = matched_set.contains(&i);
        i += 1;
        if hit {
          moved.push(p.clone());
        }
        !hit
      });
      for p in moved {
        match lib.recycle.items.iter_mut().find(|r| r.asset == p.asset) {
          Some(entry) => entry.placements.push(p),
          None => lib.recycle.items.push(RecycleItem {
            asset: p.asset.clone(),
            placements: vec![p],
            deleted_at: stamp.clone(),
          }),
        }
      }
      library::save_recycle(lib).map_err(|e| e.to_string())?;
    }
    _ => unreachable!(),
  }
  finish(lib)?;
  log::info!("删除 {} 个项目（{action}）", matched.len());
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::json_io::read_json;
  use crate::core::library::{FOLDERS_FILE, RECYCLE_FILE};
  use crate::core::models::{Folder, FoldersFile, RecycleFile};
  use crate::services::import as ops;
  use crate::services::import::{PendingFile, Source};
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

  fn add_folder(lib: &mut Library, id: &str) {
    let folder = Folder { id: id.into(), name: id.into(), parent: None, block: None, icon: None, kind: None, color: None, groups: None, order: 0 };
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

  /// 从文件夹中移除 = 挪未分类：origin / 链接同路径（folder 改 root，origin 标记与名称不动）
  #[test]
  fn delete_uncategorized_moves_to_root_asis() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01");
    add_asset(&mut h, "AN-A", "a.png", "F01");
    add_asset(&mut h, "AN-B", "b.png", "F01");
    // AN-B 的链接也挂 F01（移除它 = 挪 root；AN-B origin 留 F01 不动）
    ops::add_placement(&mut h.lib, "AN-B".into(), "F01".into(), "b链接.png".into(), false, &mut HashMap::new());
    finish(&mut h.lib).unwrap();

    delete_placements(
      &mut h.lib,
      &[pref("AN-A", "F01", true), pref("AN-B", "F01", false)],
      Some("uncategorized"),
    )
    .unwrap();
    let a = h.lib.folders.items.iter().find(|p| p.asset == "AN-A").unwrap();
    assert_eq!((a.folder.as_str(), a.origin, a.name.as_str()), (ROOT_FOLDER_ID, true, "a.png"), "origin 挪 root 原样");
    let b: Vec<_> = h.lib.folders.items.iter().filter(|p| p.asset == "AN-B").collect();
    assert_eq!(b.len(), 2, "链接挪 root 是移动不是删除——origin 留 F01 + 链接在 root");
    let link = b.iter().find(|p| !p.origin).unwrap();
    assert_eq!((link.folder.as_str(), link.name.as_str()), (ROOT_FOLDER_ID, "b链接.png"), "链接记录原样（名称不动）");
    assert_eq!(b.iter().find(|p| p.origin).unwrap().folder, "F01", "origin 不动");
    assert!(h.lib.recycle.items.is_empty(), "不进回收站");
    let orders: Vec<i32> = h.lib.folders.items.iter().filter(|p| p.folder == "root").map(|p| p.order).collect();
    assert_eq!(orders, vec![0, 1], "root 段 order 重写连续");
    // 落盘一致
    let disk: FoldersFile = read_json(&h.lib.root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(disk.items.iter().filter(|p| p.folder == "root").count(), 2);
  }

  /// 丢到回收站 = 位置级：origin 单独进回收站，链接留库内活着（不转移）；快照按资产聚合
  #[test]
  fn delete_recycle_position_level_no_transfer() {
    let mut h = make_lib();
    add_folder(&mut h.lib, "F01");
    add_folder(&mut h.lib, "F02");
    add_asset(&mut h, "AN-A", "a.png", "F01"); // origin
    ops::add_placement(&mut h.lib, "AN-A".into(), "F02".into(), "链接.png".into(), false, &mut HashMap::new());
    finish(&mut h.lib).unwrap();

    // origin 单独丢回收站：链接活着、不转移、资产不进回收站视图以外的任何处理
    delete_placements(&mut h.lib, &[pref("AN-A", "F01", true)], Some("recycle")).unwrap();
    let remaining: Vec<_> = h.lib.folders.items.iter().filter(|p| p.asset == "AN-A").collect();
    assert_eq!(remaining.len(), 1, "链接留在库内");
    assert!(!remaining[0].origin, "origin 不主动转移（用户自主设置，归阶段六）");
    assert_eq!(h.lib.recycle.items.len(), 1);
    assert_eq!(h.lib.recycle.items[0].placements.len(), 1);
    assert!(h.lib.recycle.items[0].placements[0].origin, "origin 标记随记录进快照");
    let disk: RecycleFile = read_json(&h.lib.root.join(RECYCLE_FILE)).unwrap();
    assert_eq!(disk.items.len(), 1, "recycle.json 落盘");

    // 链接再丢回收站：同资产条目聚合追加（不新建条目）
    delete_placements(&mut h.lib, &[pref("AN-A", "F02", false)], Some("recycle")).unwrap();
    assert_eq!(h.lib.recycle.items.len(), 1, "同资产聚合");
    assert_eq!(h.lib.recycle.items[0].placements.len(), 2);
    assert_eq!(count(&h.lib, "AN-A"), 0, "库内无剩余位置");
    // 未知 / 空删除方式拒绝
    assert!(delete_placements(&mut h.lib, &[], Some("销毁")).is_err());
    assert!(delete_placements(&mut h.lib, &[], None).is_err());
  }
}
