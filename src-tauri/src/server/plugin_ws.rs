//! 插件通信 WS 服务（PRD 07-01；8.1，spike/cep-proto 验证的链路正式化）：
//! 127.0.0.1 动态端口 + 端口文件（全局配置目录 AniKit/plugin-server.port——CEP 面板
//! 经 node fs 读它发现端口）；JSON 文本帧协议（camelCase）：
//! - C→S `hello`        → S→C `welcome`（libReady / libRoot / cepOutDir / settings）
//! - C→S `getTree`      → S→C `tree`（文件夹树，供面板目标选择器；2b）
//! - C→S `exported`     → 接收入库（services::cep_export 三段式；合成在锁外）→ `importDone` / `importError`
//!
//! 协议演进方向（8.2+）：批量导出任务下发 / 快速发送指令 / 设置变更推送，同通道加分 type 即可。
//! cfg(not(test)) 才由 lib.rs 拉起（同 frame_stream，踩坑 014）。

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use futures_util::{SinkExt, StreamExt};
use tauri::{AppHandle, Emitter, Manager};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot};

use crate::commands::library::AppState;
use crate::core::library::TEMP_DIR;
use crate::services::cep_export::{self, ExportPayload};

/// 客户端出口表（连接 id → 推消息通道；服务端 → 插件单播）
static CLIENTS: OnceLock<Mutex<HashMap<u64, mpsc::UnboundedSender<String>>>> = OnceLock::new();

fn clients() -> &'static Mutex<HashMap<u64, mpsc::UnboundedSender<String>>> {
  CLIENTS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// —— 发送至 AN（8.3，PRD 07-04）请求-回执 ——
/// pending 表：reqId → (客户端 id, 回执通道)；客户端断开即 drain 当场失败（不等超时，用户当场知情）
static PENDING: OnceLock<Mutex<HashMap<u64, (u64, oneshot::Sender<serde_json::Value>)>>> = OnceLock::new();
/// 全局发送锁：同一时刻仅一趟发送——旧 electron 版痛点根治（CEP 未连通时点发送请求积压不报错，
/// 重连后积压队列一次性全跑 = 舞台连粘 N 次）。本设计：不在线立即报错不排队；在途期间拒绝新发送。
static SEND_BUSY: AtomicBool = AtomicBool::new(false);
static NEXT_REQ_ID: AtomicU64 = AtomicU64::new(1);

fn pending() -> &'static Mutex<HashMap<u64, (u64, oneshot::Sender<serde_json::Value>)>> {
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

/// 向在线插件发粘贴类请求并等回执（20s 超时）。
/// msg_type = "paste"（主方案：剪贴板已由后端写好）/ "pasteFla"（兜底：jsx 打开-复制-粘贴-关闭）。
/// 无在线客户端立即 Err（不排队——积压根治）；回执 = pasteResult 消息体。
pub async fn request_paste(msg_type: &str, name: &str, fla_path: Option<&str>) -> Result<serde_json::Value, String> {
  let picked = clients().lock().unwrap().iter().next().map(|(id, tx)| (*id, tx.clone()));
  let (client_id, tx) = picked.ok_or_else(|| "Animate 面板未连接，请先在 Animate 中打开 AniKit 面板".to_string())?;
  let req_id = NEXT_REQ_ID.fetch_add(1, Ordering::SeqCst);
  let (rtx, rrx) = oneshot::channel();
  pending().lock().unwrap().insert(req_id, (client_id, rtx));
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
fn resolve_paste(v: &serde_json::Value) {
  let Some(req_id) = v.get("reqId").and_then(|x| x.as_u64()) else { return };
  if let Some((_, sender)) = pending().lock().unwrap().remove(&req_id) {
    let _ = sender.send(v.clone());
  }
}

/// 客户端断开：drain 其 pending 请求立即失败（不等超时）
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
pub fn init(app: AppHandle) {
  tauri::async_runtime::spawn(async move {
    if let Err(e) = serve(app).await {
      log::error!("插件通信 WS 服务异常退出：{e}");
    }
  });
}

async fn serve(app: AppHandle) -> Result<(), String> {
  let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|e| e.to_string())?;
  let port = listener.local_addr().map_err(|e| e.to_string())?.port();
  write_port_file(port);
  log::info!("插件通信 WS 服务就绪：127.0.0.1:{port}");
  let mut next_id = 1u64;
  loop {
    let (stream, peer) = listener.accept().await.map_err(|e| e.to_string())?;
    log::info!("插件客户端接入：{peer}");
    tokio::spawn(handle_conn(app.clone(), next_id, stream));
    next_id += 1;
  }
}

/// 端口文件（CEP 面板发现机制）：每次启动覆写；进程退出残留无害（面板连不上即显示未连接）
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
  clients().lock().unwrap().insert(id, tx.clone());
  // 出队泵：服务端 → 该客户端
  let mut pump = tokio::spawn(async move {
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
    match v.get("type").and_then(|t| t.as_str()) {
      Some("hello") => {
        let _ = tx.send(welcome_payload(&app).to_string());
      }
      Some("getTree") => {
        let _ = tx.send(tree_payload(&app).to_string());
      }
      Some("exported") => {
        let app2 = app.clone();
        let tx2 = tx.clone();
        let payload = v.get("payload").cloned().unwrap_or_default();
        tokio::spawn(async move {
          handle_exported(app2, tx2, payload).await;
        });
      }
      Some("thumbOnly") => {
        // 批次 2：仅缩略图（不导新 fla）——同三段式，命中绑定 / 池外挪原文件旁边
        let app2 = app.clone();
        let tx2 = tx.clone();
        let payload = v.get("payload").cloned().unwrap_or_default();
        tokio::spawn(async move {
          handle_thumb_only(app2, tx2, payload).await;
        });
      }
      Some("pasteResult") => {
        // 发送至 AN（8.3）粘贴回执：路由到等待中的请求
        resolve_paste(&v);
      }
      _ => {}
    }
  }
  clients().lock().unwrap().remove(&id);
  fail_pending_of(id);
  pump.abort();
  log::info!("插件客户端 #{id} 断开");
}

