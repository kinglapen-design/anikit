//! 命令 API：储存空间与资源库管理（创建 / 打开 / 当前状态 / 空间内库列表 / 最近列表）。
//! 层级：储存空间（space.json + assets/ 池）→ 资源库（纯数据目录），PRD 01_储存空间。

use std::path::Path;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, State};

use crate::core::config::{self, LibraryEntry};
use crate::core::library::{self, Library};
use crate::core::space::{self, Space};

/// 应用状态：当前储存空间 + 当前打开的素材库（同一时刻只开一个库）+ 导入任务注册表 + 收件箱实例 + 撤销栈
pub struct AppState {
  pub space: Mutex<Option<Space>>,
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
pub struct SpaceInfo {
  pub name: String,
  pub path: String,
}

fn space_info(space: &Space) -> SpaceInfo {
  SpaceInfo { name: space.name.clone(), path: space.root.to_string_lossy().into_owned() }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryInfo {
  pub name: String,
  pub path: String,
  pub asset_count: usize,
  pub folder_count: usize,
  /// 自动导入开关（库级 settings.json，2.1）：关 = 不启动收件箱（不挂载盘符、不监听）
  pub auto_import: bool,
}

fn to_info(lib: &Library) -> LibraryInfo {
  LibraryInfo {
    name: lib.root.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
    path: lib.root.to_string_lossy().into_owned(),
    asset_count: lib.index.assets.len(),
    folder_count: lib.index.folders.len(),
    auto_import: lib.settings.auto_import,
  }
}

/// 空间进入打开状态：更新应用状态 + 记录到全局配置最近列表
fn activate_space(state: &State<AppState>, space: Space) -> Result<SpaceInfo, String> {
  let info = space_info(&space);
  *state.space.lock().map_err(|e| e.to_string())? = Some(space);
  config::record_space_open(&info.name, &info.path).map_err(|e| e.to_string())?;
  Ok(info)
}

/// 库进入打开状态：更新应用状态 + 记录到全局配置最近列表 + 按库级设置启停收件箱（3.2 / 2.1：
/// auto_import 关 = 不挂载盘符、不监听；切到关闭的库时停掉旧库收件箱）；
/// 库切换清空动图解包队列 pending（任务绑定 assets 池，切库后无意义，2026-07-26）
/// 与撤销双栈（6.0：会话级不持久化，栈内快照绑定旧库路径）
fn activate(app: &AppHandle, state: &State<AppState>, lib: Library) -> Result<LibraryInfo, String> {
  crate::services::unpack_queue::clear_pending();
  state.undo.lock().map_err(|e| e.to_string())?.clear();
  let info = to_info(&lib);
  let root = lib.root.clone();
  let auto_import = lib.settings.auto_import;
  *state.library.lock().map_err(|e| e.to_string())? = Some(lib);
  config::record_library_open(&info.name, &info.path).map_err(|e| e.to_string())?;
  if auto_import {
    crate::services::inbox::start(app, &root);
  } else {
    crate::services::inbox::stop(app);
  }
  Ok(info)
}

/// 取当前空间（短锁克隆）；未打开空间给中文错
fn current_space(state: &State<AppState>) -> Result<Space, String> {
  state.space.lock().map_err(|e| e.to_string())?.clone().ok_or_else(|| "请先打开储存空间".to_string())
}

/// 在选定位置创建储存空间（前端系统对话框选定父目录后调用；文件夹名即空间名）。
/// 同名位置已是合法空间则视为打开（PRD 01_储存空间-3.1）
#[tauri::command]
pub async fn create_space(state: State<'_, AppState>, parent_path: String, name: String) -> Result<SpaceInfo, String> {
  let space = space::create(Path::new(&parent_path), &name).map_err(|e| e.to_string())?;
  activate_space(&state, space)
}

/// 打开储存空间（打开前校验 space.json，不是 AniKit 储存空间的文件夹会被拒绝）
#[tauri::command]
pub async fn open_space(state: State<'_, AppState>, path: String) -> Result<SpaceInfo, String> {
  let space = space::open(Path::new(&path)).map_err(|e| e.to_string())?;
  activate_space(&state, space)
}

/// 当前打开的储存空间；未打开返回 null（前端据此决定是否进入初次打开向导）
#[tauri::command]
pub fn get_current_space(state: State<AppState>) -> Option<SpaceInfo> {
  state.space.lock().ok()?.as_ref().map(space_info)
}

/// 当前空间内的资源库名列表（只扫空间根一级，含 folders.json 的目录即库）；无当前空间返回空列表
#[tauri::command]
pub async fn list_space_libraries(state: State<'_, AppState>) -> Result<Vec<String>, String> {
  let space = state.space.lock().map_err(|e| e.to_string())?.clone();
  match space {
    Some(space) => space::list_libraries(&space.root).map_err(|e| e.to_string()),
    None => Ok(Vec::new()),
  }
}

/// 在当前储存空间内创建资源库（文件夹名即资源库名）；auto_import 写库级 settings.json
#[tauri::command]
pub async fn create_library(
  app: AppHandle,
  state: State<'_, AppState>,
  name: String,
  auto_import: bool,
) -> Result<LibraryInfo, String> {
  let space = current_space(&state)?;
  let lib = library::create(&space, &name, auto_import).map_err(|e| e.to_string())?;
  activate(&app, &state, lib)
}

/// 在当前储存空间内按名打开资源库（打开前校验结构；旧版库结构明确报错引导新建）
#[tauri::command]
pub async fn open_library(app: AppHandle, state: State<'_, AppState>, name: String) -> Result<LibraryInfo, String> {
  let space = current_space(&state)?;
  let lib = library::open(&space.root, &space.root.join(&name)).map_err(|e| e.to_string())?;
  activate(&app, &state, lib)
}

/// 当前打开的素材库；未打开返回 null
#[tauri::command]
pub fn get_current_library(state: State<AppState>) -> Option<LibraryInfo> {
  state.library.lock().ok()?.as_ref().map(to_info)
}

/// 开关自动导入（2.1：向导第二步初值 / 快速访问"自动导入"确认弹窗开启）：
/// 库级 settings.json 单写点；开 = 启动收件箱（盘符挂载 + 监听），关 = 停止收件箱（卸载盘符）
#[tauri::command]
pub async fn set_auto_import(app: AppHandle, state: State<'_, AppState>, enabled: bool) -> Result<LibraryInfo, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开资源库".to_string())?;
  lib.settings.auto_import = enabled;
  library::save_settings(lib).map_err(|e| e.to_string())?;
  let info = to_info(lib);
  let root = lib.root.clone();
  drop(guard); // 收件箱启停走 AppState 锁，先放库锁防交叉
  if enabled {
    crate::services::inbox::start(&app, &root);
  } else {
    crate::services::inbox::stop(&app);
  }
  Ok(info)
}

/// 全局配置中的资源库最近列表
#[tauri::command]
pub fn list_recent_libraries() -> Result<Vec<LibraryEntry>, String> {
  config::load().map(|c| c.libraries).map_err(|e| e.to_string())
}
