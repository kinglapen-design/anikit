//! 插件通信 WS 服务（PRD 07-01）——协议 v2（2026-09-25 T1 开源口径泛化）：
//! 127.0.0.1 动态端口 + 端口文件（全局配置目录 AniKit/plugin-server.port——插件
//! 经 node fs 读它发现端口）；JSON 文本帧（camelCase）。消息分两层：
//!
//! 通用层（无前缀，任何软件插件接入即用）：hello / welcome / getTree / tree /
//! saveSettings / settings / settingsError / error。
//!
//! 软件命名空间层（`<app>.<name>`，前缀须与 hello 登记的 app 一致，不符回 error）：
//! animate.exported / animate.thumbOnly / animate.paste / animate.pasteFla / animate.pasteResult。
//! 接第二个软件（Blender 等）= 新命名空间 + 同模式 handler，通用层零改动。
//!
//! hello 登记连接身份（app / version / panelVersion）；welcome 回 protocol 与
//! bundledPanelVersion（app = animate 时）——面板版本落后自行提示重开（版本握手 =
//! 2026-09-02 待办②：面板无热更，旧版跑旧码静默丢消息的根治）。
//! 设置按连接身份路由：saveSettings 不带 app，按 hello 登记分发——animate =
//! settings.cepExport 子树（config 单写点 + validate）；新软件一族 = config 子树 +
//! settings_for / save_settings_for 各加一支。
//! 导入回包字段通用口径：sourceAssetId（工程源文件）/ thumbAssetId（缩略图资产）。
//! 库数据变更广播前端事件 = plugin-imported（原 cep-imported，2026-09-25 泛化改名）。
//! cfg(not(test)) 才由 lib.rs 拉起（同 frame_stream，踩坑 014）。
//! v1（裸消息名 + cepOutDir + flaAssetId/webpAssetId）2026-09-25 废除，CEP 面板 0.4.0 同步切换。

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

#[cfg(not(test))] // 以下仅 WS server 路径使用（测试构建不起 server，踩坑 014）
use futures_util::{SinkExt, StreamExt};
#[cfg(not(test))]
use tauri::{AppHandle, Emitter, Manager};
#[cfg(not(test))]
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot};

#[cfg(not(test))]
use crate::commands::library::AppState;
#[cfg(not(test))]
use crate::core::library::TEMP_DIR;
#[cfg(not(test))]
use crate::services::cep_export::{self, ExportPayload};

/// 协议版本：2 = 2026-09-25 命名空间泛化（v1 = 8.1 裸消息名，已废除）
#[cfg(not(test))] // 仅 server 路径（welcome_payload）消费
const PROTOCOL_VERSION: u32 = 2;

/// 客户端连接身份（hello 登记）
#[derive(Debug, Clone)]
struct ClientMeta {
  app: String,
  /// 软件版本（登记备查 + 能力分级演进位；现仅 hello 日志消费）
  #[cfg_attr(test, allow(dead_code))]
  version: String,
  /// 面板版本（版本握手备查；比对在面板侧做，服务端仅登记）
  #[cfg_attr(test, allow(dead_code))]
  panel_version: Option<String>,
}

/// 客户端条目：服务端 → 插件单播通道 + 连接身份（None = 未 hello）
struct ClientEntry {
  tx: mpsc::UnboundedSender<String>,
  meta: Option<ClientMeta>,
}

/// 客户端出口表（连接 id → 条目）
static CLIENTS: OnceLock<Mutex<HashMap<u64, ClientEntry>>> = OnceLock::new();

fn clients() -> &'static Mutex<HashMap<u64, ClientEntry>> {
  CLIENTS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 该连接登记的软件标识（未握手 = None）
#[cfg(not(test))]
fn registered_app(id: u64) -> Option<String> {
  clients().lock().unwrap().get(&id).and_then(|e| e.meta.as_ref().map(|m| m.app.clone()))
}

