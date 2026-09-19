//! 工具：三批写工具一个（PRD 08-06 三批，2026-09-20 用户定：允许 AI 丢回收站、不许彻底删除）
//! ——trash_placements。annotations(read_only_hint = false) + write_gate 门禁；
//! 撤销口径：进撤销（复用 commands::operations 抽出的 delete_placements_op 整段，
//! 锁 library → 锁 undo → 单写点 + 单撤销步，与 APP 命令同一实现）。
//! 红线：彻底删除 / 清空回收站 / 磁盘物理删除永不经 MCP（recycle.rs / purge.rs 不注册工具）。

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::{ErrorData, schemars, tool, tool_router};
use tauri::Manager;

use super::structure::PlacementRefParams;
use crate::commands::library::AppState;
use crate::server::mcp::{AnikitMcp, json_result, write_gate};
use crate::services::operations::PlacementRef;

/// trash_placements 入参
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TrashPlacementsParams {
  /// 待丢回收站 Placement 清单（单条 = 1 项数组）
  pub items: Vec<PlacementRefParams>,
}

#[tool_router(router = trash_router, vis = "pub")]
impl AnikitMcp {
  /// 把 Placement 丢到库内回收站（recycle.json 按资产聚合快照，磁盘文件不动）；
  /// 陈旧 / 重复三元组自动跳过不计，返回 { requested 请求条数 }。
  /// 进撤销（APP 内 Ctrl+Z 可撤、回收站可还原）；
  /// 彻底删除与清空回收站永不经 MCP，须 APP 内人工
  #[tool(name = "trash_placements", annotations(read_only_hint = false))]
  fn trash_placements(&self, Parameters(p): Parameters<TrashPlacementsParams>) -> Result<CallToolResult, ErrorData> {
    if let Some(err) = write_gate() {
      return Ok(err);
    }
    if p.items.is_empty() {
      return Ok(CallToolResult::error(vec![ContentBlock::text("items 至少 1 项")]));
    }
    let requested = p.items.len();
    let items: Vec<PlacementRef> = p
      .items
      .into_iter()
      .map(|it| PlacementRef { asset_id: it.asset_id, folder_id: it.folder_id, origin: it.origin })
      .collect();
    let state = self.app.state::<AppState>();
    match crate::commands::operations::delete_placements_op(&state, &items, Some("recycle")) {
      Ok(()) => {
        self.emit_library_changed("trash_placements");
        json_result(&serde_json::json!({ "requested": requested }))
      }
      Err(msg) => Ok(CallToolResult::error(vec![ContentBlock::text(msg)])),
    }
  }
}
