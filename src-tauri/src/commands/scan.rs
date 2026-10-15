//! 命令 API：重复文件扫描 / 文件档案链接扫描（只读，不走撤销）。

use tauri::State;

use crate::commands::library::AppState;
use crate::services::scan::{self, DupGroupView, LinkGroupView, ScanScope};

/// 扫描重复文件：按存量 hash 分组（锚点过滤，组内任一资产命中范围则整组返回）
#[tauri::command]
pub async fn scan_duplicates(state: State<'_, AppState>, scope: ScanScope) -> Result<Vec<DupGroupView>, String> {
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  Ok(scan::scan_duplicates(lib, &scope, &lib.format_icons))
}

/// 扫描文件/档案链接（2026-10-16 立项，PRD 03_查找文件-04）：全库单范围纯内存聚合，
/// 有 ≥1 条链接 Placement 的资产成组（首列原始文件卡 + 全部链接位置）
#[tauri::command]
pub async fn scan_links(state: State<'_, AppState>) -> Result<Vec<LinkGroupView>, String> {
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  Ok(scan::scan_links(lib, &lib.format_icons))
}
