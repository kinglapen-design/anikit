//! 窗口背景效果适配器（2026-10-01 终局转向：弃云母，换回最大化过渡动画）。
//!
//! 终局配置：`transparent:false + backgroundColor:#1F1F1F（不透明实底）+ shadow:true`——
//! 无云母、无整窗玻璃（`DwmExtendFrameIntoClientArea(-1)` 已撤，它本是云母载体）、
//! `WS_CAPTION` 全程保留（最大化不再摘）。DWM 最大化/还原过渡动画依附 caption
//! （Electron/VSCode 无边框先例 = 全程保 caption + NCCALCSIZE 归零 = 有动画、
//! 无系统按钮穿帮）；而 DWM 原生按钮的画布是整窗玻璃（踩坑 071 一轮实证：
//! 最大化态撤玻璃 → 稳态按钮没了）——撤玻璃后 caption 无需再摘，
//! 动画与「无原生按钮」兼得。底色改不透明后 WebView2 不再依赖 DWM backdrop，
//! 前端基态实色（--n-g-a1-solid #1F1F1F）单态——fx 双态机制（get_window_fx_mode /
//! fx-mica）2026-10-02 已整层拆除。
//!
//! 历史（踩坑 071 全录）：7.1 十七轮接云母（Win11 22H2+ Mica，063 六轮定稿
//! `transparent:false + ExtendFrame(-1)` 玻璃载体 + set_effects）→ 玻璃 + tao 保留的
//! WS_CAPTION 让 DWM 把原生按钮画进窗内（最大化稳态 + 动画期 + 常态顶边细条三态穿帮）
//! → 2026-09-08 三轮终局「WM_WINDOWPOSCHANGING 动画前摘 caption、还原加回」杀按钮，
//! 连带伤亡 = 动画期 DWM 面对无 caption 窗口不播过渡动画。2026-10-01 用户定案：
//! 动画优先于云母，删云母路线（撤 Mica + 撤玻璃 + caption 全程保留）。

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