/// 取指定软件的在线客户端（发送至 AN 只发 animate 客户端——多软件并存不串台）
fn pick_client(app: &str) -> Option<(u64, mpsc::UnboundedSender<String>)> {
  clients()
    .lock()
    .unwrap()
    .iter()
    .find(|(_, e)| e.meta.as_ref().is_some_and(|m| m.app == app))
    .map(|(id, e)| (*id, e.tx.clone()))
}

/// —— 发送至 AN（8.3，PRD 07-04）请求-回执 ——
/// pending 表：reqId → (客户端 id, 回执通道)；客户端断开即 drain 当场失败（不等超时，用户当场知情）
type PendingMap = Mutex<HashMap<u64, (u64, oneshot::Sender<serde_json::Value>)>>;
static PENDING: OnceLock<PendingMap> = OnceLock::new();
/// 全局发送锁：同一时刻仅一趟发送——旧 electron 版痛点根治（CEP 未连通时点发送请求积压不报错，
/// 重连后积压队列一次性全跑 = 舞台连粘 N 次）。本设计：不在线立即报错不排队；在途期间拒绝新发送。
static SEND_BUSY: AtomicBool = AtomicBool::new(false);
static NEXT_REQ_ID: AtomicU64 = AtomicU64::new(1);

fn pending() -> &'static PendingMap {
  PENDING.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 发送许可（RAII：一趟发送全程持有——主方案/兜底只走其一，许可内可发一次请求）
pub struct PastePermit {
  _private: (),
}

/// 抢占发送权；在途返回 None（命令层报「上一次发送尚未完成」）
pub fn try_acquire_send() -> Option<PastePermit> {
  if SEND_BUSY.swap(true, Ordering::SeqCst) {
    None
  } else {
    Some(PastePermit { _private: () })
  }
}

impl Drop for PastePermit {
  fn drop(&mut self) {
    SEND_BUSY.store(false, Ordering::SeqCst);
  }
}

/// 向在线 Animate 面板发粘贴类请求并等回执（20s 超时）。
/// action = "paste"（主方案：剪贴板已由后端写好）/ "pasteFla"（兜底：jsx 打开-复制-粘贴-关闭）；
/// 实际下发消息类型 = `animate.{action}`。无在线客户端立即 Err（不排队——积压根治）；
/// 回执 = animate.pasteResult 消息体。
pub async fn request_paste(action: &str, name: &str, fla_path: Option<&str>) -> Result<serde_json::Value, String> {
  let (client_id, tx) = pick_client("animate").ok_or_else(|| "Animate 面板未连接，请先在 Animate 中打开 AniKit 面板".to_string())?;
  let req_id = NEXT_REQ_ID.fetch_add(1, Ordering::SeqCst);
  let (rtx, rrx) = oneshot::channel();
  pending().lock().unwrap().insert(req_id, (client_id, rtx));
  let msg_type = format!("animate.{action}");
  let msg = match fla_path {
    Some(p) => serde_json::json!({ "type": msg_type, "reqId": req_id, "name": name, "flaPath": p }),
    None => serde_json::json!({ "type": msg_type, "reqId": req_id, "name": name }),
  };
  if tx.send(msg.to_string()).is_err() {
    pending().lock().unwrap().remove(&req_id);
    return Err("消息发送失败（面板连接已断开）".to_string());
  }
  let r = match tokio::time::timeout(std::time::Duration::from_secs(20), rrx).await {
    Ok(Ok(v)) => Ok(v),
    Ok(Err(_)) => Err("面板连接中断".to_string()),
    Err(_) => Err("粘贴等待超时（20s）——Animate 可能正忙，请检查后重试".to_string()),
  };
  pending().lock().unwrap().remove(&req_id);
  r
}

