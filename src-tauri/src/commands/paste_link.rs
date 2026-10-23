//! 命令 API：编辑器贴图直链（2026-10-23，PRD 05-01「编辑器贴图直链」条 + 02-09「by-name 形态」条）——
//! 插件面板详情窗数据源（Typora 自定义上传命令文本 / 收件箱真实目录）+ Obsidian 插件安装
//!（插件文件 include_bytes 嵌 exe、安装时写出——portable / dev 无路径分叉）。

use std::path::Path;

use tauri::State;

use crate::commands::library::AppState;
use crate::core::space::INBOX_DIR;

/// 详情窗数据源：command = Typora 自定义上传命令文本（exe 绝对路径含引号）；
/// toolPath = anikit-paste.exe 实际位置（当前进程同目录）；inboxDir = 收件箱真实目录（未开空间 = None）
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PasteLinkInfo {
  pub command: String,
  pub tool_path: String,
  pub inbox_dir: Option<String>,
}

/// 读取贴图直链信息（纯内存 + current_exe 快 syscall，sync 口径同 get_inbox_drive_letter）
#[tauri::command]
pub fn get_paste_link_info(state: State<'_, AppState>) -> Result<PasteLinkInfo, String> {
  let tool = std::env::current_exe()
    .map_err(|e| format!("取不到当前 exe 路径：{e}"))?
    .parent()
    .map(|p| p.join("anikit-paste.exe"))
    .ok_or_else(|| "取不到当前 exe 所在目录".to_string())?;
  let tool_path = tool.to_string_lossy().into_owned();
  let inbox_dir = {
    let guard = state.space.lock().map_err(|e| e.to_string())?;
    guard.as_ref().map(|s| s.root.join(INBOX_DIR).to_string_lossy().into_owned())
  };
  Ok(PasteLinkInfo { command: format!("\"{tool_path}\""), tool_path, inbox_dir })
}

/// Obsidian 插件文件（构建期自仓库 plugins/ 嵌入 exe——随版本升级，安装即覆写）
const OBSIDIAN_PLUGIN_MAIN: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../plugins/obsidian-anikit-paste/main.js"));
const OBSIDIAN_PLUGIN_MANIFEST: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../plugins/obsidian-anikit-paste/manifest.json"));

/// 安装 Obsidian 贴图插件：写入 {vault}/.obsidian/plugins/anikit-paste/（已存在 = 覆写升级），
/// 返回插件目录。async：文件 IO 一律走线程池（规范第 2 条）
#[tauri::command]
pub async fn install_obsidian_plugin(vault_path: String) -> Result<String, String> {
  tauri::async_runtime::spawn_blocking(move || install_obsidian_plugin_blocking(Path::new(&vault_path)))
    .await
    .map_err(|e| format!("安装任务执行失败：{e}"))?
}

fn install_obsidian_plugin_blocking(vault: &Path) -> Result<String, String> {
  if !vault.is_dir() {
    return Err("所选路径不是文件夹（请选择 Obsidian 库的根目录）".to_string());
  }
  let dir = vault.join(".obsidian").join("plugins").join("anikit-paste");
  std::fs::create_dir_all(&dir).map_err(|e| format!("创建插件目录失败：{e}"))?;
  std::fs::write(dir.join("main.js"), OBSIDIAN_PLUGIN_MAIN).map_err(|e| format!("写入 main.js 失败：{e}"))?;
  std::fs::write(dir.join("manifest.json"), OBSIDIAN_PLUGIN_MANIFEST).map_err(|e| format!("写入 manifest.json 失败：{e}"))?;
  Ok(dir.to_string_lossy().into_owned())
}
