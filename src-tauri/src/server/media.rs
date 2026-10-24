//! 本地媒体服务端点（2026-10-08 anikit 协议消费端 S1，PRD 02-09 双形态分工的对外形态）：
//! 与 MCP 同 axum 实例 / 同端口（17701 优先）同生共死——mcp/mod.rs start() 挂载路由：
//!   GET /item/{id}           图片字节流（按 ID 实时解析磁盘路径，重命名/移动不死链）
//!   GET /item/by-name/{name} 图片字节流（按 originName 解析最新一笔，2026-10-23 编辑器贴图直链——
//!                            粘贴写链接时资产可能尚未入库拿不到 ID，由写入方生成唯一时间戳名）
//!   GET /open/{kind}/{id}    点击跳转回 AniKit（emit media-open 前端选中定位 + 主窗口前置 + 极简回页；
//!                            kind = item | folder）
//! 库未打开 / ID 不存在 / 文件读取失败 = 404 纯文本；deep-link 系统注册废案（2026-08-21 定案
//! 作废）由 open 端点替代——外部 MD 回链与 AI 回复链接均走 http 形态，portable 零系统副作用。

use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use tauri::{AppHandle, Emitter, Manager};

use crate::commands::library::AppState;
use crate::services::media_source;

/// GET /item/{id}：图片字节流。Cache-Control: no-cache——origin 外部编辑自愈口径（磁盘实况为准，
/// 同 URL 内容可随外部覆盖变化，禁浏览器启发式缓存）
pub async fn item_image(State(app): State<AppHandle>, Path(id): Path<String>) -> Response {
  // 锁内只查表取路径，读文件放锁外（library = std Mutex，持锁越短越好）
  let found = {
    let state = app.state::<AppState>();
    let Ok(guard) = state.library.lock() else {
      return text_err(StatusCode::INTERNAL_SERVER_ERROR, "库锁异常");
    };
    let Some(lib) = guard.as_ref() else {
      return text_err(StatusCode::NOT_FOUND, "请先打开素材库");
    };
    let Some(meta) = lib.index.assets.get(&id) else {
      return text_err(StatusCode::NOT_FOUND, "资源不存在，可能已被删除");
    };
    media_source::image_source(lib, &id, meta)
  };
  serve_image(found).await
}

/// GET /item/by-name/{name}：按磁盘文件名（originName）解析最新一笔的图片字节流。
/// 外部编辑器会在粘贴当刻请求，而收件箱需等写入完成再导入；未命中时短等重查，
/// 避免把导入途中的暂态 404 交给编辑器缓存。库锁只覆盖查表，等待与读盘均在锁外。
pub async fn item_image_by_name(State(app): State<AppHandle>, Path(name): Path<String>) -> Response {
  let state = app.state::<AppState>();
  match media_source::by_name_image(&state.library, &name).await {
    Ok(found) => serve_image(found).await,
    Err(media_source::ByNameError::Lock) => text_err(StatusCode::INTERNAL_SERVER_ERROR, "库锁异常"),
    Err(media_source::ByNameError::NoLibrary) => text_err(StatusCode::NOT_FOUND, "请先打开素材库"),
    Err(media_source::ByNameError::NotFound) => text_err(StatusCode::NOT_FOUND, "资源不存在，可能尚未导入"),
  }
}

/// 锁外读盘 + 字节流响应（404 纯文本兜底；no-cache 口径见 item_image）
async fn serve_image(found: Option<media_source::ImageSource>) -> Response {
  let Some((path, mime)) = found else {
    return text_err(StatusCode::NOT_FOUND, "该资产没有可用的图片");
  };
  match tokio::fs::read(&path).await {
    Ok(bytes) => ([(header::CONTENT_TYPE, mime), (header::CACHE_CONTROL, "no-cache")], bytes).into_response(),
    Err(_) => text_err(StatusCode::NOT_FOUND, "文件读取失败（可能已被移动或删除）"),
  }
}

/// GET /open/{kind}/{id}：点击跳转回 AniKit——前端 media-open 监听选中定位（lib/anikitLink.ts）；
/// 回极简页（浏览器停留页，跳转发生在 AniKit 侧）
pub async fn open_ref(State(app): State<AppHandle>, Path((kind, id)): Path<(String, String)>) -> Response {
  if kind != "item" && kind != "folder" {
    return text_err(StatusCode::NOT_FOUND, "未知的目标类型（应为 item / folder）");
  }
  match ensure_exists(&app, &kind, &id) {
    None => {
      let _ = app.emit("media-open", MediaOpenPayload { kind: &kind, id: &id });
      focus_main(&app);
      Html(OPEN_PAGE).into_response()
    }
    Some(r) => r,
  }
}

/// 存在性前置校验：None = 通过；Some(响应) = 失败（锁异常 = 500；库未打开 / 目标不存在 = 404 纯文本）
fn ensure_exists(app: &AppHandle, kind: &str, id: &str) -> Option<Response> {
  let state = app.state::<AppState>();
  let Ok(guard) = state.library.lock() else {
    return Some(text_err(StatusCode::INTERNAL_SERVER_ERROR, "库锁异常"));
  };
  let Some(lib) = guard.as_ref() else {
    return Some(text_err(StatusCode::NOT_FOUND, "请先打开素材库"));
  };
  let ok = if kind == "item" { lib.index.assets.contains_key(id) } else { lib.index.folders.contains_key(id) };
  if ok { None } else { Some(text_err(StatusCode::NOT_FOUND, "目标不存在，可能已被删除")) }
}

/// 主窗口前置（lib.rs 托盘 / 单实例同口径）
fn focus_main(app: &AppHandle) {
  if let Some(w) = app.get_webview_window("main") {
    crate::show_main_window(&w, false);
  }
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct MediaOpenPayload<'a> {
  kind: &'a str,
  id: &'a str,
}

/// 极简回页：深色底一行字（点击后浏览器停留页——window.close 对直接导航页无效，只留文案）
const OPEN_PAGE: &str = "<!DOCTYPE html><html lang=\"zh-CN\"><head><meta charset=\"utf-8\"><title>AniKit</title>\
<style>html,body{height:100%;margin:0}body{display:flex;align-items:center;justify-content:center;\
background:#1f1f1f;color:#888;font:14px/1.6 system-ui,sans-serif}</style></head>\
<body>已在 AniKit 中定位，可关闭本页</body></html>";

fn text_err(status: StatusCode, msg: &'static str) -> Response {
  (status, [(header::CONTENT_TYPE, "text/plain; charset=utf-8")], msg).into_response()
}
