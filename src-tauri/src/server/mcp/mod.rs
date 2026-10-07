//! AI 外接通道（MCP 一期，PRD 08-06「AI 外接通道」）：进程内嵌 Streamable HTTP server——
//! 127.0.0.1 固定端口 17701 优先（被占回退动态）+ 端口文件（全局配置目录 AniKit/mcp-server.port，
//! 每次启动覆写实际端口）；外部 Agent 工具（Kimi Code / Codex / Claude Code 等）配静态 URL 直连。
//!
//! 实现 = 官方 Rust SDK rmcp（≥1.4.0 内置 Host 头校验防 DNS rebinding，CVE-2026-42559）
//! + axum nest_service 挂载其 Tower 服务；协议握手 / 会话 / schema 协商全由 SDK 承担。
//!
//! 一期只读红线：只读工具全部 annotations(read_only_hint = true)；薄壳转发既有命令层 / 服务层；
//! 库未打开统一回「请先打开素材库」工具级错误（isError=true），不静默空表；有副作用的
//! 准只读命令（触发转码 / 后台生成队列类）不进白名单。
//!
//! 二期首批写工具（2026-09-19 定，tools/write.rs）：create_document / write_text_asset /
//! rename_placements 三个，annotations(read_only_hint = false)。**写入闸 = 形态 A**：
//! 全局开关 config.json settings.mcp.writeEnabled（默认关，插件面板 MCP 详情窗开启——
//! 2026-09-20 从偏好设置弹窗挪入，旧落点 settings.ui.mcpWriteEnabled 由 core::config 一次性迁移），
//! 写调用逐次检查（write_gate，core::config 无缓存、低频直读），关态回工具级错误「MCP 写入未开启」；
//! 撤销口径 = create_document / rename_placements 进撤销（复用 commands::operations 抽出的
//! _op 整段，单写点 + 单撤销步），write_text_asset 不进撤销（与 APP 内文本编辑同口径，2026-08-23 定）。
//!
//! 服务总开关（2026-09-20 插件面板 MCP 详情窗配套）：settings.mcp.serverEnabled 缺省开——
//! init 读配置决定是否启动；详情窗经 commands/mcp.rs 随时启停（全局 MCP_HANDLE 存 stop 信号 +
//! 实际端口 + 任务句柄，graceful shutdown；停止即删端口文件，Agent 发现机制同步下线）。
//! 动机 = 入口统一 + 暴露面管理：不用 Agent 的用户可整个关掉，17701 不白听。
//!
//! 二批结构性写工具（2026-09-19 定，tools/structure.rs + tools/import.rs）：create_folder /
//! move_placements / link_placements / move_folders（同日用户补：结构重组留一手；同首批 _op
//! 复用范式，进撤销）+ import_paths（方案一冲突策略：hash 相同跳过回已有 id / 同名不同内容
//! 加 _序号 后缀入库，逐文件报告；逐文件同步直调既有 run_import 管线，不进撤销——APP 导入管线本就不接 UndoManager）。
//!
//! 三批回收站删除（2026-09-20 定，tools/trash.rs）：trash_placements 一个——允许 AI 丢库内
//! 回收站（recycle.json 快照、磁盘不动，同 _op 范式进撤销）；**彻底删除永不经 MCP**
//!（recycle.rs 彻底清除 / purge.rs / 系统回收站适配器一律不注册工具）。
//!
//! 提示词写工具（2026-09-26 提示词轮，tools/prompt.rs）：assign_prompt / clear_prompt 两个——
//! 目标 assetId / folderId 二选一，同 _op 范式进撤销（commands::prompt 整段复用）。
//!
//! 四批分组写工具（2026-10-08 用户定，tools/groups.rs）：set_folder_groups 一个——场次规则组的
//! Agent 侧入口（全量草稿式提交，新组 id 缺省后端 new_group_id 生成），同 _op 范式进撤销
//!（commands::operations 整段复用）；assign_group 不设工具（PRD 2026-09-19 结论维持，
//! 自由组成员经 set_folder_groups members 全量管理）。
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
        + Self::archives_router()
        + Self::write_router()
        + Self::structure_router()
        + Self::import_router()
        + Self::trash_router()
        + Self::prompt_router()
        + Self::groups_router(),
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

  /// 写工具骨架（二期）：同 with_lib 口径，取可变库引用。只服务不进撤销的写
  ///（write_text_asset 等）——进撤销的写工具走 commands::operations 的 _op 整段（锁 + undo 一体），不经本骨架
  fn with_lib_mut<T: serde::Serialize>(
    &self,
    f: impl FnOnce(&mut Library) -> Result<T, String>,
  ) -> Result<CallToolResult, ErrorData> {
    let result = {
      let state = self.app.state::<AppState>();
      let mut guard = state.library.lock().map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
      match guard.as_mut() {
        Some(lib) => f(lib),
        None => return Ok(CallToolResult::error(vec![ContentBlock::text("请先打开素材库")])),
      }
    };
    match result {
      Ok(v) => json_result(&v),
      Err(msg) => Ok(CallToolResult::error(vec![ContentBlock::text(msg)])),
    }
  }

  /// 写入成功后的前端热刷新广播（2026-09-19 用户实测「agent 新建文件夹要重启才显示」补）：
  /// 库数据文件无监听（watcher 只管收件箱），命令层写由前端自己刷新，MCP 绕开前端必须广播；
  /// 前端 content store initMcpListener 监听 mcp-library-changed，去抖 300ms 全量对齐
  ///（Agent 批量写连发合并为一拍；同 cep-imported 的 refresh 口径，但不带 CEP 的成功提示音）
  pub fn emit_library_changed(&self, what: &str) {
    use tauri::Emitter;
    let _ = self.app.emit("mcp-library-changed", what);
  }
}

