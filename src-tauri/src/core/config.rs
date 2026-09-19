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
/// （cep_export 降级链）共用同一份——单写点（APP 命令层 / CEP 面板 WS saveSettings 同写此处），
/// 8.2 批量导出同读。2026-09 重构：尺寸上限（maxEdge）废除（导出图片不再限像素，WebP 合成
/// 本就有体积压缩），旧配置残留 maxEdge 反序列化静默忽略（无 deny_unknown_fields）
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CepExportSettings {
  /// 动态导出时长上限（秒；jsx 侧帧数 = fps × 秒）
  pub max_secs: u32,
  /// WebP 体积软上限（字节；降级链目标，压不进保留产出）
  pub max_bytes: u64,
  /// 导出帧率（0 = 跟随文档帧率——新建 fla 文档的帧率设置）
  #[serde(default)]
  pub fps: f64,
}

impl Default for CepExportSettings {
  fn default() -> Self {
    Self { max_secs: 10, max_bytes: 2 * 1024 * 1024, fps: 0.0 }
  }
}

impl CepExportSettings {
  /// 参数护栏（APP 命令层 save 与插件 WS saveSettings 共用单一定义）：
  /// 时长 1-600 秒；体积 256KB-50MB；fps 0（跟随文档）或 1-120
  pub fn validate(&self) -> Result<(), String> {
    if self.max_secs == 0 || self.max_secs > 600 {
      return Err("时长上限须在 1-600 秒之间".to_string());
    }
    if self.max_bytes < 262_144 || self.max_bytes > 50 * 1024 * 1024 {
      return Err("大小上限须在 256KB-50MB 之间".to_string());
    }
    if self.fps != 0.0 && !(1.0..=120.0).contains(&self.fps) {
      return Err("帧率须为 0（跟随文档）或 1-120".to_string());
    }
    Ok(())
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

// ---------- MCP 设置（settings.mcp 子树，PRD 08-06；2026-09-20 插件面板 MCP 详情窗配套） ----------

/// MCP 设置：服务总开关（默认开，17701 不白听的用户可关）+ 写入闸（默认关）。
/// 独立子树原因：save_ui_settings 是 settings.ui 整体替换语义，MCP 两键若留 ui 子树，
/// 前端 store 快照（PERSIST_KEYS 不含）写回即被抹除；单写点 = commands/mcp.rs 两 set 命令。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpSettings {
  /// 服务总开关（缺省开）：启动时是否拉起 127.0.0.1:17701 Streamable HTTP server
  #[serde(default = "default_server_enabled")]
  pub server_enabled: bool,
  /// 写入闸（缺省关）：9 个写工具门禁，MCP 写调用逐次直读（server/mcp write_gate）
  #[serde(default)]
  pub write_enabled: bool,
}

fn default_server_enabled() -> bool {
  true
}

impl Default for McpSettings {
  fn default() -> Self {
    Self { server_enabled: true, write_enabled: false }
  }
}

/// MCP 设置读取（缺字段按默认补齐）。含一次性迁移：旧落点 settings.ui.mcpWriteEnabled
/// （2026-09-19 写入闸形态 A 初版）——settings.mcp 尚未建立时采用旧键值；下次 save_mcp_settings
/// 落新子树，ui 旧键随前端 store 字段退役（快照不再携带 + ui 整体替换写回）自然消亡
pub fn mcp_settings() -> Result<McpSettings, DataError> {
  Ok(parse_mcp_settings(&load()?.settings))
}

/// 纯解析（单测直调）：settings.mcp 优先；子树未建立时迁移旧 ui.mcpWriteEnabled
fn parse_mcp_settings(settings: &serde_json::Value) -> McpSettings {
  let mut s: McpSettings = settings
    .get("mcp")
    .cloned()
    .and_then(|raw| serde_json::from_value(raw).ok())
    .unwrap_or_default();
  if settings.get("mcp").is_none() {
    if let Some(old) = settings
      .get("ui")
      .and_then(|ui| ui.get("mcpWriteEnabled"))
      .and_then(|v| v.as_bool())
    {
      s.write_enabled = old;
    }
  }
  s
}

/// MCP 设置写回（settings.mcp 单点覆盖）
pub fn save_mcp_settings(value: McpSettings) -> Result<(), DataError> {
  let p = config_path()?;
  let mut cfg = load_from(&p);
  cfg.settings["mcp"] = serde_json::to_value(value).map_err(|e| DataError::Corrupt(e.to_string()))?;
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

  #[test]
  fn mcp_settings_defaults_migration_and_roundtrip() {
    // 空配置 = 默认：服务开 / 写入关
    assert_eq!(parse_mcp_settings(&serde_json::json!({})), McpSettings::default());
    // 迁移：settings.mcp 未建立时采用旧 ui.mcpWriteEnabled
    let old = serde_json::json!({ "ui": { "mcpWriteEnabled": true } });
    assert_eq!(parse_mcp_settings(&old), McpSettings { server_enabled: true, write_enabled: true });
    // settings.mcp 已建立 = 新子树优先，旧 ui 键不再影响
    let both = serde_json::json!({ "mcp": { "serverEnabled": false, "writeEnabled": false }, "ui": { "mcpWriteEnabled": true } });
    assert_eq!(parse_mcp_settings(&both), McpSettings { server_enabled: false, write_enabled: false });
    // 缺字段按默认补齐（serverEnabled 缺省 true）
    let partial: McpSettings = serde_json::from_value(serde_json::json!({ "writeEnabled": true })).unwrap();
    assert_eq!(partial, McpSettings { server_enabled: true, write_enabled: true });
    // 序列化落盘键名 camelCase
    let v = serde_json::to_value(McpSettings { server_enabled: false, write_enabled: true }).unwrap();
    assert_eq!(v, serde_json::json!({ "serverEnabled": false, "writeEnabled": true }));
  }

  #[test]
  fn cep_export_settings_compat_and_guard() {
    // 旧配置残留 maxEdge 静默忽略；fps 缺失 default 0.0（其余字段保留，不回退全默认）
    let s: CepExportSettings =
      serde_json::from_value(serde_json::json!({ "maxSecs": 5, "maxBytes": 1048576, "maxEdge": 1024 })).unwrap();
    assert_eq!(s, CepExportSettings { max_secs: 5, max_bytes: 1048576, fps: 0.0 });
    // 写回不再带 maxEdge；fps 照常落盘
    let v = serde_json::to_value(CepExportSettings::default()).unwrap();
    assert!(v.get("maxEdge").is_none());
    assert_eq!(v["fps"], 0.0);
    // 护栏：fps 0（跟随文档）/ 1-120 放行，其余拒绝（含负值、超界、NaN）
    assert!(CepExportSettings { fps: 0.0, ..Default::default() }.validate().is_ok());
    assert!(CepExportSettings { fps: 1.0, ..Default::default() }.validate().is_ok());
    assert!(CepExportSettings { fps: 120.0, ..Default::default() }.validate().is_ok());
    assert!(CepExportSettings { fps: 0.5, ..Default::default() }.validate().is_err());
    assert!(CepExportSettings { fps: 121.0, ..Default::default() }.validate().is_err());
    assert!(CepExportSettings { fps: -1.0, ..Default::default() }.validate().is_err());
    assert!(CepExportSettings { fps: f64::NAN, ..Default::default() }.validate().is_err());
    // 时长 / 体积护栏不变
    assert!(CepExportSettings { max_secs: 0, ..Default::default() }.validate().is_err());
    assert!(CepExportSettings { max_secs: 601, ..Default::default() }.validate().is_err());
    assert!(CepExportSettings { max_bytes: 100, ..Default::default() }.validate().is_err());
  }
}
