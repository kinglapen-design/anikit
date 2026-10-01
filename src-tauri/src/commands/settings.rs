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

/// 主窗口 DWM 边框染色——**2026-10-01 起 no-op**（弃云母转实色单态后，边框/品牌框
/// 主权全归前端自绘：常态 ::before 1px --n-g6、picking ::after 1px --n-brand；
/// DWM 染色线与 CSS 线并集必错位分离成双层，踩坑 063 实测，故染色全撤；
/// 25H2 起 DWM 默认边框本就不再绘制，不染 = 前端单画干净解）。
/// 历史：6.2 四重标识 = DWM 切染品牌色 #0F89EC（云母期口径）；2026-09-29 曾为 25H2
/// 边框停画补显式染色——那是云母模式（前端零绘制）的修复，实色单态下由前端接管。
/// 保留命令壳仅因前端 picker store 调用链不动；任何情况静默 Ok。
#[tauri::command]
pub async fn set_main_border_picking(app: tauri::AppHandle, picking: bool) -> Result<(), String> {
  let _ = (app, picking);
  Ok(())
}
