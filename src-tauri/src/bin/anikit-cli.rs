//! anikit-cli：CLI 外接入口（2026-11-02 定案，PRD 08-06「CLI 外接入口」，1.0 收尾第 8 项）。
//! 面向终端 Agent 与脚本自动化的瘦客户端：POST 127.0.0.1:{port}/mcp（MCP JSON-RPC
//! initialize → initialized 通知 → tools/call），复用运行中 AniKit 的全部工具面、
//! 写入闸、撤销与热刷新——后端零改动零新端点，本 bin 只做参数映射与协议解析。
//! 端口发现 = config 同目录 mcp-server.port（同 anikit-paste 口径），缺省回退 17701，--port 覆盖。
//! 首版 = 只读 13 工具全量子命令 + tools 列表；写命令子命令后续按真实工作流补批。
//! 输出 = 紧凑 JSON（--pretty 缩进）；退出码 0 成功 / 1 工具级错误（stdout {"error":...}）/
//! 2 连接·协议·用法错误（stderr 中文）。
//! 注意：不得加 windows_subsystem 属性——脚本与 Agent 要读 stdout（同 anikit-paste）。

use std::process::ExitCode;
use std::time::Duration;

use serde_json::{json, Value};

const DEFAULT_PORT: u16 = 17701;
const PROTOCOL_VERSION: &str = "2025-03-26";

const USAGE: &str = r#"anikit-cli —— AniKit 素材库 CLI 外接入口（MCP 瘦客户端，需 AniKit 运行中）

用法：anikit-cli <子命令> [参数] [--pretty] [--port N]

子命令（只读 13 工具 + 工具列表）：
  status                          当前储存空间与素材库状态
  search <关键词> [--folder ID] [--no-recursive]
                                  分词搜索素材库
  tree                            文件夹树（区块 + 节点 + 分组定义）
  placements [--folder ID]        文件夹内资产清单（缺省全库，量大慎用）
  detail <assetId>                资产详情（标签 / 描述 / 注释 / 链接 / 主色等）
  path <assetId>                  资产源文件与缩略图磁盘绝对路径
  read <assetId>                  读文本资产内容（txt / md）
  archives                        档案列表
  archive <folderId>              单个档案详情
  links                           全库链接 Placement 清单
  semantic <查询> [--folder ID] [--limit N] [--no-recursive]
                                  中文语义检索（只查既有索引，不触发分析）
  task <taskId>                   查询一个语义分析任务
  tasks                           列出当前库语义分析任务
  tools                           列出 MCP 工具（名称 + 描述 + 只读标记）

选项：
  --pretty                        缩进格式化输出（默认紧凑 JSON）
  --port N                        覆盖服务端口（默认读 mcp-server.port，回退 17701）
  -h, --help                      显示本帮助

id 参数可直接粘贴 anikit://item/{id} 或 http://127.0.0.1:{port}/(open/)item/{id} 链接。
退出码：0 成功；1 工具级错误（stdout 输出 {"error":"..."}）；2 连接 / 协议 / 用法错误（stderr）。
示例：
  anikit-cli status
  anikit-cli search 杰森 --folder F20260901-XXXXXX --pretty
  anikit-cli detail AN-20260901-XXXXXX
"#;

/// 失败三态：用法 / 连接协议（stderr + 2）；工具级（stdout {"error":...} + 1）
enum Fail {
  Usage(String),
  Transport(String),
  Tool(String),
}

fn main() -> ExitCode {
  let args: Vec<String> = std::env::args().skip(1).collect();
  match run(&args) {
    Ok(()) => ExitCode::SUCCESS,
    Err(Fail::Tool(msg)) => {
      println!("{}", serde_json::to_string(&json!({ "error": msg })).unwrap_or_else(|_| "{\"error\":\"工具错误\"}".into()));
      ExitCode::from(1)
    }
    Err(Fail::Usage(msg)) => {
      eprintln!("anikit-cli：{msg}\n（anikit-cli --help 查看用法）");
      ExitCode::from(2)
    }
    Err(Fail::Transport(msg)) => {
      eprintln!("anikit-cli：{msg}");
      ExitCode::from(2)
    }
  }
}

