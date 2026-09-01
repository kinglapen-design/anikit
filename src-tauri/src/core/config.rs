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
  /// 储存空间列表（名称、路径、最近打开时间），最近打开在前
  #[serde(default)]
  pub spaces: Vec<SpaceEntry>,
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
      spaces: Vec::new(),
      libraries: Vec::new(),
      workspaces: Vec::new(),
      settings: serde_json::json!({}),
    }
  }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpaceEntry {
  pub name: String,
  pub path: String,
  pub last_opened_at: String,
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

/// 主窗口几何（settings.window 子树，2026-07-28 用户定："用户设置要持久化，包括窗口大小，位置"）；
/// 预览子窗口不记录不恢复
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowState {
  pub x: i32,
  pub y: i32,
  pub width: u32,
  pub height: u32,
  pub maximized: bool,
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

/// 记录一次储存空间打开：去重、置顶、刷新最近打开时间
pub fn record_space_open(name: &str, path: &str) -> Result<(), DataError> {
  let p = config_path()?;
  let mut cfg = load_from(&p);
  upsert_space(&mut cfg, name, path);
  save_to(&p, &cfg)
}

fn upsert_space(cfg: &mut AppConfig, name: &str, path: &str) {
  cfg.spaces.retain(|e| e.path != path);
  cfg.spaces.insert(
    0,
    SpaceEntry {
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

// ---------- UI 设置与主窗口几何（2026-07-28 复验反馈：用户设置持久化） ----------

/// UI 设置读取（settings.ui 子树原样返回；从未保存为空对象，前端按默认值补齐）
pub fn ui_settings() -> Result<serde_json::Value, DataError> {
  Ok(load()?.settings.get("ui").cloned().unwrap_or_else(|| serde_json::json!({})))
}

/// CEP 导出设置（settings.cepExport 子树，8.1 二轮 2b）：插件 welcome 下发与接收服务
/// （cep_export 降级链）共用同一份——APP 设置页单写点，8.2 批量导出同读
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CepExportSettings {
  /// 动态导出时长上限（秒；jsx 侧帧数 = fps × 秒）
  pub max_secs: u32,
  /// WebP 体积软上限（字节；降级链目标，压不进保留产出）
  pub max_bytes: u64,
  /// WebP 长边像素上限（合成前缩放）
  pub max_edge: u32,
}

impl Default for CepExportSettings {
  fn default() -> Self {
    Self { max_secs: 10, max_bytes: 2 * 1024 * 1024, max_edge: 1024 }
  }
}

/// CEP 导出设置读取（缺字段按默认补齐）
pub fn cep_export_settings() -> Result<CepExportSettings, DataError> {
  let raw = load()?.settings.get("cepExport").cloned().unwrap_or_else(|| serde_json::json!({}));
  Ok(serde_json::from_value(raw).unwrap_or_default())
}

/// CEP 导出设置写回（settings.cepExport 单点覆盖）
pub fn save_cep_export_settings(value: CepExportSettings) -> Result<(), DataError> {
  let p = config_path()?;
  let mut cfg = load_from(&p);
  cfg.settings["cepExport"] = serde_json::to_value(value).map_err(|e| DataError::Corrupt(e.to_string()))?;
  save_to(&p, &cfg)
}
/// UI 设置整体写回（前端变更防抖 500ms 后整体提交，settings.ui 单点覆盖）
pub fn save_ui_settings(value: serde_json::Value) -> Result<(), DataError> {
  let p = config_path()?;
  let mut cfg = load_from(&p);
  cfg.settings["ui"] = value;
  save_to(&p, &cfg)
}

/// 主窗口几何读取（启动恢复用；从未保存 / 形状损坏为 None，走 tauri.conf 默认居中）
pub fn window_state() -> Option<WindowState> {
  let cfg = load().ok()?;
  serde_json::from_value(cfg.settings.get("window")?.clone()).ok()
}

/// 主窗口几何写回（前端 resize/move 防抖提交）。
/// 最大化时只刷新标志位：几何保留上次的还原尺寸——退出最大化后不丢位置
pub fn save_window_state(state: WindowState) -> Result<(), DataError> {
  let p = config_path()?;
  let mut cfg = load_from(&p);
  let old = cfg.settings.get("window").cloned().and_then(|v| serde_json::from_value::<WindowState>(v).ok());
  let merged = merge_window_state(old, state);
  cfg.settings["window"] = serde_json::to_value(merged).map_err(|e| DataError::Corrupt(e.to_string()))?;
  save_to(&p, &cfg)
}

/// 最大化合并（纯逻辑，单测直调）：最大化只刷标志位、几何保留旧值；非最大化整体覆盖
fn merge_window_state(old: Option<WindowState>, new: WindowState) -> WindowState {
  if new.maximized {
    old.map(|o| WindowState { maximized: true, ..o }).unwrap_or(new)
  } else {
    new
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
  fn upsert_space_dedupes_and_prepends() {
    let mut cfg = AppConfig::default();
    upsert_space(&mut cfg, "空间A", "D:/sa");
    upsert_space(&mut cfg, "空间B", "D:/sb");
    upsert_space(&mut cfg, "空间A改", "D:/sa"); // 重复路径：去重置顶并更新名称
    assert_eq!(cfg.spaces.len(), 2);
    assert_eq!(cfg.spaces[0].name, "空间A改");
    assert_eq!(cfg.spaces[1].name, "空间B");
  }

  #[test]
  fn old_config_without_spaces_loads() {
    // 旧版 config.json 无 spaces 字段（serde default 兼容），其余字段不丢
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    std::fs::write(
      &path,
      r#"{ "version": 1, "libraries": [ { "name": "库", "path": "E:/库", "lastOpenedAt": "2026-08-01T00:00:00+08:00" } ] }"#,
    )
    .unwrap();
    let cfg = load_from(&path);
    assert!(cfg.spaces.is_empty());
    assert_eq!(cfg.libraries.len(), 1);
    assert_eq!(cfg.libraries[0].name, "库");
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

  #[test]
  fn ui_settings_roundtrip_via_settings_subtree() {
    // settings.ui 子树整体覆盖写回（前端防抖提交语义），不碰资源库列表等其他字段
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("AniKit/config.json");
    let mut cfg = AppConfig::default();
    upsert_library(&mut cfg, "素材库", "E:/素材库");
    cfg.settings["ui"] = serde_json::json!({ "layoutMode": "grid", "timeDisplay": "frames" });
    save_to(&path, &cfg).unwrap();
    let back = load_from(&path);
    assert_eq!(back.settings["ui"]["layoutMode"], "grid");
    assert_eq!(back.settings["ui"]["timeDisplay"], "frames");
    assert_eq!(back.libraries.len(), 1, "写 UI 设置不应动资源库列表");
    // 从未保存过 ui 的配置读回空对象（前端按默认值补齐）
    let plain = AppConfig::default();
    assert!(plain.settings.get("ui").is_none());
  }

  #[test]
  fn window_state_maximized_keeps_restore_geometry() {
    let normal = WindowState { x: 100, y: 50, width: 1340, height: 1150, maximized: false };
    // 非最大化整体覆盖
    assert_eq!(merge_window_state(None, normal), normal);
    // 最大化只刷标志位，几何保留上次还原尺寸
    let maxed = WindowState { x: 0, y: 0, width: 2560, height: 1440, maximized: true };
    let merged = merge_window_state(Some(normal), maxed);
    assert_eq!(merged, WindowState { maximized: true, ..normal });
    // 无旧值（首次最大化）按新值存
    assert_eq!(merge_window_state(None, maxed), maxed);
  }
}