/// pasteResult 回执路由（handle_conn 内调用）
#[cfg(not(test))]
fn resolve_paste(v: &serde_json::Value) {
  let Some(req_id) = v.get("reqId").and_then(|x| x.as_u64()) else { return };
  if let Some((_, sender)) = pending().lock().unwrap().remove(&req_id) {
    let _ = sender.send(v.clone());
  }
}

/// 客户端断开：drain 其 pending 请求立即失败（不等超时）
#[cfg(not(test))]
fn fail_pending_of(client_id: u64) {
  let mut map = pending().lock().unwrap();
  let ids: Vec<u64> = map.iter().filter(|(_, (cid, _))| *cid == client_id).map(|(rid, _)| *rid).collect();
  for rid in ids {
    if let Some((_, sender)) = map.remove(&rid) {
      let _ = sender.send(serde_json::json!({ "ok": false, "message": "面板连接中断，粘贴未完成" }));
    }
  }
}


/// lib.rs setup 拉起（tauri 异步运行时）
#[cfg(not(test))] // 测试构建不起 server（踩坑 014）
pub fn init(app: AppHandle) {
  tauri::async_runtime::spawn(async move {
    if let Err(e) = serve(app).await {
      log::error!("插件通信 WS 服务异常退出：{e}");
    }
  });
}

#[cfg(not(test))]
async fn serve(app: AppHandle) -> Result<(), String> {
  let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|e| e.to_string())?;
  let port = listener.local_addr().map_err(|e| e.to_string())?.port();
  write_port_file(port);
  log::info!("插件通信 WS 服务就绪：127.0.0.1:{port}（协议 v{PROTOCOL_VERSION}）");
  let mut next_id = 1u64;
  loop {
    let (stream, peer) = listener.accept().await.map_err(|e| e.to_string())?;
    log::info!("插件客户端接入：{peer}");
    tokio::spawn(handle_conn(app.clone(), next_id, stream));
    next_id += 1;
  }
}

/// 端口文件（插件发现机制）：每次启动覆写；进程退出残留无害（面板连不上即显示未连接）
#[cfg(not(test))]
fn write_port_file(port: u16) {
  let Some(dir) = dirs::config_dir() else {
    return;
  };
  let dir = dir.join("AniKit");
  let _ = std::fs::create_dir_all(&dir);
  if let Err(e) = std::fs::write(dir.join("plugin-server.port"), port.to_string()) {
    log::warn!("插件端口文件写入失败：{e}");
  }
}

/// 协议错误回包（未知类型 / 未握手 / 命名空间与身份不符——第三方调试友好）
#[cfg(not(test))]
fn error_payload(message: impl Into<String>) -> String {
  serde_json::json!({ "type": "error", "message": message.into() }).to_string()
}