/// welcome：库就绪态 + 落点路径 + 导出设置（settings.cepExport 子树，APP 设置页单写点）
fn welcome_payload(app: &AppHandle) -> serde_json::Value {
  let state = app.state::<AppState>();
  let guard = state.library.lock().unwrap();
  let (ready, lib_root, cep_out) = match guard.as_ref() {
    Some(lib) => (
      true,
      lib.root.to_string_lossy().into_owned(),
      lib.root.join(TEMP_DIR).join("cep-out").to_string_lossy().into_owned(),
    ),
    None => (false, String::new(), String::new()),
  };
  serde_json::json!({
    "type": "welcome",
    "libReady": ready,
    "libRoot": lib_root,
    "cepOutDir": cep_out,
    "settings": settings_json()
  })
}

/// 导出设置 JSON（读盘失败回退默认档）
fn settings_json() -> serde_json::Value {
  let s = crate::core::config::cep_export_settings().unwrap_or_default();
  serde_json::json!({ "maxSecs": s.max_secs, "maxBytes": s.max_bytes, "maxEdge": s.max_edge })
}

/// 设置变更广播（APP 设置页保存后由命令层调用）：全部在线客户端推送 settings 消息
pub fn broadcast_settings() {
  let msg = serde_json::json!({ "type": "settings", "settings": settings_json() }).to_string();
  let clients = clients().lock().unwrap();
  for (id, tx) in clients.iter() {
    if let Err(e) = tx.send(msg.clone()) {
      log::warn!("设置推送失败（客户端 #{id}）：{e}");
    }
  }
}

/// tree：文件夹树（面板目标选择器数据源；folder_tree 现居 commands 层，复用——结构整理随 8.x 再下迁）
fn tree_payload(app: &AppHandle) -> serde_json::Value {
  let state = app.state::<AppState>();
  let guard = state.library.lock().unwrap();
  match guard.as_ref() {
    Some(lib) => serde_json::json!({ "type": "tree", "data": crate::commands::import::folder_tree(lib) }),
    None => serde_json::json!({ "type": "tree", "data": null }),
  }
}

/// exported：三段式接收入库——短锁 prepare → 锁外 compose（ffmpeg 秒级慢 IO 不冻结命令层）→ 锁 commit
async fn handle_exported(app: AppHandle, tx: mpsc::UnboundedSender<String>, payload: serde_json::Value) {
  let payload: ExportPayload = match serde_json::from_value(payload) {
    Ok(p) => p,
    Err(e) => {
      let _ = tx.send(serde_json::json!({ "type": "importError", "message": format!("导出载荷解析失败：{e}") }).to_string());
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
      let _ = tx.send(serde_json::json!({ "type": "importError", "message": e }).to_string());
      return;
    }
  };
  let _ = tx.send(serde_json::json!({ "type": "importProgress", "stage": "compose" }).to_string());
  let composed = cep_export::compose(&prepared); // 锁外慢 IO
  let (over_limit, bytes) = match composed {
    Ok(r) => r,
    Err(e) => {
      let _ = tx.send(serde_json::json!({ "type": "importError", "message": e }).to_string());
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
          "type": "importDone",
          "flaAssetId": outcome.fla_asset_id,
          "webpAssetId": outcome.webp_asset_id,
          "overLimit": outcome.over_limit,
          "webpBytes": outcome.webp_bytes
        })
        .to_string(),
      );
      // 前端全量对齐（fla 卡片换伴生图 + 缩略图视图入列 + 未分类新增）
      let _ = app.emit("cep-imported", ());
    }
    Err(e) => {
      let _ = tx.send(serde_json::json!({ "type": "importError", "message": e }).to_string());
    }
  }
}

/// thumbOnly（批次 2）：仅缩略图三段式——短锁 thumb_prepare → 锁外 thumb_compose → 锁 thumb_commit；
/// 命中回 thumbDone{mode:"bound"}（emit cep-imported 前端对齐），池外回 {mode:"beside"}
async fn handle_thumb_only(app: AppHandle, tx: mpsc::UnboundedSender<String>, payload: serde_json::Value) {
  let payload: cep_export::ThumbPayload = match serde_json::from_value(payload) {
    Ok(p) => p,
    Err(e) => {
      let _ = tx.send(serde_json::json!({ "type": "thumbError", "message": format!("载荷解析失败：{e}") }).to_string());
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
      let _ = tx.send(serde_json::json!({ "type": "thumbError", "message": e }).to_string());
      return;
    }
  };
  let _ = tx.send(serde_json::json!({ "type": "importProgress", "stage": "compose" }).to_string());
  let (over_limit, bytes) = match cep_export::thumb_compose(&prepared) {
    Ok(r) => r,
    Err(e) => {
      let _ = tx.send(serde_json::json!({ "type": "thumbError", "message": e }).to_string());
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
      let mut msg = serde_json::json!({ "type": "thumbDone" });
      msg.as_object_mut().unwrap().extend(serde_json::to_value(outcome).unwrap().as_object().unwrap().clone());
      let _ = tx.send(msg.to_string());
      if bound {
        let _ = app.emit("cep-imported", ()); // 库数据已变（换绑/新增 hidden），前端全量对齐
      }
    }
    Err(e) => {
      let _ = tx.send(serde_json::json!({ "type": "thumbError", "message": e }).to_string());
    }
  }
}
