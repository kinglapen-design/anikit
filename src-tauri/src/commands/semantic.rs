use tauri::Manager;
use crate::{core::config::{self, SemanticSettings}, services::semantic::{analysis, provider_config, provider_http, provider_types::{ProviderConfigView, ProviderConnection, ProviderSettings}, types::TaskSnapshot, worker::{self, SemanticRuntimeStatus}}};

#[tauri::command]
pub async fn get_semantic_settings() -> Result<SemanticSettings, String> {
  config::semantic_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_semantic_settings(app: tauri::AppHandle, settings: SemanticSettings) -> Result<(), String> {
  config::save_semantic_settings(settings).map_err(|e| e.to_string())?;
  let idle = app.state::<crate::commands::library::AppState>().semantic_tasks.lock().map_err(|e| e.to_string())?.is_empty();
  if idle {
    worker::reset_worker();
  }
  Ok(())
}

#[tauri::command]
pub async fn test_semantic_runtime(app: tauri::AppHandle) -> Result<SemanticRuntimeStatus, String> {
  let busy = !app.state::<crate::commands::library::AppState>().semantic_tasks.lock().map_err(|e| e.to_string())?.is_empty();
  if busy {
    return Err("分析任务进行中，当前任务继续使用启动时配置；请完成后测试新设备或模型目录".into());
  }
  worker::test_runtime()
}

#[tauri::command]
pub async fn get_semantic_provider_settings() -> Result<ProviderConfigView, String> {
  provider_config::get_view()
}

#[tauri::command]
pub async fn save_semantic_provider_settings(settings: ProviderSettings) -> Result<(), String> {
  provider_config::save(settings)
}

#[tauri::command]
pub async fn set_semantic_provider_key(key: String) -> Result<(), String> {
  provider_config::set_key(key)
}

#[tauri::command]
pub async fn clear_semantic_provider_key() -> Result<(), String> {
  provider_config::clear_key()
}

#[tauri::command]
pub async fn test_semantic_provider() -> Result<ProviderConnection, String> {
  provider_http::test_connection(&provider_config::settings()?)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisRequest {
  pub asset_ids: Option<Vec<String>>,
  pub folder_id: Option<String>,
  #[serde(default)]
  pub recursive: bool,
}

#[tauri::command]
pub async fn start_semantic_analysis(app: tauri::AppHandle, request: AnalysisRequest) -> Result<String, String> {
  analysis::start(&app, request.asset_ids, request.folder_id, request.recursive)
}

#[tauri::command]
pub async fn start_detailed_semantic_analysis(app: tauri::AppHandle, asset_ids: Vec<String>) -> Result<String, String> {
  analysis::start_detailed(&app, asset_ids)
}

#[tauri::command]
pub async fn get_semantic_analysis_task(app: tauri::AppHandle, task_id: String) -> Result<Option<TaskSnapshot>, String> {
  analysis::get_task(&app, &task_id)
}

#[tauri::command]
pub async fn list_semantic_analysis_tasks(app: tauri::AppHandle) -> Result<Vec<TaskSnapshot>, String> {
  analysis::list_tasks(&app)
}

#[tauri::command]
pub async fn pause_semantic_analysis(app: tauri::AppHandle, task_id: String) -> Result<(), String> {
  analysis::pause(&app, &task_id)
}

#[tauri::command]
pub async fn resume_semantic_analysis(app: tauri::AppHandle, task_id: String) -> Result<String, String> {
  analysis::resume(&app, &task_id)
}

#[tauri::command]
pub async fn cancel_semantic_analysis(app: tauri::AppHandle, task_id: String) -> Result<(), String> {
  analysis::cancel(&app, &task_id)
}

#[tauri::command]
pub async fn retry_semantic_analysis(app: tauri::AppHandle, task_id: String) -> Result<String, String> {
  analysis::retry(&app, &task_id)
}

#[tauri::command]
pub async fn semantic_search(app: tauri::AppHandle, query: String, scope: crate::services::search::SearchScope, limit: Option<usize>) -> Result<crate::services::semantic::search::SemanticSearchHits, String> {
  crate::services::semantic::search::semantic_search(&app, &query, &scope, limit)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentRequest {
  pub asset_id: String,
  pub segment_id: i64,
  #[serde(default)]
  pub target_folder_id: Option<String>,
  #[serde(default)]
  pub physical: bool,
}

#[tauri::command]
pub async fn create_semantic_segment(app: tauri::AppHandle, request: SegmentRequest) -> Result<crate::services::views::PlacementView, String> {
  let state = app.state::<crate::commands::library::AppState>();
  create_semantic_segment_op(&state, &request)
}

pub(crate) fn create_semantic_segment_op(state: &crate::commands::library::AppState, request: &SegmentRequest) -> Result<crate::services::views::PlacementView, String> {
  let prepared = crate::services::semantic::segment::prepare(state, &request.asset_id, request.segment_id, request.target_folder_id.as_deref(), request.physical)?;
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
  undo.begin(if request.physical { "物理截取片段" } else { "创建虚拟片段" });
  if let Err(error) = undo.record_file(&lib.root.join(crate::core::library::FOLDERS_FILE)) {
    crate::services::semantic::segment::discard(&prepared);
    return Err(error.to_string());
  }
  match crate::services::semantic::segment::commit(lib, prepared) {
    Ok((view, info, backup)) => {
      undo.record_disk_op(
        crate::core::undo::DiskOp::Rename { from: info.clone(), to: backup.clone() },
        crate::core::undo::DiskOp::Rename { from: backup, to: info },
      );
      undo.commit().map_err(|e| e.to_string())?;
      Ok(view)
    }
    Err(error) => Err(crate::commands::operations::fail(lib, &mut undo, error)),
  }
}