#[cfg(not(test))]
async fn handle_conn(app: AppHandle, id: u64, stream: tokio::net::TcpStream) {
  let ws = match tokio_tungstenite::accept_async(stream).await {
    Ok(w) => w,
    Err(e) => {
      log::warn!("WS 握手失败：{e}");
      return;
    }
  };
  let (mut sink, mut read) = ws.split();
  let (tx, mut rx) = mpsc::unbounded_channel::<String>();
  clients().lock().unwrap().insert(id, ClientEntry { tx: tx.clone(), meta: None });
  // 出队泵：服务端 → 该客户端
  let pump = tokio::spawn(async move {
    while let Some(text) = rx.recv().await {
      if sink.send(tokio_tungstenite::tungstenite::Message::Text(text.into())).await.is_err() {
        break;
      }
    }
  });
  log::info!("插件客户端 #{id} 握手成功");
  while let Some(msg) = read.next().await {
    let Ok(msg) = msg else { break };
    let Ok(text) = msg.into_text() else { continue };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else { continue };
    let Some(msg_type) = v.get("type").and_then(|t| t.as_str()).map(|s| s.to_string()) else { continue };
    match msg_type.as_str() {
      "hello" => {
        // 登记连接身份（app / version / panelVersion）；welcome 按身份下发（设置子树 + 面板版本握手）
        let meta = ClientMeta {
          app: v.get("app").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
          version: v.get("version").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
          panel_version: v.get("panelVersion").and_then(|x| x.as_str()).map(|s| s.to_string()),
        };
        log::info!(
          "插件 #{id} 登记：app={} version={} panelVersion={}",
          meta.app,
          meta.version,
          meta.panel_version.as_deref().unwrap_or("-")
        );
        clients().lock().unwrap().entry(id).and_modify(|e| e.meta = Some(meta.clone()));
        let _ = tx.send(welcome_payload(&app, &meta.app).to_string());
      }
      "getTree" => {
        let _ = tx.send(tree_payload(&app).to_string());
      }
      "saveSettings" => {
        // 设置回写按连接身份路由（hello 登记的 app → 对应设置族：校验 → config 单写点落盘
        // → 全员广播含发件者回显）；校验 / 解析失败回发件者 settingsError
        let raw = v.get("settings").cloned().unwrap_or_default();
        let result = match registered_app(id) {
          Some(app_id) => save_settings_for(&app_id, raw),
          None => Err("请先 hello 握手登记软件标识".to_string()),
        };
        match result {
          Ok(()) => {
            if let Some(app_id) = registered_app(id) {
              broadcast_settings(&app_id);
            }
          }
          Err(message) => {
            let _ = tx.send(serde_json::json!({ "type": "settingsError", "message": message }).to_string());
          }
        }
      }
      _ => {
        // 软件命名空间消息 <app>.<name>：前缀须与本连接登记身份一致
        let Some((ns, name)) = msg_type.split_once('.') else {
          let _ = tx.send(error_payload(format!("未知消息类型：{msg_type}")));
          continue;
        };
        match registered_app(id) {
          None => {
            let _ = tx.send(error_payload("请先 hello 握手登记软件标识"));
          }
          Some(a) if a != ns => {
            let _ = tx.send(error_payload(format!("命名空间 {ns} 与本连接登记身份 {a} 不符")));
          }
          Some(_) => match (ns, name) {
            ("animate", "exported") => {
              let app2 = app.clone();
              let tx2 = tx.clone();
              let payload = v.get("payload").cloned().unwrap_or_default();
              tokio::spawn(async move {
                handle_exported(app2, tx2, payload).await;
              });
            }
            ("animate", "thumbOnly") => {
              let app2 = app.clone();
              let tx2 = tx.clone();
              let payload = v.get("payload").cloned().unwrap_or_default();
              tokio::spawn(async move {
                handle_thumb_only(app2, tx2, payload).await;
              });
            }
            // 发送至 AN（8.3）粘贴回执：路由到等待中的请求
            ("animate", "pasteResult") => resolve_paste(&v),
            ("animate", _) => {
              let _ = tx.send(error_payload(format!("未知消息类型：{msg_type}")));
            }
            (_, _) => {
              let _ = tx.send(error_payload(format!("未知软件命名空间：{ns}")));
            }
          },
        }
      }
    }
  }
  clients().lock().unwrap().remove(&id);
  fail_pending_of(id);
  pump.abort();
  log::info!("插件客户端 #{id} 断开");
}

/// welcome：协议版本 + 库就绪态 + 导出中转落点 + 该软件设置子树（+ animate 面板版本握手）
#[cfg(not(test))]
fn welcome_payload(app: &AppHandle, app_id: &str) -> serde_json::Value {
  let state = app.state::<AppState>();
  let guard = state.library.lock().unwrap();
  let (ready, lib_root, export_dir) = match guard.as_ref() {
    Some(lib) => (
      true,
      lib.root.to_string_lossy().into_owned(),
      lib.root.join(TEMP_DIR).join("cep-out").to_string_lossy().into_owned(),
    ),
    None => (false, String::new(), String::new()),
  };
  let mut v = serde_json::json!({
    "type": "welcome",
    "protocol": PROTOCOL_VERSION,
    "libReady": ready,
    "libRoot": lib_root,
    "exportDir": export_dir,
    "settings": settings_for(app_id)
  });
  // 版本握手（2026-09-02 待办②）：面板无热更——bundled 与面板自报不一致时面板提示重开
  if app_id == "animate" {
    v["bundledPanelVersion"] = serde_json::Value::String(crate::services::cep_install::BUNDLED_VERSION.to_string());
  }
  v
}

