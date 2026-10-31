//! 工具：语义检索与分析任务（PRD 08-06，2026-10-25）。检索只读既有派生索引，绝不因查询
//! 自动分析；任务启动 / 控制均属受控写操作，逐次经过 write_gate，不进素材结构撤销栈。

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::{ErrorData, schemars, tool, tool_router};
use tauri::Manager;

use crate::server::mcp::{AnikitMcp, json_result, write_gate};
use crate::services::search::{SearchFilters, SearchScope};
use crate::services::semantic::{analysis, search};

/// semantic_search 入参
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SemanticSearchParams {
  /// 中文等自然语言查询内容；只匹配已分析的片段
  pub query: String,
  /// 限定文件夹 id；缺省 = 全库，anikit://folder/{id} 链接形态同吃
  #[serde(default, deserialize_with = "super::de_ref_id_opt")]
  pub folder_id: Option<String>,
  /// 限定文件夹时是否含全部子文件夹；缺省 true（未限定文件夹时无效）
  pub recursive: Option<bool>,
  /// 最多返回的资产数；缺省 30，业务层钳制为 1～100
  pub limit: Option<usize>,
}

/// 基础 / 详细分析启动入参：只允许显式资产清单，不开放 folderId 或缺省全库
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisParams {
  /// 待分析资产 id，至少 1 项；anikit://item/{id} 等链接形态同吃
  #[serde(deserialize_with = "super::de_ref_id_vec")]
  pub asset_ids: Vec<String>,
}

/// 任务查询 / 控制入参
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisTaskParams {
  /// start_semantic_analysis 等工具返回的 taskId；仅接受当前素材库任务
  pub task_id: String,
}

/// 业务 String 错误统一回工具级错误，不伪装成功、不升级为协议异常
fn domain_result<T: serde::Serialize>(result: Result<T, String>) -> Result<CallToolResult, ErrorData> {
  match result {
    Ok(value) => json_result(&value),
    Err(msg) => Ok(CallToolResult::error(vec![ContentBlock::text(msg)])),
  }
}

/// 受控写操作公共闸门；闭包只在开闸后执行
fn write_result<T: serde::Serialize>(f: impl FnOnce() -> Result<T, String>) -> Result<CallToolResult, ErrorData> {
  if let Some(err) = write_gate() {
    return Ok(err);
  }
  domain_result(f())
}

/// 锁外调用语义检索：服务自行短锁取候选，之后可能加载模型并推理
fn run_search(app: &tauri::AppHandle, p: &SemanticSearchParams) -> Result<serde_json::Value, String> {
  let scope = SearchScope {
    global: p.folder_id.is_none(),
    view_key: p.folder_id.as_ref().map(|id| format!("folder:{id}")).unwrap_or_else(|| "quick:all".to_string()),
    recursive: p.recursive.unwrap_or(true),
    filters: SearchFilters::default(),
  };
  let hits = search::semantic_search(app, &p.query, &scope, p.limit)?;
  let mut value = serde_json::to_value(&hits).map_err(|e| e.to_string())?;
  let port = crate::server::mcp::status().1.unwrap_or(17701);
  if let Some(items) = value.get_mut("hits").and_then(|v| v.as_array_mut()) {
    for hit in items {
      if let Some(id) = hit.pointer("/best/assetId").and_then(|v| v.as_str()).map(str::to_owned) {
        hit["uri"] = serde_json::Value::String(crate::core::anikit_ref::open_uri(port, "item", &id));
      }
    }
  }
  Ok(value)
}

#[tool_router(router = semantic_router, vis = "pub")]
impl AnikitMcp {
  /// 中文语义检索既有索引，返回每个资产的最佳片段、其他命中段、真实时间戳 / 帧索引与 uri。
  /// 支持 folderId 与 recursive；绝不因查询自动分析，也不调用外部 provider
  #[tool(name = "semantic_search", annotations(read_only_hint = true))]
  fn semantic_search(&self, Parameters(p): Parameters<SemanticSearchParams>) -> Result<CallToolResult, ErrorData> {
    domain_result(run_search(&self.app, &p))
  }

  /// 查询当前素材库的一个语义分析任务；不存在或不属于当前库时返回 null
  #[tool(name = "get_semantic_analysis_task", annotations(read_only_hint = true))]
  fn get_semantic_analysis_task(&self, Parameters(p): Parameters<AnalysisTaskParams>) -> Result<CallToolResult, ErrorData> {
    domain_result(analysis::get_task(&self.app, &p.task_id))
  }

  /// 列出当前素材库的语义分析任务（含模式、状态、进度与失败项）
  #[tool(name = "list_semantic_analysis_tasks", annotations(read_only_hint = true))]
  fn list_semantic_analysis_tasks(&self) -> Result<CallToolResult, ErrorData> {
    domain_result(analysis::list_tasks(&self.app))
  }

