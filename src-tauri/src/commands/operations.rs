//! 命令 API：素材基础操作（6.1 选择态 + 基础操作，PRD 02-10）。
//! 命令层只做锁与撤销三步接入（锁序 library → undo，与 commands::undo 同约）：
//! begin(标签) → 写点前 record_file → 业务（services::operations）→ 成功 commit / 失败 cancel；
//! 磁盘副作用（副本 info 目录、originName 改名）成对 record_disk_op。
//! 业务失败一律 cancel + 内存从盘重载（盘上是事实来源，防半截内存态）。
//! 全部 async：涉及磁盘 IO 一律走线程池（规范第 2 条）。

use std::path::PathBuf;

use tauri::State;

use crate::commands::library::AppState;
use crate::core::library::{self, Library, FOLDERS_FILE, RECYCLE_FILE};
use crate::core::models::Folder;
use crate::core::paths;
use crate::core::undo::{DiskOp, UndoManager};
use crate::services::import::ROOT_FOLDER_ID;
use crate::services::operations::{self, ApplyBlockReport, BlockInput, DeleteReport, PlacementRef};

/// metadata.json 路径（由 ID 推导，PRD 01-2.3）
fn meta_path(lib: &Library, asset_id: &str) -> PathBuf {
  paths::metadata_file(&lib.assets_dir, asset_id)
}

/// 业务失败收尾：丢弃撤销事务 + 内存从盘重载（防半截内存态），返回错误文案
/// pub(crate)：commands::recycle 还原命令同约复用
pub(crate) fn fail(lib: &mut Library, undo: &mut UndoManager, e: String) -> String {
  undo.cancel();
  if let Err(re) = library::reload(lib) {
    log::error!("操作失败后内存重载失败：{re}");
  }
  e
}