fn run(args: &[String]) -> Result<(), Fail> {
  let parsed = parse_args(args).map_err(Fail::Usage)?;
  let Action::Run { pretty, port, call } = parsed else {
    print!("{USAGE}");
    return Ok(());
  };
  let mut client = McpClient::connect(port.unwrap_or_else(read_port))?;
  client.initialize()?;
  match call {
    Call::Tool { tool, args } => {
      let result = client.rpc("tools/call", json!({ "name": tool, "arguments": args }))?;
      let text = content_text(&result);
      if result.get("isError").and_then(Value::as_bool).unwrap_or(false) {
        return Err(Fail::Tool(if text.is_empty() { "工具返回错误（无详情）".to_string() } else { text }));
      }
      if text.is_empty() {
        print_json(&result, pretty);
      } else {
        print_json_text(&text, pretty);
      }
    }
    Call::ListTools => {
      let result = client.rpc("tools/list", json!({}))?;
      print_json(&slim_tools(&result), pretty);
    }
  }
  Ok(())
}

/// 解析后的动作：Help（含 -h / --help / help / 无参数时由调用方决定出口）或一次调用
enum Action {
  Help,
  Run { pretty: bool, port: Option<u16>, call: Call },
}

enum Call {
  Tool { tool: &'static str, args: Value },
  ListTools,
}

/// 参数解析：全局 flag（--pretty / --port / -h）可出现在任意位置；子命令 = 第一个非 flag 参数，
/// 其后位置参数按子命令定义消费。id / 链接类值原样透传（服务端 anikit_ref 归一化）
fn parse_args(args: &[String]) -> Result<Action, String> {
  let mut pretty = false;
  let mut port: Option<u16> = None;
  let mut folder: Option<String> = None;
  let mut limit: Option<u32> = None;
  let mut recursive = true;
  let mut positional: Vec<String> = Vec::new();
  let mut i = 0;
  while i < args.len() {
    let arg = args[i].as_str();
    // 取 flag 值的公共步进：--port 17702 与 --port=17702 同吃
    fn value_of(args: &[String], i: usize, arg: &str, name: &str) -> Result<(String, usize), String> {
      if let Some(v) = arg.strip_prefix(&format!("{name}=")) {
        return Ok((v.to_string(), i + 1));
      }
      let Some(v) = args.get(i + 1) else { return Err(format!("{name} 缺值")) };
      Ok((v.clone(), i + 2))
    }
    match arg {
      "--pretty" => {
        pretty = true;
        i += 1;
      }
      "--no-recursive" => {
        recursive = false;
        i += 1;
      }
      "-h" | "--help" => return Ok(Action::Help),
      _ if arg == "--port" || arg.starts_with("--port=") => {
        let (v, next) = value_of(args, i, arg, "--port")?;
        port = Some(v.parse().map_err(|_| format!("端口无效：{v}"))?);
        i = next;
      }
      _ if arg == "--folder" || arg.starts_with("--folder=") => {
        let (v, next) = value_of(args, i, arg, "--folder")?;
        folder = Some(v);
        i = next;
      }
      _ if arg == "--limit" || arg.starts_with("--limit=") => {
        let (v, next) = value_of(args, i, arg, "--limit")?;
        limit = Some(v.parse().map_err(|_| format!("limit 无效：{v}"))?);
        i = next;
      }
      _ if arg.starts_with('-') && arg.len() > 1 => return Err(format!("未知选项：{arg}")),
      _ => {
        positional.push(args[i].clone());
        i += 1;
      }
    }
  }
  let Some(cmd) = positional.first().map(String::as_str) else {
    return Ok(Action::Help);
  };
  let rest = &positional[1..];
  // 位置参数 arity 校验 + 工具参数装配
  fn need<'a>(rest: &'a [String], idx: usize, what: &str, cmd: &str) -> Result<&'a str, String> {
    rest.get(idx).map(String::as_str).ok_or_else(|| format!("{cmd} 缺参数：{what}"))
  }
  fn arity(rest: &[String], max: usize, cmd: &str) -> Result<(), String> {
    if rest.len() > max {
      Err(format!("{cmd} 位置参数过多（最多 {max} 个）"))
    } else {
      Ok(())
    }
  }
  let folder_arg = |v: &Option<String>| v.as_ref().map(|f| ("folderId".to_string(), json!(f)));
  let call = match cmd {
    "help" => return Ok(Action::Help),
    "status" => {
      arity(rest, 0, cmd)?;
      Call::Tool { tool: "get_current_library", args: json!({}) }
    }
    "search" => {
      arity(rest, 1, cmd)?;
      let query = need(rest, 0, "关键词", cmd)?;
      let mut args = json!({ "query": query, "recursive": recursive });
      if let Some((k, v)) = folder_arg(&folder) {
        args[k] = v;
      }
      Call::Tool { tool: "search_assets", args }
    }
    "tree" => {
      arity(rest, 0, cmd)?;
      Call::Tool { tool: "list_folder_tree", args: json!({}) }
    }
    "placements" => {
      arity(rest, 0, cmd)?;
      let mut args = json!({});
      if let Some((k, v)) = folder_arg(&folder) {
        args[k] = v;
      }
      Call::Tool { tool: "list_placements", args }
    }
    "detail" => {
      arity(rest, 1, cmd)?;
      Call::Tool { tool: "get_asset_detail", args: json!({ "assetId": need(rest, 0, "assetId", cmd)? }) }
    }
    "path" => {
      arity(rest, 1, cmd)?;
      Call::Tool { tool: "get_asset_origin_path", args: json!({ "assetId": need(rest, 0, "assetId", cmd)? }) }
    }
    "read" => {
      arity(rest, 1, cmd)?;
      Call::Tool { tool: "read_text_asset", args: json!({ "assetId": need(rest, 0, "assetId", cmd)? }) }
    }
    "archives" => {
      arity(rest, 0, cmd)?;
      Call::Tool { tool: "list_archives", args: json!({}) }
    }
    "archive" => {
      arity(rest, 1, cmd)?;
      Call::Tool { tool: "get_archive_data", args: json!({ "folderId": need(rest, 0, "folderId", cmd)? }) }
    }
    "links" => {
      arity(rest, 0, cmd)?;
      Call::Tool { tool: "list_links", args: json!({}) }
    }
    "semantic" => {
      arity(rest, 1, cmd)?;
      let query = need(rest, 0, "查询内容", cmd)?;
      let mut args = json!({ "query": query, "recursive": recursive });
      if let Some((k, v)) = folder_arg(&folder) {
        args[k] = v;
      }
      if let Some(n) = limit {
        args["limit"] = json!(n);
      }
      Call::Tool { tool: "semantic_search", args }
    }
    "task" => {
      arity(rest, 1, cmd)?;
      Call::Tool { tool: "get_semantic_analysis_task", args: json!({ "taskId": need(rest, 0, "taskId", cmd)? }) }
    }
    "tasks" => {
      arity(rest, 0, cmd)?;
      Call::Tool { tool: "list_semantic_analysis_tasks", args: json!({}) }
    }
    "tools" => {
      arity(rest, 0, cmd)?;
      Call::ListTools
    }
    _ => return Err(format!("未知子命令：{cmd}")),
  };
  Ok(Action::Run { pretty, port, call })
}

