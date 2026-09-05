//! 命令 API：插件详情子窗口（2026-09 重构：「Animate 插件详情」独立窗）——
//! 预览子窗口同款机制（commands/preview.rs open_preview_window），同插件复用聚焦。

use tauri::Manager;

/// 打开插件详情子窗口：label = plugin-{plugin_id}，已存在则聚焦复用，否则开无边框窗。
/// plugin_id 白名单：目前仅 "animate"（未来开源插件再扩）。
/// **必须 async**——Tauri v2 非 async 命令在主线程执行，主线程内 build 窗口会与事件循环互等死锁
/// （2026-07-25 实测：界面/托盘全卡死）；async 走线程池，build 内部 dispatch 回主线程不阻塞
#[tauri::command]
pub async fn open_plugin_window(app: tauri::AppHandle, plugin_id: String) -> Result<(), String> {
  if plugin_id != "animate" {
    return Err(format!("未知插件：{plugin_id}"));
  }
  let label = format!("plugin-{plugin_id}");
  if let Some(win) = app.get_webview_window(&label) {
    win.set_focus().map_err(|e| e.to_string())?;
    return Ok(());
  }
  let url = tauri::WebviewUrl::App(format!("index.html?plugin={plugin_id}").into());
  let win = tauri::WebviewWindowBuilder::new(&app, &label, url)
    .title("AniKit 插件")
    .inner_size(544.0, 538.0)
    .resizable(true)
    .decorations(false)
    .transparent(true)
    .shadow(true)
    .center()
    .build()
    .map_err(|e| e.to_string())?;
  // 与主窗口 / 预览窗同口径接云母（7.1 十七轮）
  crate::adapters::window_fx::apply(&win);
  // DWM 默认边框抑制（2026-09-06 五轮用户定：子窗口边框改前端 CSS 亮色/半透明-1 自绘——
  // 子窗口从未染 #333（染色仅 main），DWM 画的是系统默认色；默认色线与 CSS l-a1 线并集
  // 必错位（踩坑 063 机制），COLOR_NONE 独让 CSS 自绘；Win11 22000+ 生效，Win10 忽略无害）
  #[cfg(target_os = "windows")]
  {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_BORDER_COLOR};
    if let Ok(raw) = win.hwnd() {
      const DWMWA_COLOR_NONE: u32 = 0xFFFF_FFFE;
      let border: u32 = DWMWA_COLOR_NONE;
      let _ = unsafe {
        DwmSetWindowAttribute(
          HWND(raw.0 as _),
          DWMWA_BORDER_COLOR,
          &border as *const _ as *const std::ffi::c_void,
          std::mem::size_of_val(&border) as u32,
        )
      };
    }
  }
  Ok(())
}
