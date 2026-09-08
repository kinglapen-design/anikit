//! 窗口背景效果适配器（阶段七 7.1 十七轮 · 主窗口骨架轮）：
//! Win11 22H2+ 接云母（Mica），其余版本实色回退（前端 --n-g-a1-solid #1F1F1F 实底，
//! 2026-08-30 用户定，不做 Acrylic）。模式经 get_window_fx_mode 命令告知前端切换
//! `.window` 底色。
//!
//! 窗口配置终态（063 六轮定稿）：`transparent:false + backgroundColor:#00000000 +
//! shadow:true + ExtendFrame(-1)`——tao transparent:true 的 blur-behind 空 region hack
//! 会让含非客户区的整窗表面按 alpha 合成，DWM 圆角描边曲线与方形客户区角之间的月牙
//! 窄缝 alpha=0 成透底洞（用户实机"角部钩形透出下层"，CSS 垫块只及客户区位图够不着）；
//! Mica 由 DWM 在窗口表面底下合成，不需要 tao transparent，只需 WebView2 默认背景透明
//! （backgroundColor:#00000000）；ExtendFrame(-1) 令 DWM 帧/backdrop 填充覆盖非客户区，
//! 全程恒 -1 不按最大化/全屏切换（2026-09-08 二轮根治，见 main_window_guard ②）。
//!
//! 版本判定读注册表 HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\CurrentBuildNumber
//! （REG_SZ 字符串，注册表路径零新依赖；Win11 22H2 = 22621，DWM 系统背景自此才有 Mica）。
//! 读取失败 / 非 Windows = solid 安全回退。

use std::sync::OnceLock;

/// 主窗口实际生效的模式（"mica" / "solid"），启动时 apply_main 写入，前端查询依据。
static MAIN_MODE: OnceLock<&'static str> = OnceLock::new();