/// 写入闸（PRD 08-06 形态 A，2026-09-19 用户定；2026-09-20 挪插件面板 MCP 详情窗）：全局开关 =
/// config.json settings.mcp 子树 writeEnabled（默认关）。MCP 写调用逐次检查、直读配置
///（core::config 无缓存，低频调用直读可接受）；关态 / 读取失败一律回工具级错误（同「请先打开素材库」口径）
fn write_gate() -> Option<CallToolResult> {
  let enabled = crate::core::config::mcp_settings().map(|s| s.write_enabled).unwrap_or(false);
  if enabled {
    None
  } else {
    Some(CallToolResult::error(vec![ContentBlock::text("MCP 写入未开启（插件面板 MCP 详情窗里可开启）")]))
  }
}

/// 成功回包：仅 content 文本（紧凑 JSON）。不带 structuredContent——同一份数据双带会被
/// 不查重的客户端（Codex 等）双倍计入上下文（2026-09-19 用户实测额度消耗后定的省 token 口径）；
/// 未声明 outputSchema 时 structuredContent 本就可选，文本 JSON Agent 直接可解析
fn json_result<T: serde::Serialize>(v: &T) -> Result<CallToolResult, ErrorData> {
  let text = serde_json::to_string(v).map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
  Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
}

/// ServerHandler：工具面由 tool_router 提供；server 名 / 版本 / 使用说明经宏参数生成。
/// instructions = 给 Agent 的README（每次连接必读）：典型流程 + 纪律红线，与
/// docs/05_接口文档/MCP使用指南.md 同口径（改其一必须同步另一处）
#[tool_handler(router = self.tool_router, name = "anikit", instructions = "AniKit 素材库外接通道（MCP，一期只读 8 + 写工具 12）。典型流程：get_current_library 确认库已打开 → search_assets 搜索 / list_folder_tree + list_placements 浏览 → get_asset_detail 看标签·描述·注释·提示词 → get_asset_origin_path 拿磁盘绝对路径后直读文件看图（webp 缩略图优先，不经协议传图）→ read_text_asset 读提示词·剧本等文本资产。写入（十二个）：create_document 新建文档入库 / write_text_asset 覆写文本资产内容 / rename_placements 批量改名（origin 缺省自动定位——该文件夹内仅此资产一条记录即命中，多条记录须照 list_placements 回包 origin 指定；命中 origin 记录连磁盘名同步改） / create_folder 新建文件夹 / move_placements 资产移入文件夹 / move_folders 移动文件夹重设父级 / link_placements 链接汇总（资产留原地、目标文件夹建链接）/ import_paths 导入磁盘文件入库 / trash_placements 丢回收站（Placement 进库内回收站，磁盘文件不动）/ assign_prompt 指定提示词（提示词 = 引用库内 md/txt 文本资产 + 内容快照；目标 assetId / folderId 二选一）/ clear_prompt 断开提示词（快照随解绑清除）/ set_folder_groups 设置文件夹分组（全量草稿式提交：groups = 该文件夹分组终态数组，空数组 = 取消分组，漏交 = 删组；改既有组照 list_folder_tree 回包抄 id，新组 id 缺省后端生成随回包返回；规则组 = rules 非空按规则自动归组——场次「提示词 / Pn」组用它配，自由组 = 空 rules + members assetId 手动归组）——默认关闸，需在插件面板 MCP 详情窗开启「允许写入」，关闸调用回「MCP 写入未开启」。撤销口径：write_text_asset 与 import_paths 不进撤销（覆写前先 read_text_asset 备份；导入冲突不阻断——hash 相同跳过并回已有资产 id、同名不同内容自动加 _序号 后缀，逐文件报告），其余十个进撤销（APP 内 Ctrl+Z 可撤）。纪律：资产磁盘名随重命名变化，路径每次现取、禁止缓存复用；库未打开时工具返回「请先打开素材库」；彻底删除永不经 MCP（丢回收站经 trash_placements，可逆，回收站清空 / 彻底删除须 APP 内人工）。省 token：先 search_assets 缩小范围再按需取详情，避免无 folderId 的全库 list_placements；看图读 thumbPath 缩略图而非源图。")]
impl ServerHandler for AnikitMcp {}

