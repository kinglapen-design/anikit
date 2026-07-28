//! 命令 API：撤销 / 重做（6.0 地基）。会话级双栈存 AppState（库切换清空，不持久化）；
//! undo / redo = 磁盘逆操作 + 数据文件快照原子写回 → 重载内存索引（盘上是事实来源），
//! 前端随后全量刷新（复用既有 refresh）。空栈给中文人话（规范第 6 条）。

use tauri::State;

use crate::commands::library::AppState;
use crate::core::library;
use crate::core::undo::UndoState;

/// 撤销一步，返回操作标签（通知条文案用）；空栈报"没有可撤销的操作"。
/// async：快照写回 + 全量重载涉及磁盘 IO，一律走线程池（规范第 2 条）
#[tauri::command]
pub async fn undo(state: State<'_, AppState>) -> Result<String, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  // 锁顺序约定：library → undo（redo 同序，防死锁）
  let label = {
    let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
    undo.undo().map_err(|e| e.to_string())?.ok_or_else(|| "没有可撤销的操作".to_string())?
  };
  library::reload(lib).map_err(|e| e.to_string())?;
  log::info!("已撤销：{label}");
  Ok(label)
}

/// 重做一步，返回操作标签；空栈报"没有可重做的操作"。async 同上
#[tauri::command]
pub async fn redo(state: State<'_, AppState>) -> Result<String, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let label = {
    let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
    undo.redo().map_err(|e| e.to_string())?.ok_or_else(|| "没有可重做的操作".to_string())?
  };
  library::reload(lib).map_err(|e| e.to_string())?;
  log::info!("已重做：{label}");
  Ok(label)
}

/// 栈状态查询（菜单置灰用；未打开库 = 双空）
#[tauri::command]
pub async fn get_undo_state(state: State<'_, AppState>) -> Result<UndoState, String> {
  if state.library.lock().map_err(|e| e.to_string())?.is_none() {
    return Ok(UndoState { can_undo: false, can_redo: false, undo_label: None, redo_label: None });
  }
  let undo = state.undo.lock().map_err(|e| e.to_string())?;
  Ok(undo.state())
}
