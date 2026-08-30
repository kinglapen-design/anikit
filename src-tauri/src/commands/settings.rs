//! 命令 API：UI 设置与主窗口几何持久化（2026-07-28 用户复验定："用户设置要持久化，
//! 包括窗口大小，位置"）。数据落 core/config.rs（%APPDATA%\AniKit\config.json 的
//! settings 子树，全局配置不随库，单一写点）；UI 设置前端变更防抖 500ms 整体写回，
//! 窗口几何主窗口 resize/move 防抖提交（预览子窗口不记录）。

use crate::core::config::{self, WindowState};

/// 读取 UI 设置（settings.ui 子树原样返回；从未保存为空对象，前端按默认值补齐）
#[tauri::command]
pub async fn get_ui_settings() -> Result<serde_json::Value, String> {
  config::ui_settings().map_err(|e| e.to_string())
}

/// 整体写回 UI 设置（前端 $subscribe 防抖后提交全量子树，单点覆盖）
#[tauri::command]
pub async fn save_ui_settings(settings: serde_json::Value) -> Result<(), String> {
  config::save_ui_settings(settings).map_err(|e| e.to_string())
}

/// 写回主窗口几何（最大化时后端只刷标志位、几何保留上次还原尺寸，见 config::save_window_state）
#[tauri::command]
pub async fn save_window_state(state: WindowState) -> Result<(), String> {
  config::save_window_state(state).map_err(|e| e.to_string())
}

/// 退出应用（1.3 汉堡菜单"退出"，2026-08-11 用户定：完全结束进程）
#[tauri::command]
pub async fn exit_app(app: tauri::AppHandle) -> Result<(), String> {
  app.exit(0);
  Ok(())
}

/// 查询主窗口背景效果模式（7.1 十七轮主窗口骨架轮）："mica"（Win11 22H2+ 云母）/
/// "solid"（实色回退 --n-g-a1-solid）。前端据此前切换 .window 底色。
#[tauri::command]
pub async fn get_window_fx_mode() -> Result<String, String> {
  Ok(crate::adapters::window_fx::main_mode().to_string())
}

/// 主窗口 DWM 边框色随选择模式切换（6.2 四重标识之一；2026-08-30 实机定：云母模式边框
/// 由 DWM 画，picking = 品牌色 #3DDDD2 与前端 ::after 品牌框同色重叠错位不分层、
/// 常态 = 通用/六级 #333333）。Win11 22000+ 生效，Win10 忽略（实色模式边框前端自绘，
/// 调用无害）；任何失败静默 Ok——掉回常态色，前端品牌框仍在，观感可降级不阻断。
#[tauri::command]
pub async fn set_main_border_picking(app: tauri::AppHandle, picking: bool) -> Result<(), String> {
  #[cfg(target_os = "windows")]
  {
    use tauri::Manager;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_BORDER_COLOR};
    if let Some(w) = app.get_webview_window("main") {
      if let Ok(raw) = w.hwnd() {
        let hwnd = HWND(raw.0 as _);
        // COLORREF 0x00BBGGRR：品牌 #3DDDD2 → B=0xD2 G=0xDD R=0x3D；#333333 正反对称
        let border: u32 = if picking { 0x00D2_DD3D } else { 0x0033_3333 };
        unsafe {
          let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_BORDER_COLOR,
            &border as *const _ as *const std::ffi::c_void,
            std::mem::size_of_val(&border) as u32,
          );
        }
      }
    }
  }
  #[cfg(not(target_os = "windows"))]
  let _ = (app, picking);
  Ok(())
}
