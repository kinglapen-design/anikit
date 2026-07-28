//! 命令 API：最近使用（6.1 第二轮，PRD 02 快速访问"最近使用"）。
//! 只读查询，无锁序要求；记录入口在 preview.rs open_with_default_app（打开成功才记）。

use tauri::State;

use crate::commands::library::AppState;
use crate::services::recent::{self, RecentAssetView};

/// 最近使用列表：viewedAt 倒序；回收站资产（无任何 Placement）跳过
#[tauri::command]
pub async fn list_recent_assets(state: State<'_, AppState>) -> Result<Vec<RecentAssetView>, String> {
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  Ok(recent::list_recent_assets(lib))
}
