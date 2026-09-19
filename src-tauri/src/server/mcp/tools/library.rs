//! 工具：当前空间 / 素材库状态（commands/library.rs get_current_space / get_current_library 同数据）

use rmcp::model::CallToolResult;
use rmcp::{ErrorData, tool, tool_router};
use tauri::Manager;

use crate::server::mcp::{AnikitMcp, json_result};
use crate::commands::library::{AppState, LibraryInfo, SpaceInfo, space_info, to_info};

/// 当前状态回包：未打开对应字段为 null（库未开不算错误——本工具就是给 Agent 探状态的）
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CurrentState {
  space: Option<SpaceInfo>,
  library: Option<LibraryInfo>,
}

#[tool_router(router = library_router, vis = "pub")]
impl AnikitMcp {
  /// 当前打开的储存空间与素材库状态（未打开对应字段为 null）。先调它确认库已就绪，再调其余工具
  #[tool(name = "get_current_library", annotations(read_only_hint = true))]
  fn get_current_library(&self) -> Result<CallToolResult, ErrorData> {
    let state = self.app.state::<AppState>();
    let space = state.space.lock().map_err(|e| ErrorData::internal_error(e.to_string(), None))?.as_ref().map(space_info);
    let library = state.library.lock().map_err(|e| ErrorData::internal_error(e.to_string(), None))?.as_ref().map(to_info);
    json_result(&CurrentState { space, library })
  }
}
