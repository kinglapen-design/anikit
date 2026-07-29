//! 命令 API：右键菜单配套（6.3）——打开方式 / 资源管理器定位 / 缩略图三件套 /
//! 导出 / 彻底删除（素材 / 文件夹）/ base64。
//! 仅 delete_folder（permanent=false，"删除文件夹"素材移未分类）接撤销三步（锁序 library → undo）；
//! 彻底删除两命令不接撤销（物理不可逆，删前 backup_library 兜底）；其余为只读或库外写。

use tauri::State;

use crate::adapters::shell_open;
use crate::commands::library::AppState;
use crate::commands::operations::fail;
use crate::commands::preview::locate_asset;
use crate::core::library::FOLDERS_FILE;
use crate::services::{asset_menu, purge};
use crate::services::purge::DeleteFolderReport;
use crate::services::recycle::DeleteRecycleReport;

/// 其他应用打开（系统"打开方式"对话框）
#[tauri::command]
pub async fn open_with_dialog(state: State<'_, AppState>, asset_id: String) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let (_info_dir, origin, _meta) = locate_asset(lib, &asset_id)?;
  shell_open::open_as(&origin)
}

/// 在文件管理器中打开并选中（前端先弹警示窗再调）
#[tauri::command]
pub async fn reveal_in_explorer(state: State<'_, AppState>, asset_id: String) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let (_info_dir, origin, _meta) = locate_asset(lib, &asset_id)?;
  shell_open::reveal(&origin)
}

/// 剪贴板图片应用为缩略图（2026-07-23 待办落地）
#[tauri::command]
pub async fn set_thumbnail_from_clipboard(state: State<'_, AppState>, asset_id: String) -> Result<asset_menu::ThumbResult, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  asset_menu::set_thumbnail_from_clipboard(lib, &asset_id)
}

/// 自定义缩略图（系统文件选择器选定后调用）
#[tauri::command]
pub async fn set_thumbnail_custom(
  state: State<'_, AppState>,
  asset_id: String,
  source_path: String,
) -> Result<asset_menu::ThumbResult, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  asset_menu::set_thumbnail_custom(lib, &asset_id, &source_path)
}

/// 强制刷新缩略图（从 origin 重跑导入同管线，防缩略图损坏 / 自检失败）
#[tauri::command]
pub async fn refresh_thumbnail(state: State<'_, AppState>, asset_id: String) -> Result<asset_menu::ThumbResult, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  asset_menu::refresh_thumbnail(lib, &asset_id)
}

/// 导出文件：origin 复制到目标目录（重名 -序号），返回导出数；不接撤销（库外写）
#[tauri::command]
pub async fn export_assets(state: State<'_, AppState>, asset_ids: Vec<String>, target_dir: String) -> Result<usize, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  asset_menu::export_assets(lib, &asset_ids, &target_dir)
}

/// 导出文件夹：层级保留，素材按 Placement 显示名复制；不接撤销（库外写）
#[tauri::command]
pub async fn export_folder(state: State<'_, AppState>, folder_id: String, target_dir: String) -> Result<usize, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  asset_menu::export_folder(lib, &folder_id, &target_dir)
}

/// 素材直接彻底删除（两段式；不经过回收站，不接撤销）
#[tauri::command]
pub async fn delete_assets_permanently(
  state: State<'_, AppState>,
  asset_ids: Vec<String>,
  confirmed: bool,
) -> Result<DeleteRecycleReport, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  purge::delete_assets_permanently(lib, &asset_ids, confirmed)
}

/// 删除文件夹两形态：permanent=false = "删除文件夹"（子树素材移未分类 + 节点删除，接撤销）；
/// permanent=true = "彻底删除文件夹"（两段式，子树资产物理全删，不接撤销）
#[tauri::command]
pub async fn delete_folder(
  state: State<'_, AppState>,
  folder_id: String,
  permanent: bool,
  confirmed: bool,
) -> Result<DeleteFolderReport, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  if permanent {
    return purge::delete_folder(lib, &folder_id, true, confirmed);
  }
  let name = lib.index.folders.get(&folder_id).map(|f| f.name.clone()).unwrap_or_else(|| folder_id.clone());
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin(format!("删除文件夹 {name}"));
  undo.record_file(&lib.root.join(FOLDERS_FILE)).map_err(|e| e.to_string())?;
  match purge::delete_folder(lib, &folder_id, false, confirmed) {
    Ok(report) => {
      undo.commit().map_err(|e| e.to_string())?;
      Ok(report)
    }
    Err(e) => Err(fail(lib, &mut undo, e)),
  }
}

/// 复制 base64（仅静态图片且 ≤10MB，纯 base64 不带 data: 前缀）
#[tauri::command]
pub async fn get_asset_base64(state: State<'_, AppState>, asset_id: String) -> Result<String, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  asset_menu::get_asset_base64(lib, &asset_id)
}

/// 资产源文件真实绝对路径（6.3 右键"复制文件路径"；与预览路径解析无关，恒为 info 内 origin）
#[tauri::command]
pub async fn get_asset_origin_path(state: State<'_, AppState>, asset_id: String) -> Result<String, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let (_info_dir, origin, _meta) = locate_asset(lib, &asset_id)?;
  Ok(origin.to_string_lossy().into_owned())
}
