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
  /// 导入 / 导出任务取消标记：task_id → 置位即取消（任务结束从表中移除）
  pub imports: Mutex<std::collections::HashMap<u64, std::sync::Arc<std::sync::atomic::AtomicBool>>>,
  /// 收件箱运行实例（3.2）：空间级共用，随空间打开 / 自动导入开关 / 退出由 services::inbox 启停
  pub inbox: Mutex<Option<crate::services::inbox::InboxHandle>>,
  /// 撤销 / 重做双栈（6.0）：会话级不持久化，库切换清空
  pub undo: Mutex<crate::core::undo::UndoManager>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpaceInfo {
  pub name: String,
  pub path: String,
  /// 自动导入开关（空间级设置 space.json，2026-08-12 用户定）：关 = 不启动收件箱（不挂载盘符、不监听）
  pub auto_import: bool,
}

fn space_info(space: &Space) -> SpaceInfo {
  SpaceInfo { name: space.name.clone(), path: space.root.to_string_lossy().into_owned(), auto_import: space.auto_import }
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

/// 空间进入打开状态：更新应用状态 + 记录到全局配置最近列表 + 按空间级 autoImport 启停收件箱
/// （2026-08-12 用户定：inbox 空间级共用，盘符挂载恒定指向空间 inbox/、不随切库变化；
/// 关 = 防御性停止，换掉开着收件箱的旧空间时卸干净）
fn activate_space(app: &AppHandle, state: &State<AppState>, space: Space) -> Result<SpaceInfo, String> {
  let info = space_info(&space);
  let root = space.root.clone();
  let auto_import = space.auto_import;
  *state.space.lock().map_err(|e| e.to_string())? = Some(space);
  config::record_space_open(&info.name, &info.path).map_err(|e| e.to_string())?;
  if auto_import {
    crate::services::inbox::start(app, &root);
  } else {
    crate::services::inbox::stop(app);
  }
  Ok(info)
}

/// 库进入打开状态：更新应用状态 + 记录到全局配置最近列表（切库不动收件箱——空间级共用）；
/// 库切换清空动图解包队列 pending（任务绑定 assets 池，切库后无意义，2026-07-26）
/// 与撤销双栈（6.0：会话级不持久化，栈内快照绑定旧库路径）
fn activate(state: &State<AppState>, lib: Library) -> Result<LibraryInfo, String> {
  crate::services::unpack_queue::clear_pending();
  state.undo.lock().map_err(|e| e.to_string())?.clear();
  let info = to_info(&lib);
  *state.library.lock().map_err(|e| e.to_string())? = Some(lib);
  config::record_library_open(&info.name, &info.path).map_err(|e| e.to_string())?;
  Ok(info)
}

/// 取当前空间（短锁克隆）；未打开空间给中文错
fn current_space(state: &State<AppState>) -> Result<Space, String> {
  state.space.lock().map_err(|e| e.to_string())?.clone().ok_or_else(|| "请先打开储存空间".to_string())
}

/// 在选定位置创建储存空间（前端系统对话框选定父目录后调用；文件夹名即空间名）。
/// 同名位置已是合法空间则视为打开（PRD 01_储存空间-3.1）。auto_import 写 space.json（空间级设置）
#[tauri::command]
pub async fn create_space(
  app: AppHandle,
  state: State<'_, AppState>,
  parent_path: String,
  name: String,
  auto_import: bool,
) -> Result<SpaceInfo, String> {
  let space = space::create(Path::new(&parent_path), &name, auto_import).map_err(|e| e.to_string())?;
  activate_space(&app, &state, space)
}

/// 打开储存空间（打开前校验 space.json，不是 AniKit 储存空间的文件夹会被拒绝）
#[tauri::command]
pub async fn open_space(app: AppHandle, state: State<'_, AppState>, path: String) -> Result<SpaceInfo, String> {
  let space = space::open(Path::new(&path)).map_err(|e| e.to_string())?;
  activate_space(&app, &state, space)
}

/// 当前打开的储存空间；未打开返回 null（前端据此决定是否进入初次打开向导）
#[tauri::command]
pub fn get_current_space(state: State<AppState>) -> Option<SpaceInfo> {
  state.space.lock().ok()?.as_ref().map(space_info)
}

/// 当前空间内的资源库名列表，按 space.json libraryOrder 排序返回（3.2 拖动排序持久化；
/// 不在 order 内的外来目录排尾、按名字典序）；无当前空间返回空列表
#[tauri::command]
pub async fn list_space_libraries(state: State<'_, AppState>) -> Result<Vec<String>, String> {
  let space = state.space.lock().map_err(|e| e.to_string())?.clone();
  match space {
    Some(space) => space::ordered_libraries(&space).map_err(|e| e.to_string()),
    None => Ok(Vec::new()),
  }
}

/// 在当前储存空间内创建资源库（文件夹名即资源库名）；新库名自动排尾进 libraryOrder（3.2）
#[tauri::command]
pub async fn create_library(state: State<'_, AppState>, name: String) -> Result<LibraryInfo, String> {
  let mut guard = state.space.lock().map_err(|e| e.to_string())?;
  let space = guard.as_mut().ok_or_else(|| "请先打开储存空间".to_string())?;
  let lib = library::create(space, &name).map_err(|e| e.to_string())?;
  drop(guard); // activate 走 AppState 其他锁，先放空间锁（与 set_auto_import 同风格）
  activate(&state, lib)
}

/// 在当前储存空间内按名打开资源库（打开前校验结构；旧版库结构明确报错引导新建）
#[tauri::command]
pub async fn open_library(state: State<'_, AppState>, name: String) -> Result<LibraryInfo, String> {
  let space = current_space(&state)?;
  let lib = library::open(&space.root, &space.root.join(&name)).map_err(|e| e.to_string())?;
  activate(&state, lib)
}

/// 当前打开的素材库；未打开返回 null
#[tauri::command]
pub fn get_current_library(state: State<AppState>) -> Option<LibraryInfo> {
  state.library.lock().ok()?.as_ref().map(to_info)
}

/// 开关自动导入（2.1：向导第二步初值 / 快速访问"自动导入"确认弹窗开启）：
/// 空间级设置（space.json 单写点，2026-08-12 用户定：inbox 空间级共用）；
/// 开 = 启动收件箱（盘符挂载 + 监听空间 inbox/），关 = 停止收件箱（卸载盘符）
#[tauri::command]
pub async fn set_auto_import(app: AppHandle, state: State<'_, AppState>, enabled: bool) -> Result<SpaceInfo, String> {
  let mut guard = state.space.lock().map_err(|e| e.to_string())?;
  let space = guard.as_mut().ok_or_else(|| "请先打开储存空间".to_string())?;
  space.auto_import = enabled;
  space::save_space(space).map_err(|e| e.to_string())?;
  let info = space_info(space);
  let root = space.root.clone();
  drop(guard); // 收件箱启停走 AppState 锁，先放空间锁防交叉
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

/// 持久化库列表自定义顺序（3.2 菜单抓手拖动排序）：order 必须与空间内实际库名集合一致，
/// 多 / 少都报中文错；落盘后返回按新顺序排好的最新库名列表
#[tauri::command]
pub async fn reorder_libraries(state: State<'_, AppState>, order: Vec<String>) -> Result<Vec<String>, String> {
  let mut guard = state.space.lock().map_err(|e| e.to_string())?;
  let space = guard.as_mut().ok_or_else(|| "请先打开储存空间".to_string())?;
  space::reorder_libraries(space, order).map_err(|e| e.to_string())
}

/// 统计库内孤儿素材数（2026-08-12 定案：删库确认弹窗提示"仅被该库引用的素材"数量用）。
/// 孤儿 = 仅被该库引用的资产（引用口径见 core::space::library_orphans）；库不存在报中文错
#[tauri::command]
pub async fn count_library_orphans(state: State<'_, AppState>, name: String) -> Result<u64, String> {
  apply_count_orphans(&state, &name)
}

/// 统计实现（命令层薄壳，便于单测）：编排在命令层，孤儿判定在 core::space
fn apply_count_orphans(state: &AppState, name: &str) -> Result<u64, String> {
  let space =
    state.space.lock().map_err(|e| e.to_string())?.clone().ok_or_else(|| "请先打开储存空间".to_string())?;
  let orphans = space::library_orphans(&space.root, name).map_err(|e| e.to_string())?;
  Ok(orphans.len() as u64)
}

/// 删除资源库（3.2 删除×；确认弹窗在前端）：返回删除后剩余库名列表（按 libraryOrder 序），
/// 前端据此决定切到剩余第一个库或回创建流程。delete_orphans = true 时一并删除仅被该库引用
/// 的孤儿素材实体（2026-08-12 定案：统一 assets 池默认保留，可选清理）
#[tauri::command]
pub async fn delete_library(
  _app: AppHandle,
  state: State<'_, AppState>,
  name: String,
  delete_orphans: bool,
) -> Result<Vec<String>, String> {
  apply_delete_library(&state, &name, delete_orphans)
}

/// 删除资源库的实现（命令层薄壳，便于单测）：删的是当前打开的库时先做关库收尾——
/// 清动图解包队列 pending、清撤销双栈、state.library 置 None（收件箱不动，空间级共用）；
/// 再整目录递归删除。库内数据文件删除不可恢复；素材实体默认保留在空间级 assets 池，
/// delete_orphans = true 时孤儿素材实体的 info 文件夹一并从 assets 池递归删除
fn apply_delete_library(state: &AppState, name: &str, delete_orphans: bool) -> Result<Vec<String>, String> {
  let is_current = state
    .library
    .lock()
    .map_err(|e| e.to_string())?
    .as_ref()
    .and_then(|l| l.root.file_name().map(|n| n.to_string_lossy().into_owned()))
    .as_deref()
    == Some(name);
  if is_current {
    crate::services::unpack_queue::clear_pending();
    state.undo.lock().map_err(|e| e.to_string())?.clear();
    *state.library.lock().map_err(|e| e.to_string())? = None;
  }
  let mut guard = state.space.lock().map_err(|e| e.to_string())?;
  let space = guard.as_mut().ok_or_else(|| "请先打开储存空间".to_string())?;
  space::delete_library(space, name, delete_orphans).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 建测试 AppState：空间 + 两个库（库A 为当前打开），返回 (临时目录, AppState)
  fn make_state() -> (tempfile::TempDir, AppState) {
    let dir = tempfile::tempdir().unwrap();
    let mut space = space::create(dir.path(), "空间", false).unwrap();
    let lib_a = library::create(&mut space, "库A").unwrap();
    library::create(&mut space, "库B").unwrap();
    let state = AppState {
      space: Mutex::new(Some(space)),
      library: Mutex::new(Some(lib_a)),
      imports: Mutex::new(std::collections::HashMap::new()),
      inbox: Mutex::new(None),
      undo: Mutex::new(crate::core::undo::UndoManager::default()),
    };
    (dir, state)
  }

  /// 删当前打开的库：先关库收尾（解包队列 / 撤销栈 / library 置 None），目录消失，返回剩余列表
  #[test]
  fn delete_current_library_closes_state() {
    let (_dir, state) = make_state();
    // 撤销栈里压一条，验证清栈
    {
      let mut undo = state.undo.lock().unwrap();
      undo.begin("测试事务");
      undo.commit().unwrap();
    }
    assert!(state.undo.lock().unwrap().state().can_undo, "删除前撤销栈非空");

    let remaining = apply_delete_library(&state, "库A", false).unwrap();
    assert_eq!(remaining, vec!["库B"]);
    assert!(state.library.lock().unwrap().is_none(), "删当前库后 state.library 应置 None");
    assert!(!state.undo.lock().unwrap().state().can_undo, "删当前库应清撤销栈");
    let space = state.space.lock().unwrap();
    let space = space.as_ref().unwrap();
    assert!(!space.root.join("库A").exists(), "库目录应整体消失");
    assert_eq!(space.library_order, vec!["库B"], "order 应移除被删库");
    assert!(space.root.join("库B").is_dir(), "其余库不动");
  }

  /// 删非当前库：state.library 不动
  #[test]
  fn delete_other_library_keeps_current() {
    let (_dir, state) = make_state();
    let remaining = apply_delete_library(&state, "库B", false).unwrap();
    assert_eq!(remaining, vec!["库A"]);
    assert!(state.library.lock().unwrap().is_some(), "删非当前库不应动 state.library");
  }

  /// 删不存在的库：中文报错，状态不动
  #[test]
  fn delete_missing_library_errors() {
    let (_dir, state) = make_state();
    assert!(apply_delete_library(&state, "不存在", false).is_err());
    assert!(state.library.lock().unwrap().is_some(), "删除失败不应动 state.library");
  }

  /// 造库引用关系辅助：写 folders.json items + recycle.json items，并在 assets 池建对应 info
  /// （含原始文件，验证删除是递归的）。返回 assets 池路径便于断言
  fn seed_library(
    state: &AppState,
    lib: &str,
    folder_items: &[&str],
    recycle_items: &[&str],
  ) -> std::path::PathBuf {
    use crate::core::models::*;
    use crate::core::paths;
    let guard = state.space.lock().unwrap();
    let space = guard.as_ref().unwrap();
    let lib_dir = space.root.join(lib);
    let folders = FoldersFile {
      version: DATA_VERSION,
      folders: vec![Folder { id: "F01".into(), name: "夹".into(), parent: None, order: 0 }],
      items: folder_items
        .iter()
        .map(|id| Placement { asset: id.to_string(), folder: "F01".into(), name: format!("{id}.fla"), order: 0, origin: true })
        .collect(),
    };
    crate::core::json_io::write_json(&lib_dir.join(crate::core::library::FOLDERS_FILE), &folders).unwrap();
    let recycle = RecycleFile {
      version: DATA_VERSION,
      items: recycle_items
        .iter()
        .map(|id| RecycleItem { asset: id.to_string(), placements: Vec::new(), deleted_at: "2026-08-12T00:00:00+08:00".into() })
        .collect(),
    };
    crate::core::json_io::write_json(&lib_dir.join(crate::core::library::RECYCLE_FILE), &recycle).unwrap();
    let assets = paths::assets_dir(&space.root);
    for id in folder_items.iter().chain(recycle_items) {
      let info = paths::info_dir(&assets, id);
      std::fs::create_dir_all(&info).unwrap();
      std::fs::write(info.join("原始.fla"), b"x").unwrap();
    }
    assets
  }

  /// 孤儿清理（2026-08-12 定案）：delete_orphans=true 删库后，共享资产 info 保留、
  /// 仅被该库引用的孤儿 info 从 assets 池递归删除
  #[test]
  fn delete_orphans_removes_unique_keeps_shared() {
    let (_dir, state) = make_state();
    let assets = seed_library(&state, "库A", &["SHARED", "ONLY-A"], &[]);
    seed_library(&state, "库B", &["SHARED"], &[]);

    let remaining = apply_delete_library(&state, "库A", true).unwrap();
    assert_eq!(remaining, vec!["库B"]);
    assert!(!crate::core::paths::info_dir(&assets, "ONLY-A").exists(), "孤儿 info 应递归删除");
    assert!(crate::core::paths::info_dir(&assets, "SHARED").is_dir(), "共享 info 应保留");
  }

  /// delete_orphans=false：保持现状，assets 池一概不动
  #[test]
  fn delete_without_orphans_keeps_all_info() {
    let (_dir, state) = make_state();
    let assets = seed_library(&state, "库A", &["SHARED", "ONLY-A"], &[]);
    seed_library(&state, "库B", &["SHARED"], &[]);

    apply_delete_library(&state, "库A", false).unwrap();
    assert!(crate::core::paths::info_dir(&assets, "ONLY-A").is_dir(), "false 时孤儿 info 也不动");
    assert!(crate::core::paths::info_dir(&assets, "SHARED").is_dir());
  }

  /// 回收站算引用：资产仅在被删库的 folders，但在另一库 recycle → 保留；
  /// 被删库自己 recycle 里独有的 → 孤儿，一并删除
  #[test]
  fn recycle_reference_counts_as_reference() {
    let (_dir, state) = make_state();
    let assets = seed_library(&state, "库A", &["IN-OTHER-RECYCLE"], &["OWN-RECYCLE-ONLY"]);
    seed_library(&state, "库B", &[], &["IN-OTHER-RECYCLE"]);

    assert_eq!(apply_count_orphans(&state, "库A").unwrap(), 1, "他库回收站引用的不算孤儿");
    apply_delete_library(&state, "库A", true).unwrap();
    assert!(crate::core::paths::info_dir(&assets, "IN-OTHER-RECYCLE").is_dir(), "另一库回收站引用应保留");
    assert!(!crate::core::paths::info_dir(&assets, "OWN-RECYCLE-ONLY").exists(), "本库回收站独有应删除");
  }

  /// 孤儿计数：差集口径（folders ∪ recycle − 其余库 folders ∪ recycle 并集）
  #[test]
  fn count_orphans_matches_difference_set() {
    let (_dir, state) = make_state();
    seed_library(&state, "库A", &["SHARED", "ONLY-1", "ONLY-2"], &["ONLY-3"]);
    seed_library(&state, "库B", &["SHARED"], &[]);

    assert_eq!(apply_count_orphans(&state, "库A").unwrap(), 3);
    assert_eq!(apply_count_orphans(&state, "库B").unwrap(), 0, "库B 引用全是共享，无孤儿");
    // 库不存在报中文错
    let err = apply_count_orphans(&state, "不存在").unwrap_err();
    assert!(err.contains("资源库"), "应报中文错：{err}");
  }
}
