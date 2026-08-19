//! 命令 API：分词搜索（只读，不走撤销）。

use tauri::State;

use crate::commands::library::AppState;
use crate::services::search::{self, SearchHits, SearchScope};

/// 分词搜索（3.6 一轮，PRD 03_查找文件 01）：空格分词 AND，返回命中 Placement 键集 + 命中档案 id 集
#[tauri::command]
pub async fn search_assets(state: State<'_, AppState>, query: String, scope: SearchScope) -> Result<SearchHits, String> {
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  Ok(search::search(lib, &query, &scope))
}