/// 新建文件夹：parentId = null / "root" 为顶层；afterFolderId 给了就插到该兄弟之后，否则末尾；
/// blockId 仅顶层新建生效（写入 Folder.block 并继承区块默认图标，2026-08-14 区块子系统）
#[tauri::command]
pub async fn create_folder(
  state: State<'_, AppState>,
  parent_id: Option<String>,
  name: String,
  after_folder_id: Option<String>,
  block_id: Option<String>,
) -> Result<Folder, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("新建文件夹");
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  match operations::create_folder(lib, parent_id.as_deref(), &name, after_folder_id.as_deref(), block_id.as_deref()) {
    Ok(folder) => {
      undo.commit().map_err(|e| e.to_string())?;
      Ok(folder)
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 重命名文件夹（2026-07-29 复验三轮）：仅改数据不动磁盘；名称同库名 Windows 规则
/// （非法 / 空名后端拒绝，业务失败 cancel + 内存重载）
#[tauri::command]
pub async fn rename_folder(
  state: State<'_, AppState>,
  folder_id: String,
  new_name: String,
) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin(format!("重命名文件夹为 {}", new_name.trim()));
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  match operations::rename_folder(lib, &folder_id, &new_name) {
    Ok(()) => {
      undo.commit().map_err(|e| e.to_string())?;
      Ok(())
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 移动 Placement 到目标文件夹，返回实际移动条数（去重 / 原地 / 陈旧项跳过不计）；
/// 移出 root 时清 metadata.importedFrom（摘标：移动即转正）
#[tauri::command]
pub async fn move_placements(
  state: State<'_, AppState>,
  items: Vec<PlacementRef>,
  target_folder: String,
) -> Result<usize, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin(format!("移动 {} 个项目", items.len()));
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  // 摘标候选（root → 非 root）的 metadata 写点前快照
  if target_folder != ROOT_FOLDER_ID {
    for it in &items {
      if it.folder_id == ROOT_FOLDER_ID {
        undo.record_file(&meta_path(lib, &it.asset_id)).map_err(|e| e.to_string())?;
      }
    }
  }
  match operations::move_placements(lib, &items, &target_folder) {
    Ok(moved) => {
      undo.commit().map_err(|e| e.to_string())?;
      Ok(moved)
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 段内排序：items（须同属一个文件夹）移到 targetFolder 段内 beforeItem 之前（null = 段尾）；
/// 跨文件夹移出 root 时清 metadata.importedFrom（摘标口径与 move_placements 一致）
#[tauri::command]
pub async fn reorder_placements(
  state: State<'_, AppState>,
  items: Vec<PlacementRef>,
  target_folder: String,
  before_item: Option<PlacementRef>,
) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("调整顺序");
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  // 摘标候选（root → 非 root）的 metadata 写点前快照（与 move_placements 同口径）
  if target_folder != ROOT_FOLDER_ID {
    for it in &items {
      if it.folder_id == ROOT_FOLDER_ID {
        undo.record_file(&meta_path(lib, &it.asset_id)).map_err(|e| e.to_string())?;
      }
    }
  }
  match operations::reorder_placements(lib, &items, &target_folder, before_item.as_ref()) {
    Ok(()) => {
      undo.commit().map_err(|e| e.to_string())?;
      Ok(())
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 文件夹同级排序（混排护栏：Folder 与 Placement 各自独立排序，前端分开调）
#[tauri::command]
pub async fn reorder_folders(
  state: State<'_, AppState>,
  folder_ids: Vec<String>,
  parent_id: Option<String>,
  before_folder_id: Option<String>,
) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("调整文件夹顺序");
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  match operations::reorder_folders(lib, &folder_ids, parent_id.as_deref(), before_folder_id.as_deref()) {
    Ok(()) => {
      undo.commit().map_err(|e| e.to_string())?;
      Ok(())
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 文件夹重设父级（仅改数据不动磁盘）：targetParent = null / "root" 为顶层；
/// beforeFolderId 给了 = 插到该兄弟（targetParent 直接子级）之前（跨父级上/下 1/3 落点定位，
/// 2026-07-29 复验：嵌套文件夹拖出），null = 同级末尾；
/// targetBlock（2026-08-14 区块子系统）：null = 保持原区块；落根级时给值 = 写入 Folder.block
/// （移回「目录」传 "dir"）；落进文件夹内时必须为 null；
/// 循环（移入自身或子孙）整条报错，同父 / 陈旧 ID 跳过，返回实际移动数
#[tauri::command]
pub async fn move_folders(
  state: State<'_, AppState>,
  folder_ids: Vec<String>,
  target_parent: Option<String>,
  before_folder_id: Option<String>,
  target_block: Option<String>,
) -> Result<usize, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin(format!("移动 {} 个文件夹", folder_ids.len()));
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  match operations::move_folders(lib, &folder_ids, target_parent.as_deref(), before_folder_id.as_deref(), target_block.as_deref()) {
    Ok(moved) => {
      undo.commit().map_err(|e| e.to_string())?;
      Ok(moved)
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 关联：目标文件夹逐条建链接 Placement，返回关联条数；
/// names[i] 非空 ? 自定义显示名 : 源 Placement 名
#[tauri::command]
pub async fn link_placements(
  state: State<'_, AppState>,
  items: Vec<PlacementRef>,
  target_folder: String,
  names: Vec<Option<String>>,
) -> Result<usize, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin(format!("关联 {} 个项目", items.len()));
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  match operations::link_placements(lib, &items, &target_folder, &names) {
    Ok(linked) => {
      undo.commit().map_err(|e| e.to_string())?;
      Ok(linked)
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 创建副本：info 目录整体复制 + 新资源实体，返回新 assetId 清单；
/// 磁盘副作用成对登记（undo = 删除新 info 目录 / redo = 复制还原）
#[tauri::command]
pub async fn duplicate_placements(
  state: State<'_, AppState>,
  items: Vec<PlacementRef>,
  target_folder: String,
  names: Vec<Option<String>>,
) -> Result<Vec<String>, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin(format!("创建 {} 个副本", items.len()));
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  match operations::duplicate_placements(lib, &items, &target_folder, &names) {
    Ok(out) => {
      for (old_info, new_info) in &out.copies {
        undo.record_disk_op(
          DiskOp::Delete(new_info.clone()),
          DiskOp::Copy { from: old_info.clone(), to: new_info.clone() },
        );
      }
      undo.commit().map_err(|e| e.to_string())?;
      Ok(out.new_ids)
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 新建文本文档入库（1.3，07 新增菜单"MarkDown 文档 / 文本文档"）：kind = "md" | "txt"；
/// 撤销 = info 目录移入 cache/tmp 备份（DiskOp::Rename），重做 = 移回（Delete 会丢内容无法重做）
#[tauri::command]
pub async fn create_text_document(
  state: State<'_, AppState>,
  kind: String,
  target_folder: Option<String>,
) -> Result<crate::services::views::PlacementView, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("新建文档".to_string());
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  match operations::create_text_document(lib, &kind, target_folder.as_deref()) {
    Ok((view, info_dir, backup)) => {
      undo.record_disk_op(
        DiskOp::Rename { from: info_dir.clone(), to: backup.clone() },
        DiskOp::Rename { from: backup, to: info_dir },
      );
      undo.commit().map_err(|e| e.to_string())?;
      Ok(view)
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 重命名 Placement：显示名即时改；原始 Placement 追加磁盘段（originName + 词干缩略图
/// 同步改名，成对登记 DiskOp::Rename——注意 record_disk_op(undo_op, redo_op) 参数顺序）；
/// 链接 Placement 仅改显示名
#[tauri::command]
pub async fn rename_placement(
  state: State<'_, AppState>,
  asset_id: String,
  folder_id: String,
  origin: bool,
  new_name: String,
) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin(format!("重命名为 {}", new_name.trim()));
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  if origin {
    undo.record_file(&meta_path(lib, &asset_id)).map_err(|e| e.to_string())?;
  }
  match operations::rename_placement(lib, &asset_id, &folder_id, origin, &new_name) {
    Ok(renames) => {
      for (old_path, new_path) in &renames {
        undo.record_disk_op(
          DiskOp::Rename { from: new_path.clone(), to: old_path.clone() },
          DiskOp::Rename { from: old_path.clone(), to: new_path.clone() },
        );
      }
      undo.commit().map_err(|e| e.to_string())?;
      Ok(())
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 删除 Placement（两段式）：lastAction = null 且涉及资源最后一条 Placement 时
/// 返回 needConfirm + lastAssets 不动数据；"recycle" = 移入回收站（recycle.json 记录，
/// info 原地不动）；"uncategorized" = 保留为未分类（改挂 root）；非最后一条直接移除
#[tauri::command]
pub async fn delete_placements(
  state: State<'_, AppState>,
  items: Vec<PlacementRef>,
  last_action: Option<String>,
) -> Result<DeleteReport, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin(format!("删除 {} 个项目", items.len()));
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  if last_action.as_deref() == Some("recycle") {
    undo.record_file(&lib.root.join(RECYCLE_FILE)).map_err(|e| e.to_string())?;
  }
  match operations::delete_placements(lib, &items, last_action.as_deref()) {
    // 询问段未动数据：丢弃事务，不进撤销栈
    Ok(report) if report.need_confirm => {
      undo.cancel();
      Ok(report)
    }
    Ok(report) => {
      undo.commit().map_err(|e| e.to_string())?;
      Ok(report)
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 区块管理草稿一次提交（2026-08-14 区块子系统，30 稿确认/取消）：blocks = 保留列表
/// （数组序 = 显示顺序，id = null 为新建），deleted = 待删区块 id；
/// deleted 含非空区块且 confirmed = false 时返回 needConfirm 不动数据（询问段不进撤销栈）
#[tauri::command]
pub async fn apply_block_changes(
  state: State<'_, AppState>,
  blocks: Vec<BlockInput>,
  deleted: Vec<String>,
  confirmed: bool,
) -> Result<ApplyBlockReport, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("区块管理");
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  match operations::apply_block_changes(lib, blocks, deleted, confirmed) {
    // 询问段未动数据：丢弃事务，不进撤销栈
    Ok(report) if report.need_confirm => {
      undo.cancel();
      Ok(report)
    }
    Ok(report) => {
      undo.commit().map_err(|e| e.to_string())?;
      Ok(report)
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 区块拖拽排序即时提交（2026-08-14 区块子系统）：ids 须恰好覆盖现有全部区块
#[tauri::command]
pub async fn reorder_blocks(state: State<'_, AppState>, ids: Vec<String>) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin("排序区块");
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  match operations::reorder_blocks(lib, ids) {
    Ok(()) => {
      undo.commit().map_err(|e| e.to_string())?;
      Ok(())
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}