/// 媒体服务实际端口：config 同目录 mcp-server.port（内容 = 纯数字，启动覆写，停止即删）；
/// 缺文件 / 解析失败回退固定端口 17701（同 anikit-paste 口径）
fn read_port() -> u16 {
  let Ok(cfg_path) = app_lib::core::config::config_path() else { return DEFAULT_PORT };
  let Ok(text) = std::fs::read_to_string(cfg_path.with_file_name("mcp-server.port")) else { return DEFAULT_PORT };
  text.trim().parse().unwrap_or(DEFAULT_PORT)
}

/// MCP Streamable HTTP 瘦客户端：每次运行一次握手（initialize + initialized 通知），
/// 之后逐请求 JSON-RPC。响应两种形态都接：application/json 直读 / text/event-stream 按 event 取 data
struct McpClient {
  http: reqwest::blocking::Client,
  url: String,
  session: Option<String>,
  next_id: u64,
}

impl McpClient {
  fn connect(port: u16) -> Result<Self, Fail> {
    let http = reqwest::blocking::Client::builder()
      .connect_timeout(Duration::from_secs(3))
      .timeout(Duration::from_secs(60))
      .build()
      .map_err(|e| Fail::Transport(format!("HTTP 客户端初始化失败：{e}")))?;
    Ok(Self { http, url: format!("http://127.0.0.1:{port}/mcp"), session: None, next_id: 0 })
  }