  /// 启动基础画面分析，返回 taskId。assetIds 必须是非空数组；不支持 folderId 或缺省全库。
  /// 受 MCP 写入闸控制，不进素材结构撤销栈，进度经任务检查点恢复
  #[tool(name = "start_semantic_analysis", annotations(read_only_hint = false))]
  fn start_semantic_analysis(&self, Parameters(p): Parameters<AnalysisParams>) -> Result<CallToolResult, ErrorData> {
    write_result(|| {
      if p.asset_ids.is_empty() {
        return Err("assetIds 至少 1 项".to_string());
      }
      analysis::start(&self.app, Some(p.asset_ids), None, false)
    })
  }

  /// 启动详细语义分析，返回 taskId。assetIds 必须是非空数组；仅使用用户已配置的 provider，
  /// 不返回或配置 API key。受 MCP 写入闸控制，不进素材结构撤销栈
  #[tool(name = "start_detailed_semantic_analysis", annotations(read_only_hint = false))]
  fn start_detailed_semantic_analysis(&self, Parameters(p): Parameters<AnalysisParams>) -> Result<CallToolResult, ErrorData> {
    write_result(|| {
      if p.asset_ids.is_empty() {
        return Err("assetIds 至少 1 项".to_string());
      }
      analysis::start_detailed(&self.app, p.asset_ids)
    })
  }

  /// 暂停当前素材库的指定分析任务。受 MCP 写入闸控制，不进素材结构撤销栈
  #[tool(name = "pause_semantic_analysis", annotations(read_only_hint = false))]
  fn pause_semantic_analysis(&self, Parameters(p): Parameters<AnalysisTaskParams>) -> Result<CallToolResult, ErrorData> {
    write_result(|| analysis::pause(&self.app, &p.task_id))
  }

  /// 恢复当前素材库的指定暂停任务，返回原 taskId 或恢复中断任务时的新 taskId；保持原分析模式
  #[tool(name = "resume_semantic_analysis", annotations(read_only_hint = false))]
  fn resume_semantic_analysis(&self, Parameters(p): Parameters<AnalysisTaskParams>) -> Result<CallToolResult, ErrorData> {
    write_result(|| analysis::resume(&self.app, &p.task_id))
  }

  /// 取消当前素材库的指定分析任务。受 MCP 写入闸控制，不进素材结构撤销栈
  #[tool(name = "cancel_semantic_analysis", annotations(read_only_hint = false))]
  fn cancel_semantic_analysis(&self, Parameters(p): Parameters<AnalysisTaskParams>) -> Result<CallToolResult, ErrorData> {
    write_result(|| analysis::cancel(&self.app, &p.task_id))
  }

  /// 重试当前素材库指定任务的失败 / 未完成素材，返回新 taskId；保持原分析模式
  #[tool(name = "retry_semantic_analysis", annotations(read_only_hint = false))]
  fn retry_semantic_analysis(&self, Parameters(p): Parameters<AnalysisTaskParams>) -> Result<CallToolResult, ErrorData> {
    write_result(|| analysis::retry(&self.app, &p.task_id))
  }

  /// 把已检索的语义片段创建为虚拟片段或物理截取资产。逐次过写入闸，沿用普通创建撤销与库变更广播
  #[tool(name = "create_semantic_segment", annotations(read_only_hint = false))]
  fn create_semantic_segment(&self, Parameters(p): Parameters<SegmentParams>) -> Result<CallToolResult, ErrorData> {
    if let Some(err) = write_gate() {
      return Ok(err);
    }
    let request = crate::commands::semantic::SegmentRequest {
      asset_id: p.asset_id,
      segment_id: p.segment_id,
      target_folder_id: p.target_folder_id,
      physical: p.physical,
    };
    let result = crate::commands::semantic::create_semantic_segment_op(&self.app.state::<crate::commands::library::AppState>(), &request);
    match result {
      Ok(view) => {
        self.emit_library_changed("create_semantic_segment");
        json_result(&view)
      }
      Err(msg) => Ok(CallToolResult::error(vec![ContentBlock::text(msg)])),
    }
  }
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SegmentParams {
  /// 原始素材 id（不是已创建的虚拟片段）；anikit://item/{id} 链接形态同吃
  #[serde(deserialize_with = "super::de_ref_id")]
  pub asset_id: String,
  /// semantic_search 返回的 best.segmentId 或 segments[].segmentId
  pub segment_id: i64,
  /// 新片段的放置文件夹；缺省 = 未分类
  #[serde(default, deserialize_with = "super::de_ref_id_opt")]
  pub target_folder_id: Option<String>,
  /// false = 引用源范围的虚拟片段；true = ffmpeg 物理截取为独立普通资产
  #[serde(default)]
  pub physical: bool,
}
