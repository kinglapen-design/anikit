//! 命令 API：插件 WS 通道服务开关（PRD 07-01；2026-09-25，MCP 同款总开关）——
//! 不用动画软件插件的用户可关，127.0.0.1 不白听。配置落 settings.pluginServer 子树
//!（core::config 单写点；独立子树原因同 MCP——save_ui_settings 整体替换语义会抹掉）。
//! server/plugin_ws 生命周期三函数 #[cfg(not(test))]（踩坑 014）——启停调用处同 cfg 分支，
//! test 构建按未运行报。

use crate::core::config;

/// 插件 WS 服务状态（get 与 set 回包同构：前端动作后直接以回包校正）
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginServerStatus {
  /// 服务实际在跑（句柄活且任务未退出）
  pub running: bool,
  /// 实际监听端口（running 时有值；动态端口）
  pub port: Option<u16>,
  /// 配置：服务总开关（settings.pluginServer.serverEnabled）
  pub server_enabled: bool,
}

fn current_status() -> PluginServerStatus {
  #[cfg(not(test))]
  let (running, port) = crate::server::plugin_server_status();
  #[cfg(test)]
  let (running, port) = (false, None::<u16>);
  PluginServerStatus {
    running,
    port,
    server_enabled: config::plugin_server_settings().map(|s| s.server_enabled).unwrap_or(true),
  }
}

#[tauri::command]
pub fn get_plugin_server_status() -> PluginServerStatus {
  current_status()
}

/// 服务总开关：先启停后持久化（启动失败不记开态）；返回最新状态
#[tauri::command]
pub async fn set_plugin_server_enabled(app: tauri::AppHandle, enabled: bool) -> Result<PluginServerStatus, String> {
  #[cfg(not(test))]
  {
    if enabled {
      crate::server::start_plugin_server(app).await?;
    } else {
      crate::server::stop_plugin_server().await?;
    }
  }
  #[cfg(test)]
  let _ = (app, enabled);
  let mut s = config::plugin_server_settings().map_err(|e| e.to_string())?;
  s.server_enabled = enabled;
  config::save_plugin_server_settings(s).map_err(|e| e.to_string())?;
  Ok(current_status())
}
