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

pub use block::{apply_block_changes, reorder_blocks, ApplyBlockReport, BlockInput};
pub use clone::clone_folders;
pub use delete::delete_placements;
pub use document::create_document;
pub use duplicate::{duplicate_placements, rename_placement, rename_placements, BatchRenameFailed};
pub use folder::{create_folder, move_folders, rename_folder, reorder_folders, set_folder_color, set_folder_icon};
pub use placement::{link_placements, move_placements, reorder_placements};

use serde::{Deserialize, Serialize};

use crate::core::library::{self, Library};
use crate::services::import::ROOT_FOLDER_ID;

/// Placement 定位三元组（Placement 无自身 ID，models.rs；前端选择项原样回传）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacementRef {
  pub asset_id: String,
  pub folder_id: String,
  #[serde(default)]
  pub origin: bool,
}

/// 批量重命名单项（3.4，37 稿）：定位三元组 + 新显示名（词干规则由前端组装，含扩展名）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameItem {
  pub asset_id: String,
  pub folder_id: String,
  #[serde(default)]
  pub origin: bool,
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

/// 按三元组定位 Placement 在 folders.items 中的下标
pub(crate) fn find_placement(lib: &Library, item: &PlacementRef) -> Option<usize> {
  lib
    .folders
    .items
    .iter()
    .position(|p| p.asset == item.asset_id && p.folder == item.folder_id && p.origin == item.origin)
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
