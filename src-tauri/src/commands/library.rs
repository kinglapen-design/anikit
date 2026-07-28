//! 命令 API：库管理（创建 / 打开 / 当前状态 / 最近列表）。

use std::path::Path;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, State};

use crate::core::config::{self, LibraryEntry};
use crate::core::library::{self, Library};

/// 应用状态：当前打开的素材库（同一时刻只开一个库）+ 导入任务注册表 + 收件箱实例 + 撤销栈
pub struct AppState {
  pub library: Mutex<Option<Library>>,
  /// 导入任务取消标记：task_id → 置位即取消（任务结束从表中移除）
  pub imports: Mutex<std::collections::HashMap<u64, std::sync::Arc<std::sync::atomic::AtomicBool>>>,
  /// 收件箱运行实例（3.2）：随库打开 / 切换 / 退出由 services::inbox 重建
  pub inbox: Mutex<Option<crate::services::inbox::InboxHandle>>,
  /// 撤销 / 重做双栈（6.0）：会话级不持久化，库切换清空
  pub undo: Mutex<crate::core::undo::UndoManager>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryInfo {
  pub name: String,
  pub path: String,
  pub asset_count: usize,
  pub folder_count: usize,
}

fn to_info(lib: &Library) -> LibraryInfo {
  LibraryInfo {
    name: lib.root.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
    path: lib.root.to_string_lossy().into_owned(),
    asset_count: lib.index.assets.len(),
    folder_count: lib.index.folders.len(),
  }
}

/// 库进入打开状态：更新应用状态 + 记录到全局配置最近列表 + 启动收件箱（3.2）；
/// 库切换清空动图解包队列 pending（任务绑定库 root，切库后无意义，2026-07-26）
/// 与撤销双栈（6.0：会话级不持久化，栈内快照绑定旧库路径）
fn activate(app: &AppHandle, state: &State<AppState>, lib: Library) -> Result<LibraryInfo, String> {
  crate::services::unpack_queue::clear_pending();
  state.undo.lock().map_err(|e| e.to_string())?.clear();
  let info = to_info(&lib);
  let root = lib.root.clone();
  *state.library.lock().map_err(|e| e.to_string())? = Some(lib);
  config::record_library_open(&info.name, &info.path).map_err(|e| e.to_string())?;
  crate::services::inbox::start(app, &root);
  Ok(info)
}

/// 在选定文件夹创建素材库（前端系统对话框选定后调用；文件夹名即素材库名）
#[tauri::command]
pub fn create_library(app: AppHandle, state: State<'_, AppState>, path: String) -> Result<LibraryInfo, String> {
  let lib = library::create(Path::new(&path)).map_err(|e| e.to_string())?;
  activate(&app, &state, lib)
}

/// 打开素材库（打开前校验结构，不是 AniKit 资源库的文件夹会被拒绝）
#[tauri::command]
pub fn open_library(app: AppHandle, state: State<'_, AppState>, path: String) -> Result<LibraryInfo, String> {
  let lib = library::open(Path::new(&path)).map_err(|e| e.to_string())?;
  activate(&app, &state, lib)
}

/// 当前打开的素材库；未打开返回 null（前端据此决定是否弹"创建/打开资源库"窗口）
#[tauri::command]
pub fn get_current_library(state: State<AppState>) -> Option<LibraryInfo> {
  state.library.lock().ok()?.as_ref().map(to_info)
}

/// 全局配置中的资源库最近列表
#[tauri::command]
pub fn list_recent_libraries() -> Result<Vec<LibraryEntry>, String> {
  config::load().map(|c| c.libraries).map_err(|e| e.to_string())
}
