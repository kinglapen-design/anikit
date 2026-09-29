//! 命令 API：插件详情子窗口（2026-09 重构：「Animate 插件详情」独立窗）——
//! 预览子窗口同款机制（commands/preview.rs open_preview_window），同插件复用聚焦。

use tauri::Manager;

/// 打开插件详情子窗口：label = plugin-{plugin_id}，已存在则聚焦复用，否则开无边框窗。
/// plugin_id 白名单与窗口尺寸：animate 544×578（2026-09-06 九轮用户定：稿值 538 在路径组长路径
/// 折行后最后一行被迫滚动一条——「为了最后一行字滚动一条实在没有必要」；内容实量 ≈568 + 10 余量）、
/// mcp 544×640（2026-09-20：四组内容多于 Animate 窗，滚动兜底前的实量估值，实机走查校）。
/// **必须 async**——Tauri v2 非 async 命令在主线程执行，主线程内 build 窗口会与事件循环互等死锁
/// （2026-07-25 实测：界面/托盘全卡死）；async 走线程池，build 内部 dispatch 回主线程不阻塞
#[tauri::command]
pub async fn open_plugin_window(app: tauri::AppHandle, plugin_id: String) -> Result<(), String> {
  let (w, h) = match plugin_id.as_str() {
    "animate" => (544.0, 578.0),
    "mcp" => (544.0, 640.0),
    _ => return Err(format!("未知插件：{plugin_id}")),
  };
  let label = format!("plugin-{plugin_id}");
  if let Some(win) = app.get_webview_window(&label) {
    win.set_focus().map_err(|e| e.to_string())?;
    return Ok(());
  }
  let mk_url = |fx_solid: bool| {
    tauri::WebviewUrl::App(
      format!("index.html?plugin={plugin_id}{}", if fx_solid { "&fx=solid" } else { "" }).into(),
    )
  };
  log::info!("[plugin] 创建插件详情子窗口 {label}（transparent）");
  // 开窗走 preview.rs 共用实现（透明优先 / 失败降级非透明重建，2026-10-03 兼容轮）
  crate::commands::preview::open_child_window(&app, &label, mk_url, "AniKit 插件", w, h, "[plugin]")?;
  Ok(())
}