/// lib.rs setup 拉起（tauri 异步运行时，同 plugin_ws）：读服务总开关（settings.mcp.serverEnabled，
/// 缺省开）决定本轮是否启动；关 = 不监听（插件面板 MCP 详情窗可随时拉起）
pub fn init(app: AppHandle) {
  let enabled = crate::core::config::mcp_settings().map(|s| s.server_enabled).unwrap_or(true);
  if !enabled {
    log::info!("MCP 服务总开关关，本轮不启动（插件面板 MCP 详情窗可开启）");
    return;
  }
  tauri::async_runtime::spawn(async move {
    if let Err(e) = start(app).await {
      log::error!("MCP 服务启动失败：{e}");
    }
  });
}

/// 运行句柄（服务总开关）：stop 信号 + 实际端口 + serve 任务；全局单例（OnceLock 惰性初始化）。
/// task 用 tokio::spawn（同一 tauri tokio 运行时）——tauri::async_runtime::JoinHandle 无 is_finished
struct McpHandle {
  stop: tokio::sync::oneshot::Sender<()>,
  port: u16,
  task: tokio::task::JoinHandle<()>,
}

static MCP_HANDLE: std::sync::OnceLock<std::sync::Mutex<Option<McpHandle>>> = std::sync::OnceLock::new();

fn handle_slot() -> &'static std::sync::Mutex<Option<McpHandle>> {
  MCP_HANDLE.get_or_init(|| std::sync::Mutex::new(None))
}

/// 启动（幂等：运行中直接回实际端口）。固定端口 17701 优先（2026-09-19 用户定：Agent 工具
/// 只读静态配置 config.toml，动态端口每重启失效不匹配——与 plugin_ws 的场景差在客户端不会读
/// 端口文件）；被占（双开 / 他占）回退动态，实际端口恒写端口文件
pub async fn start(app: AppHandle) -> Result<u16, String> {
  {
    let mut guard = handle_slot().lock().map_err(|e| e.to_string())?;
    if let Some(h) = guard.as_ref() {
      if !h.task.is_finished() {
        return Ok(h.port);
      }
      guard.take(); // 尸体清理：serve 异常退出后句柄残留，走重起路径
    }
  }
  const FIXED_PORT: u16 = 17701;
  let listener = match TcpListener::bind(("127.0.0.1", FIXED_PORT)).await {
    Ok(l) => l,
    Err(e) => {
      log::warn!("MCP 固定端口 {FIXED_PORT} 被占（{e}），回退动态端口");
      TcpListener::bind("127.0.0.1:0").await.map_err(|e| e.to_string())?
    }
  };
  let port = listener.local_addr().map_err(|e| e.to_string())?.port();
  let service = StreamableHttpService::new(
    move || Ok::<_, std::io::Error>(AnikitMcp::new(app.clone())),
    LocalSessionManager::default().into(),
    StreamableHttpServerConfig::default(),
  );
  let router = axum::Router::new().nest_service("/mcp", service);
  let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
  let task = tokio::spawn(async move {
    let server = axum::serve(listener, router).with_graceful_shutdown(async move {
      let _ = stop_rx.await;
    });
    if let Err(e) = server.await {
      log::error!("MCP 服务异常退出：{e}");
    }
  });
  let mut guard = handle_slot().lock().map_err(|e| e.to_string())?;
  if let Some(h) = guard.as_ref() {
    // await 期间被另一路 start 抢先（init / 详情窗并发）：弃新保旧
    task.abort();
    return Ok(h.port);
  }
  *guard = Some(McpHandle { stop: stop_tx, port, task });
  write_port_file(port);
  log::info!("MCP 服务就绪：http://127.0.0.1:{port}/mcp");
  Ok(port)
}

/// 停止（幂等：未运行直接 Ok）：graceful shutdown 收束后删端口文件（Agent 发现机制同步下线——
/// 残留端口文件会误导 Agent 连死端口）
pub async fn stop() -> Result<(), String> {
  let handle = handle_slot().lock().map_err(|e| e.to_string())?.take();
  let Some(h) = handle else { return Ok(()); };
  let _ = h.stop.send(());
  let _ = h.task.await;
  remove_port_file();
  log::info!("MCP 服务已停止");
  Ok(())
}

/// 运行状态查询（commands/mcp get_mcp_status 用）：serve 意外退出（句柄在但任务已完）按未运行报
pub fn status() -> (bool, Option<u16>) {
  let Ok(guard) = handle_slot().lock() else { return (false, None) };
  match guard.as_ref() {
    Some(h) if !h.task.is_finished() => (true, Some(h.port)),
    _ => (false, None),
  }
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

/// 停止时删端口文件（见 stop）
fn remove_port_file() {
  let Some(dir) = dirs::config_dir() else {
    return;
  };
  let _ = std::fs::remove_file(dir.join("AniKit").join("mcp-server.port"));
}
