//! 命令 API：AI 外接通道（MCP）状态与开关（PRD 08-06；2026-09-20 插件面板 MCP 详情窗配套）——
//! 服务总开关（启停 127.0.0.1:17701）+ 写入闸（只读/写入切换），配置落 settings.mcp 子树
//!（core::config 单写点，不经前端 ui store 管道：save_ui_settings 整体替换语义会抹掉非快照键，
//! 且子窗口 store 快照与主窗口双写竞态）。
//! server/mcp 整模块 #[cfg(not(test))]（踩坑 014）——启停调用处同 cfg 分支，test 构建按未运行报。

use crate::core::config;

/// MCP 面板状态（get_mcp_status 与两 set 命令回包同构：前端动作后直接以回包校正）
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpStatus {
  /// server 实际在跑（句柄活且任务未退出）
  pub running: bool,
  /// 实际监听端口（running 时有值；17701 被占回退动态端口）
  pub port: Option<u16>,
  /// 接入地址（running 时有值）：http://127.0.0.1:{port}/mcp
  pub url: Option<String>,
  /// 配置：服务总开关（settings.mcp.serverEnabled）
  pub server_enabled: bool,
  /// 配置：写入闸（settings.mcp.writeEnabled）
  pub write_enabled: bool,
}

fn current_status() -> McpStatus {
  #[cfg(not(test))]
  let (running, port) = crate::server::mcp_status();
  #[cfg(test)]
  let (running, port) = (false, None::<u16>);
  let settings = config::mcp_settings().unwrap_or_default();
  McpStatus {
    running,
    port,
    url: if running { port.map(|p| format!("http://127.0.0.1:{p}/mcp")) } else { None },
    server_enabled: settings.server_enabled,
    write_enabled: settings.write_enabled,
  }
}

#[tauri::command]
pub fn get_mcp_status() -> McpStatus {
  current_status()
}

/// 服务总开关：先启停后持久化（启动失败不记开态）；返回最新状态
#[tauri::command]
pub async fn set_mcp_server_enabled(app: tauri::AppHandle, enabled: bool) -> Result<McpStatus, String> {
  #[cfg(not(test))]
  {
    if enabled {
      crate::server::start_mcp(app).await?;
    } else {
      crate::server::stop_mcp().await?;
    }
  }
  #[cfg(test)]
  let _ = (app, enabled);
  let mut settings = config::mcp_settings().map_err(|e| e.to_string())?;
  settings.server_enabled = enabled;
  config::save_mcp_settings(settings).map_err(|e| e.to_string())?;
  Ok(current_status())
}

/// 写入闸（只读/写入切换）：持久化即生效（server/mcp write_gate 逐次直读，无缓存）
#[tauri::command]
pub fn set_mcp_write_enabled(enabled: bool) -> Result<McpStatus, String> {
  let mut settings = config::mcp_settings().map_err(|e| e.to_string())?;
  settings.write_enabled = enabled;
  config::save_mcp_settings(settings).map_err(|e| e.to_string())?;
  Ok(current_status())
}
