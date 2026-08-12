//! 最近使用（recent.json，6.1 第二轮）：upsert 到最前 + LRU 500 截断 + 落盘。
//! 判定收窄（2026-07-29 用户定）：只挂 open_with_default_app 一个入口（打开成功才记）；
//! 重命名 / 移动 / 预览一律不记；哈希变更 / 发送动画软件 / 跨窗口拖出三个入口留机制
//! （touch_recent pub(crate)），待 8.5 动画软件通信 / 文件同步落地时接入。
//! 浏览行为不是文档修改——不接撤销（Eagle 同）。

use std::collections::HashSet;

use serde::Serialize;

use crate::core::library::{self, Library};
use crate::core::models::{RecentItem, RECENT_LIMIT};
use crate::services::operations::now;
use crate::services::views::{placement_view, PlacementView};

/// 命令 list_recent_assets 输出：PlacementView 同构 + viewedAt
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentAssetView {
  #[serde(flatten)]
  pub view: PlacementView,
  pub viewed_at: String,
}

/// 记录一次使用：同 ID 提前到最前（upsert），超出 RECENT_LIMIT 淘汰最旧，随后落盘。
/// pub(crate)：哈希变更 / 发送动画软件 / 跨窗口拖出三入口的预留挂钩点（待 8.5 / 文件同步落地）
pub(crate) fn touch_recent(lib: &mut Library, asset_id: &str) -> Result<(), String> {
  lib.recent.items.retain(|i| i.asset != asset_id);
  lib.recent.items.insert(0, RecentItem { asset: asset_id.to_string(), viewed_at: now() });
  lib.recent.items.truncate(RECENT_LIMIT);
  library::save_recent(lib).map_err(|e| e.to_string())
}

/// 最近使用列表：viewedAt 倒序；资产已无任何 Placement（在回收站）跳过；
/// 视图取该资产 origin Placement（无 origin 取第一条）
pub fn list_recent_assets(lib: &Library) -> Vec<RecentAssetView> {
  let mut items = lib.recent.items.clone();
  // 稳定排序：同秒并列保持存储序（touch 插入序本就是最新在前）
  items.sort_by(|a, b| b.viewed_at.cmp(&a.viewed_at));
  items.into_iter().filter_map(|i| recent_view(lib, i)).collect()
}

fn recent_view(lib: &Library, item: RecentItem) -> Option<RecentAssetView> {
  let meta = lib.index.assets.get(&item.asset)?;
  let placements = lib.index.placements_of(&item.asset);
  if placements.is_empty() {
    return None;
  }
  let p = placements.iter().find(|p| p.origin).unwrap_or(&placements[0]);
  Some(RecentAssetView { view: placement_view(lib, meta, p, &lib.format_icons), viewed_at: item.viewed_at })
}

