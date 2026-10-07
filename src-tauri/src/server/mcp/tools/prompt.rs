//! 工具：提示词写工具（2026-09-26 提示词轮，PRD 03_数据链接 02_文件「提示词」，两个，全部
//! annotations(read_only_hint = false)）——assign_prompt / clear_prompt。
//! 提示词 = 对库内 md/txt 文本资产的「引用 + 内容快照」：平时显示快照（源文档被彻底删除
//! 不断链——引用置空、快照保留降级为已扫描态），可跳转源文件。
//! 门禁 = 全局写入开关（mod.rs write_gate）；撤销口径 = 进撤销（复用 commands::prompt 抽出的
//! _op 整段，单写点 + 单撤销步，与 APP 命令同一实现）。

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::{ErrorData, schemars, tool, tool_router};
use tauri::Manager;

use crate::commands::library::AppState;
use crate::server::mcp::{AnikitMcp, json_result, write_gate};

/// assign_prompt / clear_prompt 入参（目标二选一）
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PromptTargetParams {
  /// 目标资产 id（与 folderId 二选一；anikit://item/{id} 链接形态同吃）
  #[serde(deserialize_with = "super::de_ref_id_opt")]
  pub asset_id: Option<String>,
  /// 目标档案 folderId（与 assetId 二选一）
  #[serde(deserialize_with = "super::de_ref_id_opt")]
  pub folder_id: Option<String>,
  /// 【assign_prompt 必填】提示词文档资产 id（库内 md/txt 文本资产）
  #[serde(deserialize_with = "super::de_ref_id_opt")]
  pub prompt_asset_id: Option<String>,
}

/// 目标二选一校验 + 分流到资产侧 / 档案侧 _op（is_archive = true 走档案侧）
fn with_target(
  app: &tauri::AppHandle,
  p: &PromptTargetParams,
  f: impl FnOnce(&AppState, &str, bool) -> Result<(), String>,
) -> Result<(), String> {
  let state = app.state::<AppState>();
  match (p.asset_id.as_deref(), p.folder_id.as_deref()) {
    (Some(a), None) => f(&state, a, false),
    (None, Some(fld)) => f(&state, fld, true),
    _ => Err("assetId 与 folderId 须且只须给一个".to_string()),
  }
}

#[tool_router(router = prompt_router, vis = "pub")]
impl AnikitMcp {
  /// 指定提示词：把库内 md/txt 文本资产设为某资产 / 档案的提示词文档（引用 + 内容快照——
  /// 快照随指定时刻写入，源后续改动经 APP 详情「保存提示词」同步；源被彻底删除不断链，
  /// 引用置空、快照保留降级为已扫描态）。目标 assetId / folderId 二选一。进撤销
  #[tool(name = "assign_prompt", annotations(read_only_hint = false))]
  fn assign_prompt(&self, Parameters(p): Parameters<PromptTargetParams>) -> Result<CallToolResult, ErrorData> {
    if let Some(err) = write_gate() {
      return Ok(err);
    }
    let Some(doc) = p.prompt_asset_id.as_deref() else {
      return Ok(CallToolResult::error(vec![ContentBlock::text("assign_prompt 缺少 promptAssetId")]));
    };
    match with_target(&self.app, &p, |state, id, is_archive| {
      if is_archive {
        crate::commands::prompt::assign_archive_prompt_op(state, id, doc)
      } else {
        crate::commands::prompt::assign_asset_prompt_op(state, id, doc)
      }
    }) {
      Ok(()) => {
        self.emit_library_changed("assign_prompt");
        json_result(&serde_json::json!({ "ok": true }))
      }
      Err(msg) => Ok(CallToolResult::error(vec![ContentBlock::text(msg)])),
    }
  }

  /// 断开提示词：移除目标资产 / 档案的提示词绑定——**快照随解绑清除**（不留半态；
  /// 「源删除留快照」仅由源文档被彻底删除触发）。目标 assetId / folderId 二选一。进撤销
  #[tool(name = "clear_prompt", annotations(read_only_hint = false))]
  fn clear_prompt(&self, Parameters(p): Parameters<PromptTargetParams>) -> Result<CallToolResult, ErrorData> {
    if let Some(err) = write_gate() {
      return Ok(err);
    }
    match with_target(&self.app, &p, |state, id, is_archive| {
      if is_archive {
        crate::commands::prompt::clear_archive_prompt_op(state, id, None)
      } else {
        crate::commands::prompt::clear_asset_prompt_op(state, id, None)
      }
    }) {
      Ok(()) => {
        self.emit_library_changed("clear_prompt");
        json_result(&serde_json::json!({ "ok": true }))
      }
      Err(msg) => Ok(CallToolResult::error(vec![ContentBlock::text(msg)])),
    }
  }
}
