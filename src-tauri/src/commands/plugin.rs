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
  let url = tauri::WebviewUrl::App(format!("index.html?plugin={plugin_id}").into());
  let win = tauri::WebviewWindowBuilder::new(&app, &label, url)
    .title("AniKit 插件")
    .inner_size(w, h)
    .resizable(true)
    .decorations(false)
    .transparent(true)
    .shadow(true)
    .center()
    // 锁深色（2026-09-29 修复「浅色系统主题下云母变白」）：默认跟随系统会让浅色主题
    // 把窗口 dark mode 置 0 → Mica 浅色/退化白底 × 深色 UI 无浅色适配；锁定后 Mica
    // 恒深色变体、DWM 退化实底也恒深色。与主窗口 tauri.conf theme:"Dark" 同口径
    .theme(Some(tauri::Theme::Dark))
    .build()
    .map_err(|e| e.to_string())?;
  // 与主窗口 / 预览窗同口径接云母（7.1 十七轮）；边框 = 显式染 #333（2026-09-29 修订：
  // 25H2 起 DWM 默认边框 COLOR_NONE 不再绘制，显式染色才画线）
  crate::adapters::window_fx::apply(&win);
  #[cfg(target_os = "windows")]
  if let Ok(raw) = win.hwnd() {
    crate::adapters::window_fx::paint_border(windows::Win32::Foundation::HWND(raw.0 as _));
  }
  Ok(())
}
