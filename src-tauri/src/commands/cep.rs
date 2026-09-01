//! 命令 API：CEP 插件安装与导出设置（8.1 二轮 2b）——安装状态查询 / 一键安装 / 导出参数读写。
//! 纯库外操作（文件复制 + 注册表 + 全局 config），不持库锁、不接撤销。

use crate::core::config::{self, CepExportSettings};
use crate::services::cep_install::{self, CepInstallStatus};

/// CEP 插件安装状态（设置页 / 菜单展示）
#[tauri::command]
pub async fn get_cep_install_status() -> Result<CepInstallStatus, String> {
  Ok(cep_install::status())
}

/// 一键安装 / 升级 CEP 插件（复制到 extensions + PlayerDebugMode 确保）
#[tauri::command]
pub async fn install_cep_plugin() -> Result<CepInstallStatus, String> {
  cep_install::install()
}

/// 导出设置读取（settings.cepExport 子树；CEP 面板与 8.2 批量导出共用）
#[tauri::command]
pub async fn get_cep_export_settings() -> Result<CepExportSettings, String> {
  config::cep_export_settings().map_err(|e| e.to_string())
}

/// 导出设置保存（写回 + WS 广播推送在线插件实时生效）
#[tauri::command]
pub async fn save_cep_export_settings(settings: CepExportSettings) -> Result<(), String> {
  if settings.max_secs == 0 || settings.max_secs > 600 {
    return Err("时长上限须在 1-600 秒之间".to_string());
  }
  if settings.max_edge < 128 || settings.max_edge > 4096 {
    return Err("尺寸上限须在 128-4096 像素之间".to_string());
  }
  if settings.max_bytes < 262_144 || settings.max_bytes > 50 * 1024 * 1024 {
    return Err("大小上限须在 256KB-50MB 之间".to_string());
  }
  config::save_cep_export_settings(settings).map_err(|e| e.to_string())?;
  crate::server::broadcast_settings();
  Ok(())
}
