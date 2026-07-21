//! 全局配置（PRD 06-01.3）：系统配置目录下 AniKit/config.json
//! （Windows：%APPDATA%\AniKit），内容：资源库列表 / 工作空间列表 / 全局设置。
//! 全局唯一，不属于任何库；丢失成本低，损坏时记日志并自愈重建。

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::core::json_io::{read_json, write_json, DataError};
use crate::core::models::DATA_VERSION;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
  pub version: u32,
  /// 资源库列表（名称、路径、最近打开时间），最近打开在前
  #[serde(default)]
  pub libraries: Vec<LibraryEntry>,
  #[serde(default)]
  pub workspaces: Vec<WorkspaceEntry>,
  /// 全局设置（热键、双击行为、布局默认值等），随用随加
  #[serde(default)]
  pub settings: serde_json::Value,
}

impl Default for AppConfig {
  fn default() -> Self {
    Self {
      version: DATA_VERSION,
      libraries: Vec::new(),
      workspaces: Vec::new(),
      settings: serde_json::json!({}),
    }
  }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryEntry {
  pub name: String,
  pub path: String,
  pub last_opened_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceEntry {
  pub name: String,
  pub path: String,
  pub last_opened_at: String,
}

pub fn config_path() -> Result<std::path::PathBuf, DataError> {
  dirs::config_dir()
    .map(|d| d.join("AniKit").join("config.json"))
    .ok_or_else(|| DataError::Corrupt("无法定位系统配置目录".into()))
}

pub fn load() -> Result<AppConfig, DataError> {
  Ok(load_from(&config_path()?))
}

/// 记录一次库打开：去重、置顶、刷新最近打开时间
pub fn record_library_open(name: &str, path: &str) -> Result<(), DataError> {
  let p = config_path()?;
  let mut cfg = load_from(&p);
  upsert_library(&mut cfg, name, path);
  save_to(&p, &cfg)
}

fn upsert_library(cfg: &mut AppConfig, name: &str, path: &str) {
  cfg.libraries.retain(|e| e.path != path);
  cfg.libraries.insert(
    0,
    LibraryEntry {
      name: name.to_string(),
      path: path.to_string(),
      last_opened_at: chrono::Local::now().to_rfc3339(),
    },
  );
}

fn load_from(path: &Path) -> AppConfig {
  if !path.exists() {
    return AppConfig::default();
  }
  match read_json(path) {
    Ok(cfg) => cfg,
    Err(e) => {
      // 配置文件损坏不阻断启动：记日志，以默认配置重建
      log::error!("全局配置损坏，已重置为默认：{e}");
      AppConfig::default()
    }
  }
}

fn save_to(path: &Path, cfg: &AppConfig) -> Result<(), DataError> {
  if let Some(parent) = path.parent() {
    std::fs::create_dir_all(parent)?;
  }
  write_json(path, cfg)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn upsert_dedupes_and_prepends() {
    let mut cfg = AppConfig::default();
    upsert_library(&mut cfg, "库A", "D:/a");
    upsert_library(&mut cfg, "库B", "D:/b");
    upsert_library(&mut cfg, "库A改", "D:/a"); // 重复路径：去重置顶并更新名称
    assert_eq!(cfg.libraries.len(), 2);
    assert_eq!(cfg.libraries[0].name, "库A改");
    assert_eq!(cfg.libraries[1].name, "库B");
  }

  #[test]
  fn save_then_load_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("AniKit/config.json");
    let mut cfg = AppConfig::default();
    upsert_library(&mut cfg, "素材库", "E:/素材库");
    save_to(&path, &cfg).unwrap();
    let back = load_from(&path);
    assert_eq!(back.libraries.len(), 1);
    assert_eq!(back.libraries[0].path, "E:/素材库");
    assert_eq!(back.version, DATA_VERSION);
  }

  #[test]
  fn corrupt_config_resets_to_default() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    std::fs::write(&path, "{ broken").unwrap();
    let cfg = load_from(&path);
    assert_eq!(cfg, AppConfig::default());
  }
}
