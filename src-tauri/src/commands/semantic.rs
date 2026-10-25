use crate::{core::config::{self, SemanticSettings}, services::semantic::worker::{self, SemanticRuntimeStatus}};

#[tauri::command]
pub async fn get_semantic_settings() -> Result<SemanticSettings, String> {
  config::semantic_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_semantic_settings(settings: SemanticSettings) -> Result<(), String> {
  config::save_semantic_settings(settings).map_err(|e| e.to_string())?;
  worker::reset_worker();
  Ok(())
}

#[tauri::command]
pub async fn test_semantic_runtime() -> Result<SemanticRuntimeStatus, String> {
  worker::test_runtime()
}
