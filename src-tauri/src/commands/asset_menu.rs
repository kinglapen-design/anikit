//! 命令 API：右键菜单配套（6.3）——打开方式 / 资源管理器定位 / 缩略图三件套 /
//! 导出 / 彻底删除（素材 / 文件夹）/ base64。
//! 仅 delete_folder（permanent=false，"删除文件夹"素材移未分类）接撤销三步（锁序 library → undo）；
//! 彻底删除两命令不接撤销（物理不可逆，删前 backup_library 兜底）；其余为只读或库外写。

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

use crate::adapters::shell_open;
use crate::commands::import::NEXT_TASK_ID;
use crate::commands::library::AppState;
use crate::commands::operations::fail;
use crate::core::library::FOLDERS_FILE;
use crate::services::{asset_menu, purge};
use crate::services::purge::DeleteFolderReport;
use crate::services::recycle::DeleteRecycleReport;

/// 其他应用打开（系统"打开方式"对话框）；序列帧 = frames/ 封面帧（real_origin_file 特判）
#[tauri::command]
pub async fn open_with_dialog(state: State<'_, AppState>, asset_id: String) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let origin = asset_menu::real_origin_file(lib, &asset_id)?;
  shell_open::open_as(&origin)
}

/// 在文件管理器中打开并选中（前端先弹警示窗再调）；序列帧 = frames/ 封面帧
#[tauri::command]
pub async fn reveal_in_explorer(state: State<'_, AppState>, asset_id: String) -> Result<(), String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let origin = asset_menu::real_origin_file(lib, &asset_id)?;
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

/// 导出数据源快照：锁内克隆（资产索引 + folders 文件），工作线程只持快照——
/// 导出 IO 不占用库锁，长拷贝不阻塞 UI 其他命令
fn export_snapshot(app: &AppHandle) -> Result<asset_menu::ExportSource, String> {
  let state = app.state::<AppState>();
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  Ok(asset_menu::ExportSource {
    assets_dir: lib.assets_dir.clone(),
    assets: lib.index.assets.clone(),
    folders: lib.folders.clone(),
  })
}

/// 导出任务 spawn：注册取消标记（与导入任务共用注册表与 id 命名空间）→ 工作线程执行 →
/// 推 Finished（含失败清单；业务失败进 error 字段）→ 注册表移除
fn spawn_export<F>(app: &AppHandle, on_event: Channel<asset_menu::ExportEvent>, job: F) -> u64
where
  F: FnOnce(&AtomicBool, &mut dyn FnMut(asset_menu::ExportEvent)) -> Result<asset_menu::ExportOutcome, String> + Send + 'static,
{
  let task_id = NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed);
  let cancel = Arc::new(AtomicBool::new(false));
  match app.state::<AppState>().imports.lock() {
    Ok(mut registry) => {
      registry.insert(task_id, cancel.clone());
    }
    Err(_) => log::error!("导出任务 {task_id} 注册失败（锁冲突），该任务将无法取消"),
  }
  let app_handle = app.clone();
  std::thread::spawn(move || {
    let mut report = |e: asset_menu::ExportEvent| {
      if let Err(err) = on_event.send(e) {
        log::debug!("导出事件推送失败（前端可能已关闭）：{err}");
      }
    };
    let (copied, failed, error) = match job(&cancel, &mut report) {
      Ok(o) => (o.copied as u32, o.failed, None),
      Err(e) => (0, Vec::new(), Some(e)),
    };
    report(asset_menu::ExportEvent::Finished { copied, failed, error });
    if let Ok(mut registry) = app_handle.state::<AppState>().imports.lock() {
      registry.remove(&task_id);
    }
  });
  task_id
}

/// 导出文件：任务化——立即返回 task_id，进度经 Channel（Started / Item / Finished）；
/// 序列帧 = 文件夹装全部帧；库外写不接撤销；cancel_export 中断（当前文件拷完后停）
#[tauri::command]
pub fn export_assets(
  app: AppHandle,
  asset_ids: Vec<String>,
  target_dir: String,
  on_event: Channel<asset_menu::ExportEvent>,
) -> Result<u64, String> {
  let src = export_snapshot(&app)?;
  Ok(spawn_export(&app, on_event, move |cancel, report| {
    asset_menu::export_assets(&src, &asset_ids, &target_dir, report, cancel)
  }))
}

/// 导出文件夹：任务化同 export_assets；层级保留，素材按 Placement 显示名
#[tauri::command]
pub fn export_folder(
  app: AppHandle,
  folder_id: String,
  target_dir: String,
  on_event: Channel<asset_menu::ExportEvent>,
) -> Result<u64, String> {
  let src = export_snapshot(&app)?;
  Ok(spawn_export(&app, on_event, move |cancel, report| {
    asset_menu::export_folder(&src, &folder_id, &target_dir, report, cancel)
  }))
}

/// 导出失败重试：items = 首跑 Finished.failed 原样回传（源→目标原路覆盖，不读库无需快照）
#[tauri::command]
pub fn export_retry(
  app: AppHandle,
  items: Vec<asset_menu::ExportFailedItem>,
  on_event: Channel<asset_menu::ExportEvent>,
) -> Result<u64, String> {
  Ok(spawn_export(&app, on_event, move |cancel, report| Ok(asset_menu::retry_export(&items, report, cancel))))
}

/// 取消导出任务：置位取消标记（与导入共用注册表），工作线程当前文件拷完后中断
#[tauri::command]
pub fn cancel_export(state: State<'_, AppState>, task_id: u64) -> Result<(), String> {
  let registry = state.imports.lock().map_err(|e| e.to_string())?;
  match registry.get(&task_id) {
    Some(flag) => {
      flag.store(true, Ordering::Relaxed);
      Ok(())
    }
    None => Err("导出任务不存在或已结束".to_string()),
  }
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

/// 资产源文件真实绝对路径（6.3 右键"复制文件路径"；与预览路径解析无关，恒为 info 内实体；
/// 序列帧 = frames/ 封面帧——info 根路径不存在，2026-08-14 修复）
#[tauri::command]
pub async fn get_asset_origin_path(state: State<'_, AppState>, asset_id: String) -> Result<String, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let origin = asset_menu::real_origin_file(lib, &asset_id)?;
  Ok(origin.to_string_lossy().into_owned())
}