/// 该软件的设置子树 JSON（welcome 下发 / settings 广播共用）；无设置族的软件 = null
fn settings_for(app: &str) -> serde_json::Value {
  match app {
    // animate = settings.cepExport 子树（读盘失败回退默认档）
    "animate" => {
      let s = crate::core::config::cep_export_settings().unwrap_or_default();
      serde_json::json!({ "maxSecs": s.max_secs, "maxBytes": s.max_bytes, "fps": s.fps })
    }
    _ => serde_json::Value::Null,
  }
}

/// saveSettings 按连接身份路由到对应软件设置族（护栏校验 + config 单写点落盘）
#[cfg(not(test))]
fn save_settings_for(app: &str, raw: serde_json::Value) -> Result<(), String> {
  match app {
    "animate" => serde_json::from_value::<crate::core::config::CepExportSettings>(raw)
      .map_err(|e| format!("设置载荷解析失败：{e}"))
      .and_then(|s| s.validate().map(|_| s))
      .and_then(|s| crate::core::config::save_cep_export_settings(s).map_err(|e| e.to_string())),
    _ => Err(format!("软件 {app:?} 无可写设置族")),
  }
}

/// 设置变更广播（APP 命令层保存 / 插件 saveSettings 回写后调用）：全部在线客户端推送
/// settings 消息（带 app 标识——客户端只收自己命名空间的设置）
pub fn broadcast_settings(app: &str) {
  let msg = serde_json::json!({ "type": "settings", "app": app, "settings": settings_for(app) }).to_string();
  let clients = clients().lock().unwrap();
  for (id, entry) in clients.iter() {
    if let Err(e) = entry.tx.send(msg.clone()) {
      log::warn!("设置推送失败（客户端 #{id}）：{e}");
    }
  }
}

/// tree：文件夹树（面板目标选择器数据源；services::folder_tree 组装，命令层 / MCP 同函数）
#[cfg(not(test))]
fn tree_payload(app: &AppHandle) -> serde_json::Value {
  let state = app.state::<AppState>();
  let guard = state.library.lock().unwrap();
  match guard.as_ref() {
    Some(lib) => serde_json::json!({ "type": "tree", "data": crate::services::folder_tree::folder_tree(lib) }),
    None => serde_json::json!({ "type": "tree", "data": null }),
  }
}

