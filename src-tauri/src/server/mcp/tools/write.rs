//! 工具：二期首批写工具（PRD 08-06 写入闸形态 A，2026-09-19 用户定，三个，全部
//! annotations(read_only_hint = false)）——create_document / write_text_asset / rename_placements。
//! 门禁 = 全局写入开关（mod.rs write_gate：settings.ui.mcpWriteEnabled 逐次检查，关态回
//! 「MCP 写入未开启」工具级错误）；撤销口径 = create_document / rename_placements 进撤销
//!（复用 commands::operations 命令整段抽出的 _op 函数，单写点 + 单撤销步），
//! write_text_asset 不进撤销（与 APP 内文本编辑同口径，2026-08-23 定，工具描述同口径告知 Agent）。

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::{ErrorData, schemars, tool, tool_router};
use tauri::Manager;

use crate::commands::library::AppState;
use crate::server::mcp::{AnikitMcp, json_result, write_gate};
use crate::services::operations::RenameItem;

/// create_document 入参
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateDocumentParams {
  /// 文档类型：md / txt / fla / psd / ai / docx / xlsx（写提示词场景主要 md）
  pub kind: String,
  /// 目标文件夹 id（取自 list_folder_tree 节点 id；"root" = 根目录）；缺省 = 根目录
  pub folder_id: Option<String>,
}

/// write_text_asset 入参
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WriteTextParams {
  /// 资产 id（search_assets / list_placements 回包里的 assetId）
  pub asset_id: String,
  /// 新内容全文（整体覆写，恒 UTF-8 无 BOM；上限 2MB）
  pub content: String,
}

/// rename_placements 单项（同 APP 批量重命名 RenameItem 口径）
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenameItemParams {
  /// 资产 id
  pub asset_id: String,
  /// 所在文件夹 id（Placement 定位二元组之一，取自 list_placements 回包 folderId）
  pub folder_id: String,
  /// true = 原始 Placement（磁盘文件名 originName 与词干缩略图同步改名）；false / 缺省 = 仅改显示名
  #[serde(default)]
  pub origin: bool,
  /// 新名（含扩展名；origin 项无扩展名时自动补原扩展名）
  pub new_name: String,
}

/// rename_placements 入参
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenamePlacementsParams {
  /// 改名清单（单条改名 = 1 项数组）
  pub items: Vec<RenameItemParams>,
}

#[tool_router(router = write_router, vis = "pub")]
impl AnikitMcp {
  /// 在素材库新建文档入库（md / txt / fla / psd / ai / docx / xlsx），返回 PlacementView。
  /// 写入操作：进撤销（APP 内 Ctrl+Z 可撤回）；需先在 APP 偏好设置开启「允许 MCP 写入」
  #[tool(name = "create_document", annotations(read_only_hint = false))]
  fn create_document(&self, Parameters(p): Parameters<CreateDocumentParams>) -> Result<CallToolResult, ErrorData> {
    if let Some(err) = write_gate() {
      return Ok(err);
    }
    let state = self.app.state::<AppState>();
    match crate::commands::operations::create_document_op(&state, &p.kind, p.folder_id.as_deref()) {
      Ok(view) => json_result(&view),
      Err(msg) => Ok(CallToolResult::error(vec![ContentBlock::text(msg)])),
    }
  }

  /// 整体覆写文本资产（txt / md，提示词正文等）内容，metadata.size / modifiedAt 同步。
  /// **注意：内容编辑不进撤销（与 APP 内编辑同口径）——覆写不可撤回，覆写前先 read_text_asset 备份原文**；
  /// 需先在 APP 偏好设置开启「允许 MCP 写入」
  #[tool(name = "write_text_asset", annotations(read_only_hint = false))]
  fn write_text_asset(&self, Parameters(p): Parameters<WriteTextParams>) -> Result<CallToolResult, ErrorData> {
    if let Some(err) = write_gate() {
      return Ok(err);
    }
    self.with_lib_mut(|lib| crate::commands::text::write_text(lib, &p.asset_id, &p.content))
  }

  /// 批量改名（单条改名 = 1 项数组）：逐条改名，失败项跳过不整体回滚，
  /// 返回 BatchRenameReport { renamed 成功计数, failed 失败清单（新名 + 原因） }。
  /// 进撤销（整体单撤销步，APP 内 Ctrl+Z 可撤回）；需先在 APP 偏好设置开启「允许 MCP 写入」
  #[tool(name = "rename_placements", annotations(read_only_hint = false))]
  fn rename_placements(&self, Parameters(p): Parameters<RenamePlacementsParams>) -> Result<CallToolResult, ErrorData> {
    if let Some(err) = write_gate() {
      return Ok(err);
    }
    if p.items.is_empty() {
      return Ok(CallToolResult::error(vec![ContentBlock::text("items 至少 1 项")]));
    }
    let items: Vec<RenameItem> = p
      .items
      .into_iter()
      .map(|it| RenameItem { asset_id: it.asset_id, folder_id: it.folder_id, origin: it.origin, new_name: it.new_name })
      .collect();
    let state = self.app.state::<AppState>();
    match crate::commands::operations::rename_placements_op(&state, &items) {
      Ok(report) => json_result(&report),
      Err(msg) => Ok(CallToolResult::error(vec![ContentBlock::text(msg)])),
    }
  }
}
