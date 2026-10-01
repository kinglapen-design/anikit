//! 窗口背景效果适配器（2026-10-01 终局转向：弃云母，换回最大化过渡动画）。
//!
//! 终局配置：`transparent:false + backgroundColor:#1F1F1F（不透明实底）+ shadow:true`——
//! 无云母、无整窗玻璃（`DwmExtendFrameIntoClientArea(-1)` 已撤，它本是云母载体）、
//! `WS_CAPTION` 全程保留（最大化不再摘）。DWM 最大化/还原过渡动画依附 caption
//! （Electron/VSCode 无边框先例 = 全程保 caption + NCCALCSIZE 归零 = 有动画、
//! 无系统按钮穿帮）；而 DWM 原生按钮的画布是整窗玻璃（踩坑 071 一轮实证：
//! 最大化态撤玻璃 → 稳态按钮没了）——撤玻璃后 caption 无需再摘，
//! 动画与「无原生按钮」兼得。底色改不透明后 WebView2 不再依赖 DWM backdrop，
//! 前端 get_window_fx_mode 恒 solid 走基态实色（--n-g-a1-solid #1F1F1F，前端零改动）。
//!
//! 历史（踩坑 071 全录）：7.1 十七轮接云母（Win11 22H2+ Mica，063 六轮定稿
//! `transparent:false + ExtendFrame(-1)` 玻璃载体 + set_effects）→ 玻璃 + tao 保留的
//! WS_CAPTION 让 DWM 把原生按钮画进窗内（最大化稳态 + 动画期 + 常态顶边细条三态穿帮）
//! → 2026-09-08 三轮终局「WM_WINDOWPOSCHANGING 动画前摘 caption、还原加回」杀按钮，
//! 连带伤亡 = 动画期 DWM 面对无 caption 窗口不播过渡动画。2026-10-01 用户定案：
//! 动画优先于云母，删云母路线（撤 Mica + 撤玻璃 + caption 全程保留）。

use std::sync::OnceLock;

/// 主窗口实际生效的模式（恒 "solid"；保留接口仅因前端 get_window_fx_mode 查询链不动，
/// 前端基态即实色，solid 时不加 fx-mica 类）。
static MAIN_MODE: OnceLock<&'static str> = OnceLock::new();

/// 主窗口守卫 subclass：失焦欺骗（2026-08-30 用户方案）——失焦时 DWM 把窗口阴影收成
/// 小圈，绑 DWM 的"窗口激活态"渲染判定；拦截 WM_NCACTIVATE 把 wParam 恒改 TRUE 再下放
/// 默认链（lParam -1 = 阻止默认流程重绘非客户区），骗 DWM 恒按激活渲染帧/阴影。
/// 焦点/activation 本身不变（WM_ACTIVATE、前后台焦点事件不受影响）。
/// （历史：曾兼管「最大化摘 caption、还原加回」与「最大化矩形约束 rcWork」——
/// 2026-10-01 随弃云母一并撤除：caption 全程保留后 Windows 默认最大化矩形即 rcWork，
/// 无需约束；摘 caption 杀按钮的职责由「撤玻璃」取代，见模块注释。）
#[cfg(windows)]
unsafe extern "system" fn main_window_guard(
  hwnd: windows::Win32::Foundation::HWND,
  msg: u32,
  wparam: windows::Win32::Foundation::WPARAM,
  lparam: windows::Win32::Foundation::LPARAM,
  _id: usize,
  _data: usize,
) -> windows::Win32::Foundation::LRESULT {
  use windows::Win32::Foundation::{LPARAM, WPARAM};
  use windows::Win32::UI::Shell::DefSubclassProc;
  use windows::Win32::UI::WindowsAndMessaging::WM_NCACTIVATE;
  if msg == WM_NCACTIVATE {
    return DefSubclassProc(hwnd, msg, WPARAM(1), LPARAM(-1));
  }
  DefSubclassProc(hwnd, msg, wparam, lparam)
}

/// 给主窗口装守卫 subclass（show 后随边框染色一起装，本进程只装一次）。
#[cfg(windows)]
pub fn install_main_window_guard(hwnd: windows::Win32::Foundation::HWND) {
  use windows::Win32::UI::Shell::SetWindowSubclass;
  let ok = unsafe { SetWindowSubclass(hwnd, Some(main_window_guard), 1, 0) };
  log::info!("[window_fx] 主窗口守卫 subclass（失焦欺骗）安装 = {ok:?}");
}

/// 常态边框色（COLORREF 0x00BBGGRR）= 通用/六级 --n-g6 #333——实色模式前端自绘
/// 边框同色，全窗统一。
pub const BORDER_COLOR_NORMAL: u32 = 0x0033_3333;

/// 窗口边框显式染色（2026-09-29 修复「边框线没了」）：**Win11 25H2（26200）起 DWM
/// 默认边框（DWMWA_COLOR_NONE）不再绘制**——实机剖线实证窗外→窗内像素直接突变无过渡，
/// 显式染色则 DWM 正常画出 2 物理 px 线（25H2 实测）。故「DWM 默认半透明边框」口径
/// （2026-09-06 六轮定）在 25H2 失效，回到显式染色（同 0830 方案，色仍 #333）。
/// 选择模式进出由 set_main_border_picking 切染品牌色 ↔ 本函数染回常态。
#[cfg(windows)]
pub fn paint_border(hwnd: windows::Win32::Foundation::HWND) {
  use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_BORDER_COLOR};
  let color: u32 = BORDER_COLOR_NORMAL;
  let r = unsafe {
    DwmSetWindowAttribute(
      hwnd,
      DWMWA_BORDER_COLOR,
      &color as *const _ as *const std::ffi::c_void,
      std::mem::size_of_val(&color) as u32,
    )
  };
  log::info!("[window_fx] DwmSetWindowAttribute(BORDER_COLOR, #333) = {r:?}");
}

/// 非 Windows 占位（主窗口/子窗口调用点不重复 cfg 块）。
#[cfg(not(windows))]
pub fn paint_border(_hwnd: ()) {}

/// 窗口级边框染色封装：hwnd 提取与 cfg 收编（preview / plugin 子窗口调用点不再重复 cfg 块）。
#[cfg(windows)]
pub fn paint_window_border(win: &tauri::WebviewWindow) {
  if let Ok(raw) = win.hwnd() {
    paint_border(windows::Win32::Foundation::HWND(raw.0 as _));
  }
}

/// 非 Windows 占位。
#[cfg(not(windows))]
pub fn paint_window_border(_win: &tauri::WebviewWindow) {}

/// 对窗口应用模式：2026-10-01 起恒 solid（云母已撤，见模块注释）。保留函数与签名
/// 仅因主窗口（lib.rs）与子窗口（open_child_window）共用本调用点。
pub fn apply(_win: &tauri::WebviewWindow) -> &'static str {
  "solid"
}

/// 主窗口启动时调用：记录主模式（前端 get_window_fx_mode 查询依据）。
pub fn apply_main(win: &tauri::WebviewWindow) {
  let _ = MAIN_MODE.set(apply(win));
}

/// 主窗口实际模式（恒 solid，前端基态即实色）。
pub fn main_mode() -> &'static str {
  MAIN_MODE.get().copied().unwrap_or("solid")
}