/// 主窗口守卫 subclass：
/// ① 失焦欺骗（2026-08-30 用户方案）——失焦时 DWM 把云母退化为实色、窗口阴影收成小圈，
///    两者都绑 DWM 的"窗口激活态"渲染判定；拦截 WM_NCACTIVATE 把 wParam 恒改 TRUE 再下放
///    默认链（lParam -1 = 阻止默认流程重绘非客户区），骗 DWM 恒按激活渲染帧/backdrop/阴影。
///    焦点/activation 本身不变（WM_ACTIVATE、前后台焦点事件不受影响）。
/// ② 最大化 caption 条带归零（2026-09-08 二轮根治，踩坑 071）——DWM 原生按钮画在
///    「WS_MAXIMIZE + WS_CAPTION + 最大化非客户区条带」上：tao 的 WM_NCCALCSIZE 最大化
///    特判把客户区设为 rcWork，与系统默认最大化矩形（rcWork 外扩 frame 厚度）之间留出
///    条带；按钮稳态画进条带，最大化过渡动画期亦按条带绘制——「动画前撤玻璃」一轮
///    方案因此治不好（实机复验动画期照闪）。根治 = 接管两条消息让条带不存在：
///    WM_NCCALCSIZE 最大化时客户区 = 整窗矩形（返回 0 不下放 tao；非最大化照旧下放，
///    shadow insets 边框阴影体系不动）+ WM_GETMINMAXINFO 把最大化矩形限定为 rcWork
///    （先下放让 tao 填 min/max 尺寸约束，再覆盖 ptMaxPosition/ptMaxSize），内容四边
///    不外扩不被裁。玻璃延伸恒 -1 不动——无条带即无按钮，云母最大化保留（Codex/
///    Electron 无边框同路线实机先例）。自动隐藏任务栏的边客户区留 1px（照搬 tao
///    has_edge/SHAppBarMessage 补偿），不挡屏幕边缘触发。
#[cfg(windows)]
unsafe extern "system" fn main_window_guard(
  hwnd: windows::Win32::Foundation::HWND,
  msg: u32,
  wparam: windows::Win32::Foundation::WPARAM,
  lparam: windows::Win32::Foundation::LPARAM,
  _id: usize,
  _data: usize,
) -> windows::Win32::Foundation::LRESULT {
  use windows::Win32::Foundation::{LPARAM, LRESULT, POINT, WPARAM};
  use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
  };
  use windows::Win32::UI::Shell::{
    DefSubclassProc, SHAppBarMessage, ABE_BOTTOM, ABE_LEFT, ABE_RIGHT, ABE_TOP,
    ABM_GETAUTOHIDEBAR, APPBARDATA,
  };
  use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, GWL_STYLE, MINMAXINFO, NCCALCSIZE_PARAMS, WM_GETMINMAXINFO, WM_NCACTIVATE,
    WM_NCCALCSIZE, WS_MAXIMIZE,
  };
  if msg == WM_NCACTIVATE {
    return DefSubclassProc(hwnd, msg, WPARAM(1), LPARAM(-1));
  }
  if msg == WM_NCCALCSIZE && wparam.0 != 0 {
    let maximized = (GetWindowLongPtrW(hwnd, GWL_STYLE) as u32) & WS_MAXIMIZE.0 != 0;
    if maximized {
      // rgrc[0] 输入即提议窗口矩形：保持不动 = 客户区占满整窗（caption 条带归零）；
      // 仅自动隐藏任务栏的边留 1px（照搬 tao 无边框最大化特判的同款补偿）
      fn has_edge(edge: u32) -> bool {
        let mut app_data = APPBARDATA {
          cbSize: std::mem::size_of::<APPBARDATA>() as _,
          uEdge: edge,
          ..Default::default()
        };
        unsafe { SHAppBarMessage(ABM_GETAUTOHIDEBAR, &mut app_data) != 0 }
      }
      let params = &mut *(lparam.0 as *mut NCCALCSIZE_PARAMS);
      if has_edge(ABE_BOTTOM) {
        params.rgrc[0].bottom -= 1;
      }
      if has_edge(ABE_LEFT) {
        params.rgrc[0].left += 1;
      }
      if has_edge(ABE_TOP) {
        params.rgrc[0].top += 1;
      }
      if has_edge(ABE_RIGHT) {
        params.rgrc[0].right -= 1;
      }
      return LRESULT(0);
    }
  }
  if msg == WM_GETMINMAXINFO {
    // 先下放让 tao 填 min/max 尺寸约束（ptMinTrackSize 等），再限定最大化矩形 = 工作区
    let r = DefSubclassProc(hwnd, msg, wparam, lparam);
    let mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
    let mut mi = MONITORINFO {
      cbSize: std::mem::size_of::<MONITORINFO>() as u32,
      ..Default::default()
    };
    if GetMonitorInfoW(mon, &mut mi).as_bool() {
      let mmi = &mut *(lparam.0 as *mut MINMAXINFO);
      mmi.ptMaxPosition = POINT {
        x: mi.rcWork.left,
        y: mi.rcWork.top,
      };
      mmi.ptMaxSize = POINT {
        x: mi.rcWork.right - mi.rcWork.left,
        y: mi.rcWork.bottom - mi.rcWork.top,
      };
    }
    return r;
  }
  DefSubclassProc(hwnd, msg, wparam, lparam)
}

/// 给主窗口装守卫 subclass（show 后随边框染色/帧延伸一起装，本进程只装一次）。
#[cfg(windows)]
pub fn install_main_window_guard(hwnd: windows::Win32::Foundation::HWND) {
  use windows::Win32::UI::Shell::SetWindowSubclass;
  let ok = unsafe { SetWindowSubclass(hwnd, Some(main_window_guard), 1, 0) };
  log::info!("[window_fx] 主窗口守卫 subclass（失焦欺骗 + 最大化条带归零）安装 = {ok:?}");
}

/// 框架延伸整窗（margins -1），启动 show 后调一次、全程不再动（2026-09-08 二轮终局）：
/// -1 = DWM 帧/backdrop 填充覆盖整窗——云母载体（063 六轮定稿配置），DWM 最大化时
/// 亦不关 backdrop，云母最大化保留（Codex 实机先例；微软自家应用最大化同样不关）。
/// 历史一轮方案曾按最大化/全屏态撤玻璃杀原生按钮（踩坑 071：撤玻璃纯黑、摘 caption
/// 白边两轮翻车后的折中），实机复验动画期照闪——按钮画在最大化 caption 条带上而非
/// 玻璃上；根治改由守卫接管 NCCALCSIZE/GETMINMAXINFO 让条带归零（main_window_guard ②），
/// 玻璃不再承担杀按钮职责。
#[cfg(windows)]
pub fn extend_frame(hwnd: windows::Win32::Foundation::HWND) {
  use windows::Win32::Graphics::Dwm::DwmExtendFrameIntoClientArea;
  use windows::Win32::UI::Controls::MARGINS;
  let margins = MARGINS {
    cxLeftWidth: -1,
    cxRightWidth: -1,
    cyTopHeight: -1,
    cyBottomHeight: -1,
  };
  let r = unsafe { DwmExtendFrameIntoClientArea(hwnd, &margins) };
  log::info!("[window_fx] DwmExtendFrameIntoClientArea(-1) = {r:?}");
}