  /// POST 一次 JSON-RPC：通知（id=None）不等响应体；请求返回匹配 id 的响应对象。
  /// 同时回传响应头里的 mcp-session-id（initialize 时建立会话）
  fn post(&mut self, method: &str, params: Value, with_id: bool) -> Result<(Option<String>, Value), Fail> {
    let id = if with_id {
      self.next_id += 1;
      Some(self.next_id)
    } else {
      None
    };
    let mut body = json!({ "jsonrpc": "2.0", "method": method, "params": params });
    if let Some(id) = id {
      body["id"] = json!(id);
    }
    let mut req = self
      .http
      .post(&self.url)
      .header("Content-Type", "application/json")
      .header("Accept", "application/json, text/event-stream")
      .json(&body);
    if let Some(session) = &self.session {
      req = req.header("mcp-session-id", session);
    }
    let resp = req.send().map_err(|e| {
      Fail::Transport(format!("连接 AniKit 失败（{}）：{e}\n请确认 AniKit 正在运行，且插件面板 MCP 详情窗的服务总开关已开启", self.url))
    })?;
    let status = resp.status();
    let session = resp
      .headers()
      .get("mcp-session-id")
      .and_then(|v| v.to_str().ok())
      .map(str::to_string);
    let is_sse = resp
      .headers()
      .get(reqwest::header::CONTENT_TYPE)
      .and_then(|v| v.to_str().ok())
      .map(|v| v.contains("text/event-stream"))
      .unwrap_or(false);
    let text = resp.text().map_err(|e| Fail::Transport(format!("响应读取失败：{e}")))?;
    if !status.is_success() {
      return Err(Fail::Transport(format!("服务返回 HTTP {status}：{}", truncate(&text, 200))));
    }
    if !with_id || text.trim().is_empty() {
      return Ok((session, Value::Null));
    }
    // JSON 直读或 SSE event 扫描；取含匹配 id 的 JSON-RPC 响应（通知 / 其他 id 跳过）
    let candidates: Vec<Value> = if is_sse {
      sse_events(&text).iter().filter_map(|data| serde_json::from_str(data).ok()).collect()
    } else {
      serde_json::from_str(&text).map(|v| vec![v]).map_err(|e| Fail::Transport(format!("响应 JSON 解析失败：{e}；原文 {}", truncate(&text, 200))))?
    };
    for v in candidates {
      if v.get("id").and_then(Value::as_u64) == id {
        if let Some(err) = v.get("error") {
          return Err(Fail::Transport(format!("JSON-RPC 错误：{}", compact(err))));
        }
        return Ok((session, v.get("result").cloned().unwrap_or(Value::Null)));
      }
    }
    Err(Fail::Transport(format!("响应中找不到请求 id 的结果：{}", truncate(&text, 200))))
  }

