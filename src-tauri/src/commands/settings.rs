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

/// 查询主窗口背景效果模式："solid"（实色 --n-g-a1-solid）。2026-10-01 弃云母后恒 solid
/// （踩坑 071 四轮：撤 Mica + 撤整窗玻璃 + caption 全程保留，换回最大化过渡动画）；
/// 接口保留仅因前端查询链不动，前端基态即实色。
#[tauri::command]
pub async fn get_window_fx_mode() -> Result<String, String> {
  Ok(crate::adapters::window_fx::main_mode().to_string())
}

/// 主窗口 DWM 边框色随选择模式切换（6.2 四重标识之一；踩坑 063：边框 = DWM 染色线
/// 独存，前端零绘制——CSS 任何线/环与 DWM 线并集必错位分离成双层；2026-10-02 实机复证：
/// 弃云母撤玻璃后 DWM 恢复画默认边框，前端 ::before 自绘与之叠成 2px 双层被用户抓出，
/// 染色体系恢复）。
/// picking = 品牌色-常态 #0F89EC（2026-08-30 用户改定，原 品牌色-边框线 #3CDDD2 废除）、
/// 常态 = 染回 #333（window_fx::BORDER_COLOR_NORMAL；2026-09-29 修订——Win11 25H2 起
/// COLOR_NONE 默认边框不再绘制，恢复原「恢复默认」会丢边框线）。
/// Win11 22000+ 生效，Win10 忽略（DWM 无边框可画，调用无害）；
/// 任何失败静默 Ok——掉回常态色，观感可降级不阻断。
#[tauri::command]
pub async fn set_main_border_picking(app: tauri::AppHandle, picking: bool) -> Result<(), String> {
  #[cfg(target_os = "windows")]
  {
    use tauri::Manager;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_BORDER_COLOR};
    if let Some(w) = app.get_webview_window("main") {
      match w.hwnd() {
        Ok(raw) => {
          let hwnd = HWND(raw.0 as _);
          // COLORREF 0x00BBGGRR：品牌色-常态 #0F89EC → B=0xEC G=0x89 R=0x0F
          let border: u32 = if picking {
            0x00EC_890F
          } else {
            crate::adapters::window_fx::BORDER_COLOR_NORMAL
          };
          let r = unsafe {
            DwmSetWindowAttribute(
              hwnd,
              DWMWA_BORDER_COLOR,
              &border as *const _ as *const std::ffi::c_void,
              std::mem::size_of_val(&border) as u32,
            )
          };
          // 埋点（2026-08-30 三轮排障：选择模式角部暗点疑似切色未生效——失败原静默吞掉无信号）
          if let Err(e) = r {
            log::warn!("[window_fx] set_main_border_picking({picking}) DwmSetWindowAttribute 失败：{e}");
          } else {
            log::info!("[window_fx] 主窗口边框切色 picking={picking}");
          }
        }
        Err(e) => log::warn!("[window_fx] set_main_border_picking 取 hwnd 失败：{e}"),
      }
    } else {
      log::warn!("[window_fx] set_main_border_picking：main 窗口不存在");
    }
  }
  #[cfg(not(target_os = "windows"))]
  let _ = (app, picking);
  Ok(())
}