/// build 号 → 模式（纯函数，单测边界）。
pub fn mode_for_build(build: u32) -> &'static str {
  if build >= 22621 {
    "mica"
  } else {
    "solid"
  }
}

/// 判定当前系统模式（读 build 失败 = solid 安全回退）。
pub fn detect_mode() -> &'static str {
  let build = imp::current_build();
  let mode = match build {
    Some(b) => mode_for_build(b),
    None => "solid",
  };
  log::info!("[window_fx] 系统 build={build:?} → 模式 {mode}");
  mode
}

/// 对窗口应用模式：mica 调 set_effects，solid 不动（前端实色底盖住）。
/// 返回实际生效模式（set_effects 失败 = solid 回退）。
/// **必须在窗口 show 后调用**（7.1 十七轮实机实证：show 前 set_effects 会吞掉
/// WebView2 内容只剩云母渐变；窗口显示后再设两者兼容）。
pub fn apply(win: &tauri::WebviewWindow) -> &'static str {
  let mode = detect_mode();
  if mode == "mica" {
    let effects = tauri::window::EffectsBuilder::new()
      .effect(tauri::window::Effect::Mica)
      .build();
    match win.set_effects(effects) {
      Ok(()) => log::info!("[window_fx] 云母 Mica set_effects 成功"),
      Err(e) => {
        log::error!("[window_fx] set_effects(Mica) 失败：{e}——回退 solid");
        return "solid";
      }
    }
  }
  mode
}

/// 主窗口启动时调用：apply + 记录主模式（前端 get_window_fx_mode 查询依据）。
pub fn apply_main(win: &tauri::WebviewWindow) {
  let _ = MAIN_MODE.set(apply(win));
}

/// 主窗口实际模式（默认 solid = 安全观感，前端基态即实色，防首帧半透明闪）。
pub fn main_mode() -> &'static str {
  MAIN_MODE.get().copied().unwrap_or("solid")
}

#[cfg(windows)]
mod imp {
  use windows::core::w;
  use windows::Win32::System::Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ};

  /// CurrentBuildNumber 是 REG_SZ 字符串（如 "22631"）；读取 / 解析失败返回 None。
  pub(super) fn current_build() -> Option<u32> {
    let mut buf = [0u16; 16];
    let mut byte_len = (buf.len() * 2) as u32;
    let r = unsafe {
      RegGetValueW(
        HKEY_LOCAL_MACHINE,
        w!("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion"),
        w!("CurrentBuildNumber"),
        RRF_RT_REG_SZ,
        None,
        Some(buf.as_mut_ptr() as *mut _),
        Some(&mut byte_len),
      )
    };
    if r.is_err() {
      return None;
    }
    let chars = (byte_len as usize / 2).min(buf.len());
    // 含终止 NUL 则去掉再解析
    let slice = &buf[..chars];
    let end = slice.iter().position(|&c| c == 0).unwrap_or(slice.len());
    String::from_utf16_lossy(&slice[..end]).trim().parse().ok()
  }
}

#[cfg(not(windows))]
mod imp {
  pub(super) fn current_build() -> Option<u32> {
    None
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn build_threshold() {
    assert_eq!(mode_for_build(22620), "solid");
    assert_eq!(mode_for_build(22621), "mica"); // Win11 22H2 起
    assert_eq!(mode_for_build(22631), "mica");
    assert_eq!(mode_for_build(19045), "solid"); // Win10 22H2
    assert_eq!(mode_for_build(0), "solid");
  }

  #[test]
  fn detect_returns_valid_mode() {
    let m = detect_mode();
    assert!(m == "mica" || m == "solid");
  }
}