  fn initialize(&mut self) -> Result<(), Fail> {
    let (session, _) = self.post(
      "initialize",
      json!({
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": {},
        "clientInfo": { "name": "anikit-cli", "version": "0.1.0" }
      }),
      true,
    )?;
    self.session = session;
    self.post("notifications/initialized", json!({}), false)?;
    Ok(())
  }

  fn rpc(&mut self, method: &str, params: Value) -> Result<Value, Fail> {
    let (_, result) = self.post(method, params, true)?;
    Ok(result)
  }
}

/// SSE 事件切分：空行为 event 界；event 内多行 data: 以 \n 拼接（SSE 规范）；非 data 行忽略
fn sse_events(body: &str) -> Vec<String> {
  let mut events = Vec::new();
  let mut data_lines: Vec<&str> = Vec::new();
  for line in body.lines() {
    if line.trim().is_empty() {
      if !data_lines.is_empty() {
        events.push(data_lines.join("\n"));
        data_lines.clear();
      }
      continue;
    }
    if let Some(data) = line.strip_prefix("data:") {
      data_lines.push(data.strip_prefix(' ').unwrap_or(data));
    }
  }
  if !data_lines.is_empty() {
    events.push(data_lines.join("\n"));
  }
  events
}

/// tools/call 回包 content 里 type=text 的文本拼接（本项目 json_result 恒单条紧凑 JSON 文本）
fn content_text(result: &Value) -> String {
  let Some(items) = result.get("content").and_then(Value::as_array) else { return String::new() };
  items
    .iter()
    .filter(|c| c.get("type").and_then(Value::as_str) == Some("text"))
    .filter_map(|c| c.get("text").and_then(Value::as_str))
    .collect::<Vec<_>>()
    .join("\n")
}

/// tools/list 瘦身：全量 inputSchema 输出太大，Agent 场景只留名称 + 描述 + 只读标记
fn slim_tools(result: &Value) -> Value {
  let tools = result.get("tools").and_then(Value::as_array).cloned().unwrap_or_default();
  let slim: Vec<Value> = tools
    .iter()
    .map(|t| {
      json!({
        "name": t.get("name").cloned().unwrap_or(Value::Null),
        "description": t.get("description").cloned().unwrap_or(Value::Null),
        "readOnly": t.get("annotations").and_then(|a| a.get("readOnlyHint")).and_then(Value::as_bool).unwrap_or(false),
      })
    })
    .collect();
  json!({ "tools": slim })
}

/// 工具回包文本原样输出（本就是紧凑 JSON）；--pretty 时重排，解析失败保原文不丢数据
fn print_json_text(text: &str, pretty: bool) {
  if pretty {
    if let Ok(v) = serde_json::from_str::<Value>(text) {
      println!("{}", serde_json::to_string_pretty(&v).unwrap_or_else(|_| text.to_string()));
      return;
    }
  }
  println!("{text}");
}

fn print_json(v: &Value, pretty: bool) {
  if pretty {
    println!("{}", serde_json::to_string_pretty(v).unwrap_or_else(|_| compact(v)));
  } else {
    println!("{}", compact(v));
  }
}

fn compact(v: &Value) -> String {
  serde_json::to_string(v).unwrap_or_else(|_| "null".to_string())
}

