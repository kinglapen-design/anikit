//! 命令 API：回收站（6.1 第二轮）——视图 / 还原 / 彻底删除 / 清空。
//! 还原接撤销三步（锁序 library → undo，与 commands::operations 同约；
//! record_file 覆盖 folders.json + recycle.json 两个写点）。
//! 彻底删除 / 清空**不接撤销**——物理删除不可逆，操作前 backup_library 兜底（服务层执行）。

use tauri::State;

use crate::commands::library::AppState;
use crate::commands::operations::fail;
use crate::core::library::{FOLDERS_FILE, RECYCLE_FILE};
use crate::services::recycle::{self, DeleteRecycleReport, RecycleEntryView};

/// 回收站列表：deletedAt 倒序（folderId 恒 "recycle" 伪值，origin 恒 false）
#[tauri::command]
pub async fn list_recycle(state: State<'_, AppState>) -> Result<Vec<RecycleEntryView>, String> {
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  Ok(recycle::list_recycle(lib))
}

/// 还原：Placement 写回 folders.json（目标文件夹缺失挂 root、同名唯一化）+ 回收站移除条目，
/// 返回还原的资源数
#[tauri::command]
pub async fn restore_recycle(state: State<'_, AppState>, asset_ids: Vec<String>) -> Result<usize, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin(format!("还原 {} 个项目", asset_ids.len()));
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  undo.record_file(&lib.root.join(RECYCLE_FILE)).map_err(|e| e.to_string())?;
  match recycle::restore_recycle(lib, &asset_ids) {
    Ok(restored) => {
      undo.commit().map_err(|e| e.to_string())?;
      Ok(restored)
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 彻底删除（两段式）：库内其他虚拟片段 source 指向被删集合时，未确认只返回
/// needConfirm + segmentRefs 不动数据；确认后物理删 info 目录并同步清理全部引用。
/// 不接撤销（物理不可逆，操作前 backup_library 兜底）
#[tauri::command]
pub async fn delete_recycle(
  state: State<'_, AppState>,
  asset_ids: Vec<String>,
  confirmed: bool,
) -> Result<DeleteRecycleReport, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  recycle::delete_recycle(lib, &asset_ids, confirmed)
}

/// 清空回收站：等价于对全部条目执行 delete_recycle（segmentRefs 对全集汇总）
#[tauri::command]
pub async fn clear_recycle(state: State<'_, AppState>, confirmed: bool) -> Result<DeleteRecycleReport, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  recycle::clear_recycle(lib, confirmed)
}
