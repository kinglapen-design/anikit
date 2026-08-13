//! 命令 API：重复文件扫描（只读，不走撤销）。

use tauri::State;

use crate::commands::library::AppState;
use crate::services::scan::{self, DupGroupView, ScanScope};

/// 扫描重复文件：按存量 hash 分组（锚点过滤，组内任一资产命中范围则整组返回）
#[tauri::command]
pub async fn scan_duplicates(state: State<'_, AppState>, scope: ScanScope) -> Result<Vec<DupGroupView>, String> {
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  Ok(scan::scan_duplicates(lib, &scope, &lib.format_icons))
}
