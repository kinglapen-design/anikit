//! 素材基础操作服务（6.1 选择态 + 基础操作）：新建文件夹、移动 / 排序 / 关联 / 副本 /
//! 重命名 / 删除 Placement。命令层（commands::operations）持锁并做撤销三步接入
//! （锁序 library → undo），本模块只拿 &mut Library 做业务；
//! 改动 Placement 的统一经 finish() 收尾：重建内存索引 + folders.json 一次落盘。

mod block;
mod clone;
mod delete;
mod document;
mod duplicate;
mod folder;
mod placement;
mod sequence;

pub use block::{apply_block_changes, reorder_blocks, ApplyBlockReport, BlockInput};
pub use clone::clone_folders;
pub use delete::delete_placements;
pub use document::create_document;
pub use duplicate::{duplicate_placements, rename_placement, rename_placements, BatchRenameFailed};
pub use folder::{assign_group, create_folder, move_folders, rename_folder, reorder_folders, set_folder_color, set_folder_colors, set_folder_groups, set_folder_icon};
pub use placement::{link_placements, move_placements, reorder_placements, set_placement_origin, sync_asset_links};

/// 按三元组 + 可选显示名定位 Placement（name 给了 = 精确到行：同文件夹同 origin 的重复链接
/// 可区分；缺省 = 三元组第一条，sync_asset_links 删除 / 改名定位用）
pub(crate) fn find_placement_named(lib: &Library, item: &PlacementRef, name: Option<&str>) -> Option<usize> {
  lib.folders.items.iter().position(|p| {
    p.asset == item.asset_id && p.folder == item.folder_id && p.origin == item.origin && name.map_or(true, |n| p.name == n)
  })
}
pub use sequence::{delete_frames, dissolve_sequence, renumber_frames};

use serde::{Deserialize, Serialize};

use crate::core::library::{self, Library};
use crate::services::import::ROOT_FOLDER_ID;

/// Placement 定位三元组（Placement 无自身 ID，models.rs；前端选择项原样回传）。
/// name（2026-10-11 选择域独立化配套）：同文件夹同资产多条位置（origin + 链接 / 重复链接，
/// 2026-08-13 放开）时按显示名精确到行——find_placement 见名即用 find_placement_named 语义；
/// None = 三元组首条（原语义，MCP 等旧调用方不变）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacementRef {
  pub asset_id: String,
  pub folder_id: String,
  #[serde(default)]
  pub origin: bool,
  #[serde(default)]
  pub name: Option<String>,
}

/// 批量重命名单项（3.4，37 稿）：定位三元组 + 新显示名（词干规则由前端组装，含扩展名）。
/// origin = None 时自动消歧（2026-09-20，MCP 调用方常省略踩坑后加）：(asset, folder)
/// 恰好一条记录即命中；多条（origin + 链接同文件夹）进 failed 要求指定。APP 前端恒传 Some。
/// old_name（2026-10-11 选择域独立化配套）：同 origin 重复链接按旧显示名精确到行
///（resolve_origin 消歧也吃它——origin None + oldName 唯一命中即取该行 origin）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameItem {
  pub asset_id: String,
  pub folder_id: String,
  #[serde(default)]
  pub origin: Option<bool>,
  pub new_name: String,
  #[serde(default)]
  pub old_name: Option<String>,
}

/// 链接管理（17 稿，6.2 三轮）新增链接一条：目标文件夹 + 显示名（空 = 用资产名）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncLinkAdd {
  pub folder_id: String,
  pub name: String,
}

/// 链接管理删除一条：定位（folderId + origin；同文件夹同 origin 重复链接时 name 精确到行——
/// Placement 无自身 ID，三元组只能命中第一条，2026-08-13 重复链接放开后弹窗平铺多行需要）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncLinkRemove {
  pub folder_id: String,
  #[serde(default)]
  pub origin: bool,
  pub name: Option<String>,
}

/// 链接管理改名一条：定位（folderId + origin，重复链接时 oldName 精确到行）+ 新显示名
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncLinkRename {
  pub folder_id: String,
  #[serde(default)]
  pub origin: bool,
  pub old_name: Option<String>,
  pub new_name: String,
}

/// RFC3339 本地时间（精确到秒），与 metadata / recycle 时间字段格式一致
pub(crate) fn now() -> String {
  chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, false)
}

/// 文件夹存在性校验（root 保留 ID 放行）
pub(crate) fn check_folder(lib: &Library, folder_id: &str) -> Result<(), String> {
  if folder_id != ROOT_FOLDER_ID && !lib.index.folders.contains_key(folder_id) {
    return Err("目标文件夹不存在，可能已被删除".to_string());
  }
  Ok(())
}

/// 父级 ID 归一：None / "root" → 顶层（Folder.parent = None）
pub(crate) fn parent_key(parent: Option<&str>) -> Option<String> {
  match parent {
    None | Some(ROOT_FOLDER_ID) => None,
    Some(id) => Some(id.to_string()),
  }
}

/// 按三元组定位 Placement 在 folders.items 中的下标；item.name 给了 = 精确到行
///（同文件夹重复链接 / origin+链接同夹可区分，2026-10-11 选择域独立化配套）
pub(crate) fn find_placement(lib: &Library, item: &PlacementRef) -> Option<usize> {
  find_placement_named(lib, item, item.name.as_deref())
}

/// RenameItem.origin 缺省消歧（2026-09-20）：Some 直用；None = 按 (asset, folder) 找——
/// oldName 给了先按 (asset, folder, name) 精确试（2026-10-11：同 origin 重复链接可消歧）；
/// 恰好一条记录取其实际 origin；零条 / 多条 = None（零条走原「项目不存在」路径、多条进
/// failed 要求指定，由调用方分流）。命令层 undo 快照与 service 匹配共用本 helper（单写点）
pub(crate) fn resolve_origin(lib: &Library, it: &RenameItem) -> Option<bool> {
  match it.origin {
    Some(o) => Some(o),
    None => {
      if let Some(n) = &it.old_name {
        let mut hits =
          lib.folders.items.iter().filter(|p| p.asset == it.asset_id && p.folder == it.folder_id && p.name == *n);
        if let (Some(p), None) = (hits.next(), hits.next()) {
          return Some(p.origin);
        }
      }
      let mut hits = lib.folders.items.iter().filter(|p| p.asset == it.asset_id && p.folder == it.folder_id);
      match (hits.next(), hits.next()) {
        (Some(p), None) => Some(p.origin),
        _ => None,
      }
    }
  }
}

/// 文件夹段 order 重写为连续 0..n（按 items vec 顺序；reorder 落盘前调用）
pub(crate) fn rewrite_orders(lib: &mut Library, folder_id: &str) {
  let mut n = 0;
  for p in &mut lib.folders.items {
    if p.folder == folder_id {
      p.order = n;
      n += 1;
    }
  }
}

/// 改动 Placement 后的标准收尾：重建内存索引 + folders.json 落盘（resolve.rs 同样板）
pub(crate) fn finish(lib: &mut Library) -> Result<(), String> {
  lib.index.rebuild_placements(&lib.folders.items);
  library::save_folders(lib).map_err(|e| e.to_string())
}

/// Folder 节点增删后的索引同步（folders.folders 为事实来源，全量重建映射）——
/// 原 operations/folder.rs 私有辅助，delete_folder（purge.rs）同样需要，提为 pub(crate)
pub(crate) fn sync_folder_index(lib: &mut Library) {
  lib.index.folders = lib.folders.folders.iter().map(|f| (f.id.clone(), f.clone())).collect();
}