/// animate.exported：三段式接收入库——短锁 prepare → 锁外 compose（ffmpeg 秒级慢 IO 不冻结命令层）→ 锁 commit
#[cfg(not(test))]
async fn handle_exported(app: AppHandle, tx: mpsc::UnboundedSender<String>, payload: serde_json::Value) {
  let payload: ExportPayload = match serde_json::from_value(payload) {
    Ok(p) => p,
    Err(e) => {
      let _ = tx.send(serde_json::json!({ "type": "animate.importError", "message": format!("导出载荷解析失败：{e}") }).to_string());
      return;
    }
  };
  let state = app.state::<AppState>();
  let prepared = {
    let guard = state.library.lock().unwrap();
    match guard.as_ref() {
      Some(lib) => cep_export::prepare(lib, &payload),
      None => Err("未打开资源库".to_string()),
    }
  };
  let prepared = match prepared {
    Ok(p) => p,
    Err(e) => {
      let _ = tx.send(serde_json::json!({ "type": "animate.importError", "message": e }).to_string());
      return;
    }
  };
  let _ = tx.send(serde_json::json!({ "type": "animate.importProgress", "stage": "compose" }).to_string());
  let composed = cep_export::compose(&prepared); // 锁外慢 IO
  let (over_limit, bytes) = match composed {
    Ok(r) => r,
    Err(e) => {
      let _ = tx.send(serde_json::json!({ "type": "animate.importError", "message": e }).to_string());
      return;
    }
  };
  let result = {
    let mut guard = state.library.lock().unwrap();
    match guard.as_mut() {
      Some(lib) => cep_export::commit(lib, &prepared, over_limit, bytes),
      None => Err("未打开资源库".to_string()),
    }
  };
  match result {
    Ok(outcome) => {
      let _ = tx.send(
        serde_json::json!({
          "type": "animate.importDone",
          "sourceAssetId": outcome.fla_asset_id,
          "thumbAssetId": outcome.webp_asset_id,
          "overLimit": outcome.over_limit,
          "webpBytes": outcome.webp_bytes
        })
        .to_string(),
      );
      // 前端全量对齐（fla 卡片换伴生图 + 缩略图视图入列 + 未分类新增）
      let _ = app.emit("plugin-imported", ());
    }
    Err(e) => {
      let _ = tx.send(serde_json::json!({ "type": "animate.importError", "message": e }).to_string());
    }
  }
}

/// animate.thumbOnly（批次 2）：仅缩略图三段式——短锁 thumb_prepare → 锁外 thumb_compose → 锁 thumb_commit；
/// 命中回 animate.thumbDone{mode:"bound"}（emit plugin-imported 前端对齐），池外回 {mode:"beside"}
#[cfg(not(test))]
async fn handle_thumb_only(app: AppHandle, tx: mpsc::UnboundedSender<String>, payload: serde_json::Value) {
  let payload: cep_export::ThumbPayload = match serde_json::from_value(payload) {
    Ok(p) => p,
    Err(e) => {
      let _ = tx.send(serde_json::json!({ "type": "animate.thumbError", "message": format!("载荷解析失败：{e}") }).to_string());
      return;
    }
  };
  let state = app.state::<AppState>();
  let prepared = {
    let guard = state.library.lock().unwrap();
    match guard.as_ref() {
      Some(lib) => cep_export::thumb_prepare(lib, &payload),
      None => Err("未打开资源库".to_string()),
    }
  };
  let prepared = match prepared {
    Ok(p) => p,
    Err(e) => {
      let _ = tx.send(serde_json::json!({ "type": "animate.thumbError", "message": e }).to_string());
      return;
    }
  };
  let _ = tx.send(serde_json::json!({ "type": "animate.importProgress", "stage": "compose" }).to_string());
  let (over_limit, bytes) = match cep_export::thumb_compose(&prepared) {
    Ok(r) => r,
    Err(e) => {
      let _ = tx.send(serde_json::json!({ "type": "animate.thumbError", "message": e }).to_string());
      return;
    }
  };
  let result = {
    let mut guard = state.library.lock().unwrap();
    match guard.as_mut() {
      Some(lib) => cep_export::thumb_commit(lib, &prepared, over_limit, bytes),
      None => Err("未打开资源库".to_string()),
    }
  };
  match result {
    Ok(outcome) => {
      let bound = matches!(outcome, cep_export::ThumbOutcome::Bound { .. });
      let mut msg = serde_json::json!({ "type": "animate.thumbDone" });
      msg.as_object_mut().unwrap().extend(serde_json::to_value(outcome).unwrap().as_object().unwrap().clone());
      let _ = tx.send(msg.to_string());
      if bound {
        let _ = app.emit("plugin-imported", ()); // 库数据已变（换绑/新增 hidden），前端全量对齐
      }
    }
    Err(e) => {
      let _ = tx.send(serde_json::json!({ "type": "animate.thumbError", "message": e }).to_string());
    }
  }
}
