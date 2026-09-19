//! AI 外接通道（MCP 一期，PRD 08-06「AI 外接通道」）：进程内嵌 Streamable HTTP server——
//! 127.0.0.1 动态端口 + 端口文件（全局配置目录 AniKit/mcp-server.port，每次启动覆写，
//! 同 plugin_ws 发现机制）；外部 Agent 工具（Kimi Code / Claude Code 等）配 URL 直连。
//!
//! 实现 = 官方 Rust SDK rmcp（≥1.4.0 内置 Host 头校验防 DNS rebinding，CVE-2026-42559）
//! + axum nest_service 挂载其 Tower 服务；协议握手 / 会话 / schema 协商全由 SDK 承担。
//!
//! 一期只读红线：全部工具 annotations(read_only_hint = true)；薄壳转发既有命令层 / 服务层；
//! 库未打开统一回「请先打开素材库」工具级错误（isError=true），不静默空表；有副作用的
//! 准只读命令（触发转码 / 后台生成队列类）不进白名单。
//!
//! 工具注册表模式（PRD 定，范式 = 前端 barSpecs）：一类一文件 + 下方一行注册；
//! 加工具 = tools/ 加一个文件 + new() 注册处加一行。本注册表未来复用为插件 API 注册表。
//!
//! 整模块 #[cfg(not(test))] 才编译（测试构建不起 server，踩坑 014）。

mod tools;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::{ErrorData, ServerHandler, tool_handler};
use tauri::{AppHandle, Manager};
use tokio::net::TcpListener;

use rmcp::transport::streamable_http_server::{
  StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};

use crate::commands::library::AppState;
use crate::core::library::Library;

/// MCP server 实例：持 AppHandle（工具内取 AppState）+ 合并后的工具路由表
#[derive(Clone)]
pub struct AnikitMcp {
  app: AppHandle,
  tool_router: ToolRouter<Self>,
}

impl AnikitMcp {
  fn new(app: AppHandle) -> Self {
    Self {
      app,
      // 工具注册表：一类一文件 + 此处一行（router fn = 各文件 #[tool_router] 生成的关联函数）
      tool_router: Self::library_router()
        + Self::search_router()
        + Self::folders_router()
        + Self::assets_router()
        + Self::text_router()
        + Self::archives_router(),
    }
  }

  /// 工具薄壳公共骨架：取库锁 → 未开库回「请先打开素材库」工具级错误 → 业务闭包 → JSON 回包。
  /// 域错误（String）一律落 CallToolResult::error（工具级 isError，Agent 可见原因），
  /// 协议级 McpError 只留给序列化失败等内部异常
  fn with_lib<T: serde::Serialize>(&self, f: impl FnOnce(&Library) -> Result<T, String>) -> Result<CallToolResult, ErrorData> {
    let result = {
      let state = self.app.state::<AppState>();
      let guard = state.library.lock().map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
      match guard.as_ref() {
        Some(lib) => f(lib),
        None => return Ok(CallToolResult::error(vec![ContentBlock::text("请先打开素材库")])),
      }
    };
    match result {
      Ok(v) => json_result(&v),
      Err(msg) => Ok(CallToolResult::error(vec![ContentBlock::text(msg)])),
    }
  }
}

/// 成功回包：content 文本（pretty JSON，兼容只读文本的客户端）+ structuredContent 双带
fn json_result<T: serde::Serialize>(v: &T) -> Result<CallToolResult, ErrorData> {
  let text = serde_json::to_string_pretty(v).map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
  let mut result = CallToolResult::success(vec![ContentBlock::text(text)]);
  result.structured_content = Some(serde_json::to_value(v).map_err(|e| ErrorData::internal_error(e.to_string(), None))?);
  Ok(result)
}

/// ServerHandler：工具面由 tool_router 提供；server 名 / 版本 / 使用说明经宏参数生成
#[tool_handler(router = self.tool_router, name = "anikit", instructions = "AniKit 素材库只读通道（一期）：搜索 / 浏览文件夹与资产 / 读资产详情与文本内容 / 取源文件与缩略图磁盘路径（看图直读 webp 缩略图，不经协议传图）。全部工具只读；素材库未打开时工具返回「请先打开素材库」。")]
impl ServerHandler for AnikitMcp {}

/// lib.rs setup 拉起（tauri 异步运行时，同 plugin_ws）
pub fn init(app: AppHandle) {
  tauri::async_runtime::spawn(async move {
    if let Err(e) = serve(app).await {
      log::error!("MCP 服务异常退出：{e}");
    }
  });
}

async fn serve(app: AppHandle) -> Result<(), String> {
  let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|e| e.to_string())?;
  let port = listener.local_addr().map_err(|e| e.to_string())?.port();
  write_port_file(port);
  log::info!("MCP 服务就绪：http://127.0.0.1:{port}/mcp");
  let service = StreamableHttpService::new(
    move || Ok::<_, std::io::Error>(AnikitMcp::new(app.clone())),
    LocalSessionManager::default().into(),
    StreamableHttpServerConfig::default(),
  );
  let router = axum::Router::new().nest_service("/mcp", service);
  axum::serve(listener, router).await.map_err(|e| e.to_string())
}

/// 端口文件（Agent 工具发现机制）：每次启动覆写；进程退出残留无害（连不上即未启动）
fn write_port_file(port: u16) {
  let Some(dir) = dirs::config_dir() else {
    return;
  };
  let dir = dir.join("AniKit");
  let _ = std::fs::create_dir_all(&dir);
  if let Err(e) = std::fs::write(dir.join("mcp-server.port"), port.to_string()) {
    log::warn!("MCP 端口文件写入失败：{e}");
  }
}
