use crate::core::config::{self, SemanticSettings};

#[tauri::command]
pub async fn get_semantic_settings() -> Result<SemanticSettings, String> {
  config::semantic_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_semantic_settings(settings: SemanticSettings) -> Result<(), String> {
  config::save_semantic_settings(settings).map_err(|e| e.to_string())
}
