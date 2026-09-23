//! 工具：import_paths（二批，PRD 08-06 方案一冲突策略，2026-09-19 用户定）——Agent 外部生成文件
//! 入库入口。annotations(read_only_hint = false) + write_gate 门禁。
//!
//! 接入方式：**逐文件同步直调既有 run_import 管线**（四阶段并行流水线：hash 查重 → 建档 →
//! 缩略图 → Placement 注册 + folders.json 节流落盘，单写点不破）——不另造导入管线；逐文件驱动
//! 换来回包逐文件精确归属（Asset 事件不带源路径，批量调用无法对账）。on_event 闭包由本工具
//! 自用（APP 前端的 Channel 无人订阅本调用，事件无害）；调用同步等待完成（工具回包即最终态，
//! 无进度事件、不进 imports 任务注册表、不支持取消——与 APP 异步任务形态不同，如实告知）。
//!
//! 冲突策略 = 方案一（不阻断）：hash 完全相同 → 管线产出 ConflictItem（本工具映射为
//! skippedDuplicate + 已有资产 id 回给 Agent）；同名不同内容 → add_placement 既有同名单一化
//!（`_序号` 后缀，core::library::unique_name 口径）入库（映射为 renamed + finalName）。
//! **撤销口径：不进撤销**——APP 导入管线本就不接 UndoManager（导入 = 资产池新增，
//! 撤销体系覆盖库结构操作不覆盖导入），MCP 同口径不硬接。

use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::{ErrorData, schemars, tool, tool_router};
use tauri::Manager;

use crate::commands::library::AppState;
use crate::server::mcp::{AnikitMcp, json_result, write_gate};
use crate::services::import::{ImportEvent, ImportOptions, ROOT_FOLDER_ID, SequenceMode, Source, Structure};
use crate::services::import_task::run_import;

/// import_paths 入参
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ImportPathsParams {
  /// 磁盘文件绝对路径清单（仅文件，文件夹请展开为文件清单逐个给）
  pub paths: Vec<String>,
  /// 目标文件夹 id（取自 list_folder_tree 节点 id；"root" = 未分类；缺省 = 未分类）
  pub folder_id: Option<String>,
}

/// 逐文件处理报告（方案一：imported / skippedDuplicate / renamed / failed）
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportItemReport {
  /// 源文件绝对路径（入参原样）
  path: String,
  /// imported = 新入库；skippedDuplicate = hash 完全相同跳过；renamed = 同名不同内容加后缀入库；failed = 失败
  status: &'static str,
  /// 资产 id（skippedDuplicate = 库内已有资产 id，可直接拿去用）
  #[serde(skip_serializing_if = "Option::is_none")]
  asset_id: Option<String>,
  /// 最终显示名（renamed 为加后缀后的名字；skippedDuplicate 为已有资产名）
  #[serde(skip_serializing_if = "Option::is_none")]
  final_name: Option<String>,
  /// 失败原因（status = failed 时携带）
  #[serde(skip_serializing_if = "Option::is_none")]
  reason: Option<String>,
}

/// import_paths 回包：逐文件报告 + 各状态计数
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportReport {
  items: Vec<ImportItemReport>,
  imported: usize,
  skipped_duplicate: usize,
  renamed: usize,
  failed: usize,
}

#[tool_router(router = import_router, vis = "pub")]
impl AnikitMcp {
  /// 导入磁盘文件入库（仅文件；复制语义——源文件不动），缩略图随管线自动生成（导入即可看图）。
  /// 冲突不阻断：hash 完全相同 → 跳过并回已有资产 id；同名不同内容 → 自动加 _序号 后缀入库；
  /// 逐文件报告 { path, status, assetId?, finalName?, reason? }。不进撤销；同步执行、不支持取消
  #[tool(name = "import_paths", annotations(read_only_hint = false))]
  fn import_paths(&self, Parameters(p): Parameters<ImportPathsParams>) -> Result<CallToolResult, ErrorData> {
    if let Some(err) = write_gate() {
      return Ok(err);
    }
    if p.paths.is_empty() {
      return Ok(CallToolResult::error(vec![ContentBlock::text("paths 至少 1 项")]));
    }
    let state = self.app.state::<AppState>();
    // 目标文件夹预检（None / "root" 放行，同 commands/import.rs ensure_target 口径）+ 短锁取库级路径
    let (root, assets_dir) = {
      let guard = state.library.lock().map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
      let Some(lib) = guard.as_ref() else {
        return Ok(CallToolResult::error(vec![ContentBlock::text("请先打开素材库")]));
      };
      if let Some(id) = p.folder_id.as_deref() {
        if id != ROOT_FOLDER_ID && !lib.index.folders.contains_key(id) {
          return Ok(CallToolResult::error(vec![ContentBlock::text("目标文件夹不存在，可能已被删除")]));
        }
      }
      (lib.root.clone(), lib.assets_dir.clone())
    };

    let mut items: Vec<ImportItemReport> = Vec::new();
    let cancel = AtomicBool::new(false); // MCP 同步导入不支持取消（不入 imports 任务注册表）
    for path in &p.paths {
      let src = PathBuf::from(path);
      if src.is_dir() {
        items.push(ImportItemReport {
          path: path.clone(),
          status: "failed",
          asset_id: None,
          final_name: None,
          reason: Some("本工具只收文件路径，文件夹请展开为文件清单逐个导入".to_string()),
        });
        continue;
      }
      let file_name = src.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
      // 逐文件直调既有管线：Asset 事件（注册成功）进捕获袋，冲突 / 失败从 outcome 对账
      let captured: Mutex<Vec<crate::services::views::PlacementView>> = Mutex::new(Vec::new());
      let outcome = run_import(
        &state.library,
        root.clone(),
        assets_dir.clone(),
        ImportOptions {
          structure: Structure::Flatten,
          source: Source::Copy,
          target_folder: p.folder_id.clone(),
          paths: vec![src],
          imported_from: None,
          sequence_mode: SequenceMode::Scatter,
        },
        |ev| {
          if let ImportEvent::Asset { placement } = ev {
            if let Ok(mut v) = captured.lock() {
              v.push(*placement);
            }
          }
        },
        &cancel,
      );
      for view in captured.into_inner().unwrap_or_default() {
        let renamed = view.name != file_name; // add_placement 同名单一化加 _序号（同名不同内容）
        items.push(ImportItemReport {
          path: path.clone(),
          status: if renamed { "renamed" } else { "imported" },
          asset_id: Some(view.asset_id),
          final_name: Some(view.name),
          reason: None,
        });
      }
      for c in outcome.conflicts {
        // 方案一：hash 完全相同（含同请求内重复给同一文件）→ 跳过 + 已有资产 id 回给 Agent
        items.push(ImportItemReport {
          path: path.clone(),
          status: "skippedDuplicate",
          asset_id: Some(c.existing_asset_id),
          final_name: Some(c.existing_name),
          reason: None,
        });
      }
      for f in outcome.failed {
        items.push(ImportItemReport { path: f.path, status: "failed", asset_id: None, final_name: None, reason: Some(f.reason) });
      }
    }

    let count = |s: &'static str| items.iter().filter(|it| it.status == s).count();
    let report = ImportReport {
      imported: count("imported"),
      skipped_duplicate: count("skippedDuplicate"),
      renamed: count("renamed"),
      failed: count("failed"),
      items,
    };
    // 有真实入库才广播热刷新（全 skippedDuplicate / failed 库未变，不打扰前端）
    if report.imported + report.renamed > 0 {
      self.emit_library_changed("import_paths");
    }
    json_result(&report)
  }
}