fn truncate(s: &str, max: usize) -> String {
  if s.chars().count() <= max {
    s.to_string()
  } else {
    format!("{}…", s.chars().take(max).collect::<String>())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
  }

  fn parsed_call(list: &[&str]) -> (Call, bool, Option<u16>) {
    match parse_args(&args(list)).unwrap() {
      Action::Run { pretty, port, call } => (call, pretty, port),
      Action::Help => panic!("应解析为调用"),
    }
  }

  fn tool_of(call: &Call) -> (&'static str, Value) {
    match call {
      Call::Tool { tool, args } => (tool, args.clone()),
      Call::ListTools => panic!("应为工具调用"),
    }
  }

  #[test]
  fn parse_read_only_commands() {
    let (call, pretty, port) = parsed_call(&["status"]);
    assert_eq!(tool_of(&call).0, "get_current_library");
    assert!(!pretty && port.is_none());

    let (call, _, _) = parsed_call(&["search", "杰森", "--folder", "F1", "--no-recursive", "--pretty", "--port", "17702"]);
    let (tool, a) = tool_of(&call);
    assert_eq!(tool, "search_assets");
    assert_eq!(a, json!({ "query": "杰森", "recursive": false, "folderId": "F1" }));

    let (call, _, _) = parsed_call(&["semantic", "跑步动作", "--limit=5", "--folder=F2"]);
    let (tool, a) = tool_of(&call);
    assert_eq!(tool, "semantic_search");
    assert_eq!(a, json!({ "query": "跑步动作", "recursive": true, "folderId": "F2", "limit": 5 }));

    let (call, pretty, port) = parsed_call(&["--pretty", "--port=18000", "placements"]);
    let (tool, a) = tool_of(&call);
    assert_eq!(tool, "list_placements");
    assert_eq!(a, json!({}));
    assert!(pretty && port == Some(18000));

    for (cmd, tool) in [
      ("tree", "list_folder_tree"),
      ("archives", "list_archives"),
      ("links", "list_links"),
      ("tasks", "list_semantic_analysis_tasks"),
    ] {
      let (call, _, _) = parsed_call(&[cmd]);
      assert_eq!(tool_of(&call).0, tool);
    }
    for (cmd, tool, key) in [
      ("detail", "get_asset_detail", "assetId"),
      ("path", "get_asset_origin_path", "assetId"),
      ("read", "read_text_asset", "assetId"),
      ("archive", "get_archive_data", "folderId"),
      ("task", "get_semantic_analysis_task", "taskId"),
    ] {
      let (call, _, _) = parsed_call(&[cmd, "ID-1"]);
      let (t, a) = tool_of(&call);
      assert_eq!(t, tool);
      assert_eq!(a[key], json!("ID-1"));
    }
  }

  #[test]
  fn parse_errors_and_help() {
    assert!(parse_args(&args(&["unknown"])).is_err());
    assert!(parse_args(&args(&["detail"])).is_err());
    assert!(parse_args(&args(&["status", "多余"])).is_err());
    assert!(parse_args(&args(&["search", "x", "--port"])).is_err());
    assert!(parse_args(&args(&["search", "x", "--port", "abc"])).is_err());
    assert!(parse_args(&args(&["search", "x", "--bogus"])).is_err());
    assert!(matches!(parse_args(&args(&[])).unwrap(), Action::Help));
    assert!(matches!(parse_args(&args(&["--help"])).unwrap(), Action::Help));
    assert!(matches!(parse_args(&args(&["help"])).unwrap(), Action::Help));
  }

  #[test]
  fn sse_split_events() {
    let body = "event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":1}\n\ndata: {\"a\":\ndata: 1}\n\n: 注释行\n";
    let events = sse_events(body);
    assert_eq!(events.len(), 2);
    assert_eq!(events[0], "{\"jsonrpc\":\"2.0\",\"id\":1}");
    assert_eq!(events[1], "{\"a\":\n1}");
    assert!(sse_events("").is_empty());
  }

  #[test]
  fn content_text_and_slim_tools() {
    let result = json!({ "content": [ { "type": "text", "text": "{\"a\":1}" }, { "type": "image" } ] });
    assert_eq!(content_text(&result), "{\"a\":1}");
    assert_eq!(content_text(&json!({})), "");
    let list = json!({ "tools": [ { "name": "search_assets", "description": "搜索", "inputSchema": { "big": true }, "annotations": { "readOnlyHint": true } }, { "name": "create_folder" } ] });
    let slim = slim_tools(&list);
    assert_eq!(slim["tools"][0], json!({ "name": "search_assets", "description": "搜索", "readOnly": true }));
    assert_eq!(slim["tools"][1]["readOnly"], json!(false));
  }
}