/// 剔除指定资源的最近使用记录，有剔除才落盘（delete_recycle 彻底删除时同步清理）
pub(crate) fn remove_recent(lib: &mut Library, asset_ids: &HashSet<&str>) -> Result<(), String> {
  let before = lib.recent.items.len();
  lib.recent.items.retain(|i| !asset_ids.contains(i.asset.as_str()));
  if lib.recent.items.len() != before {
    library::save_recent(lib).map_err(|e| e.to_string())?;
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::json_io::read_json;
  use crate::core::library::RECENT_FILE;
  use crate::core::models::RecentFile;
  use crate::services::import::{self as ops, PendingFile, Source};
  use crate::services::operations::finish;
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

  /// 造一个资源实体并在指定文件夹登记一条 Placement
  fn add_asset(h: &mut Harness, id: &str, file: &str, folder: &str, origin: bool) {
    let src = h.dir.path().join(file);
    std::fs::write(&src, b"fake").unwrap();
    let pf = PendingFile { path: src, chain: Vec::new() };
    let meta = ops::build_asset_with_id(&h.lib.assets_dir, &pf, "sha256:t", id, Source::Copy, None).unwrap();
    h.lib.index.assets.insert(id.into(), meta);
    ops::add_placement(&mut h.lib, id.into(), folder.into(), file.into(), origin, &mut HashMap::new());
    finish(&mut h.lib).unwrap();
  }

  #[test]
  fn touch_recent_upserts_to_front_and_persists() {
    let mut h = make_lib();
    add_asset(&mut h, "AN-A", "a.png", "root", true);
    add_asset(&mut h, "AN-B", "b.png", "root", true);

    touch_recent(&mut h.lib, "AN-A").unwrap();
    touch_recent(&mut h.lib, "AN-B").unwrap();
    touch_recent(&mut h.lib, "AN-A").unwrap(); // 再次使用提前到最前，不重复

    let ids: Vec<&str> = h.lib.recent.items.iter().map(|i| i.asset.as_str()).collect();
    assert_eq!(ids, ["AN-A", "AN-B"], "upsert 到最前");
    assert_eq!(h.lib.recent.items.len(), 2, "同 ID 不重复");
    let disk: RecentFile = read_json(&h.lib.root.join(RECENT_FILE)).unwrap();
    assert_eq!(disk.items.len(), 2, "recent.json 落盘");
    assert_eq!(disk.items[0].asset, "AN-A");
    assert!(!disk.items[0].viewed_at.is_empty(), "viewedAt 有值");
  }

  #[test]
  fn touch_recent_truncates_to_limit() {
    let mut h = make_lib();
    for n in 0..RECENT_LIMIT + 5 {
      touch_recent(&mut h.lib, &format!("AN-{n:04}")).unwrap();
    }
    assert_eq!(h.lib.recent.items.len(), RECENT_LIMIT, "LRU 截断到 500");
    assert_eq!(h.lib.recent.items[0].asset, format!("AN-{:04}", RECENT_LIMIT + 4), "最新在最前");
    assert_eq!(h.lib.recent.items.last().unwrap().asset, "AN-0005", "最旧的 5 条被淘汰");
    let disk: RecentFile = read_json(&h.lib.root.join(RECENT_FILE)).unwrap();
    assert_eq!(disk.items.len(), RECENT_LIMIT, "落盘同步截断");
  }

  #[test]
  fn list_recent_assets_desc_and_skips_recycled() {
    let mut h = make_lib();
    add_asset(&mut h, "AN-A", "a.png", "root", true);
    add_asset(&mut h, "AN-B", "b.png", "root", true);
    touch_recent(&mut h.lib, "AN-A").unwrap();
    touch_recent(&mut h.lib, "AN-B").unwrap();

    let list = list_recent_assets(&h.lib);
    assert_eq!(list.iter().map(|v| v.view.asset_id.as_str()).collect::<Vec<_>>(), ["AN-B", "AN-A"], "倒序");
    assert!(!list[0].viewed_at.is_empty());
    assert_eq!(list[0].view.folder_id, "root", "取 origin Placement");
    assert!(list[0].view.origin);

    // AN-A 进回收站（全部 Placement 移除）→ 最近使用列表跳过
    h.lib.folders.items.retain(|p| p.asset != "AN-A");
    h.lib.index.rebuild_placements(&h.lib.folders.items);
    let list = list_recent_assets(&h.lib);
    assert_eq!(list.len(), 1, "回收站资产跳过");
    assert_eq!(list[0].view.asset_id, "AN-B");

    // 索引里也没有的 ID（脏数据）同样跳过，不 panic
    h.lib.recent.items.insert(0, RecentItem { asset: "AN-GONE".into(), viewed_at: now() });
    assert_eq!(list_recent_assets(&h.lib).len(), 1);
  }

  #[test]
  fn list_recent_assets_falls_back_to_first_placement_without_origin() {
    let mut h = make_lib();
    add_asset(&mut h, "AN-A", "a.png", "root", true);
    // 无 origin 只有链接 Placement：取第一条
    h.lib.folders.items.iter_mut().for_each(|p| p.origin = false);
    h.lib.index.rebuild_placements(&h.lib.folders.items);
    touch_recent(&mut h.lib, "AN-A").unwrap();
    let list = list_recent_assets(&h.lib);
    assert_eq!(list.len(), 1);
    assert!(!list[0].view.origin);
  }

  #[test]
  fn remove_recent_only_writes_when_changed() {
    let mut h = make_lib();
    touch_recent(&mut h.lib, "AN-A").unwrap();
    let ids: HashSet<&str> = ["AN-A"].into_iter().collect();
    remove_recent(&mut h.lib, &ids).unwrap();
    assert!(h.lib.recent.items.is_empty());
    let disk: RecentFile = read_json(&h.lib.root.join(RECENT_FILE)).unwrap();
    assert!(disk.items.is_empty(), "有剔除则落盘");
    // 无引用：不动（不报错即可）
    remove_recent(&mut h.lib, &ids).unwrap();
  }
}
