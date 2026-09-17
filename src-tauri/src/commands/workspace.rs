//! 命令 API：索引式工作空间（PRD 02 步骤 1 只读底座）——根信息 / 目录列举 / 缩略图请求。
//! 未开空间一律返回空（null / []）不报错（参照 list_space_libraries 先例）；
//! 业务在 services::workspace，本层只做取空间快照与参数传递。

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::commands::library::AppState;
use crate::adapters::shell_open;
use crate::core::space::WORKSPACE_DIR;
use crate::services::workspace::{self, ThumbReady, WsEntry};

/// 工作空间根信息
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceRootInfo {
  /// workspace/ 目录绝对路径（前端 convertFileSrc 直读其下文件）
  pub root_path: String,
  /// 目录是否存在（空间创建初期 / 目录被删时 false，前端据此显空态）
  pub exists: bool,
}

/// 当前空间的工作空间根；未开空间返回 null
#[tauri::command]
pub fn workspace_root(state: State<AppState>) -> Option<WorkspaceRootInfo> {
  let space = state.space.lock().ok()?.clone()?;
  let root = space.root.join(WORKSPACE_DIR);
  Some(WorkspaceRootInfo { root_path: root.to_string_lossy().into_owned(), exists: root.is_dir() })
}

/// 列举工作空间目录：relPath 空串 = 根；目录在前、同名不区分大小写排序；thumbPath 仅缓存新鲜时给。
/// 未开空间 / workspace 目录缺失 / 越界或读取失败 → Ok(空列表)（降级不报错，参照 list_space_libraries）
#[tauri::command]
pub async fn list_workspace_dir(state: State<'_, AppState>, rel_path: String) -> Result<Vec<WsEntry>, String> {
  let Some(space) = state.space.lock().map_err(|e| e.to_string())?.clone() else { return Ok(Vec::new()) };
  let root = space.root.join(WORKSPACE_DIR);
  if !root.is_dir() {
    return Ok(Vec::new());
  }
  Ok(workspace::list_dir(&root, &rel_path).unwrap_or_else(|e| {
    log::warn!("工作空间目录列举失败（rel={rel_path}）：{e}");
    Vec::new()
  }))
}

/// 在文件管理器中打开并选中（工作空间条目右键；按真实路径，约束在当前空间 workspace 内）
#[tauri::command]
pub fn reveal_path(state: State<AppState>, abs_path: String) -> Result<(), String> {
  let space = state.space.lock().map_err(|e| e.to_string())?.clone().ok_or_else(|| "请先打开储存空间".to_string())?;
  let root = space.root.join(WORKSPACE_DIR);
  let target = std::path::PathBuf::from(&abs_path);
  if !workspace::is_under(&root, &target) {
    return Err("路径不在当前工作空间内".to_string());
  }
  if !target.exists() {
    return Err("文件已不存在（可能刚被移动或删除）".to_string());
  }
  shell_open::reveal(&target)
}

/// 请求缩略图：立即返回，后台线程逐一生成缺失 / 过期的缓存（解码失败条目跳过），
/// 完成 emit workspace-thumbs-ready（负载 = 本趟实际生成成功的 Vec<ThumbReady>，可能为空数组）。
/// 线程内不可用 State（spawn 前克隆 AppHandle）；未开空间 / 目录缺失不动
#[tauri::command]
pub fn request_workspace_thumbs(app: AppHandle, state: State<AppState>, abs_paths: Vec<String>) -> Result<(), String> {
  let Some(space) = state.space.lock().map_err(|e| e.to_string())?.clone() else { return Ok(()) };
  let root = space.root.join(WORKSPACE_DIR);
  if !root.is_dir() {
    return Ok(());
  }
  let paths: Vec<std::path::PathBuf> = abs_paths.into_iter().map(std::path::PathBuf::from).collect();
  std::thread::spawn(move || {
    let ready: Vec<ThumbReady> = workspace::request_thumbs(&root, paths);
    if let Err(e) = app.emit(workspace::THUMBS_EVENT, ready) {
      log::debug!("工作空间缩略图事件推送失败（前端可能已关闭）：{e}");
    }
  });
  Ok(())
}
