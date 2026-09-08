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
//! （backgroundColor:#00000000）；ExtendFrame(-1) 令 DWM 帧/backdrop 填充覆盖非客户区。
//!
//! 版本判定读注册表 HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\CurrentBuildNumber
//! （REG_SZ 字符串，注册表路径零新依赖；Win11 22H2 = 22621，DWM 系统背景自此才有 Mica）。
//! 读取失败 / 非 Windows = solid 安全回退。

use std::sync::OnceLock;

/// 主窗口实际生效的模式（"mica" / "solid"），启动时 apply_main 写入，前端查询依据。
static MAIN_MODE: OnceLock<&'static str> = OnceLock::new();

/// 玻璃当前是否已撤（true = margins 0，最大化/全屏态）——extend_frame 唯一写点，
/// 守卫 subclass 读（防 WM_WINDOWPOSCHANGING 预撤与 Resized 补撤重复调 DWM）
#[cfg(windows)]
static FRAME_GLASS_OFF: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// 主窗口守卫 subclass：① 失焦欺骗（2026-08-30 用户方案）——失焦时 DWM 把云母退化为
/// 实色、窗口阴影收成小圈，两者都绑 DWM 的"窗口激活态"渲染判定；拦截 WM_NCACTIVATE
/// 把 wParam 恒改 TRUE 再下放默认链（lParam -1 = 阻止默认流程重绘非客户区），骗 DWM
/// 恒按激活渲染帧/backdrop/阴影。焦点/activation 本身不变（WM_ACTIVATE、前后台焦点
/// 事件不受影响）。② 最大化动画防闪（2026-09-08 实机走查 17 复验：动画里闪 DWM 原生
/// 按钮）——Resized 事件在最大化动画结束后才到，动画期窗口仍是玻璃+caption，DWM 把
/// 按钮画进动画；拦 WM_WINDOWPOSCHANGING，新几何覆盖工作区（最大化/snap 置顶/Win+Up/
/// tauri toggleMaximize 的 ShowWindow 全过此消息）且玻璃未撤时，动画开始前先撤玻璃。
/// 还原方向不预撤：动画期保持无玻璃（无按钮可画），完成后由 Resized 处理器回 -1。
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
  use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST};
  use windows::Win32::UI::Shell::DefSubclassProc;
  use windows::Win32::UI::WindowsAndMessaging::{SWP_NOSIZE, WINDOWPOS, WM_NCACTIVATE, WM_WINDOWPOSCHANGING};
  if msg == WM_NCACTIVATE {
    return DefSubclassProc(hwnd, msg, WPARAM(1), LPARAM(-1));
  }
  if msg == WM_WINDOWPOSCHANGING {
    let wp = &*(lparam.0 as *const WINDOWPOS);
    if (wp.flags & SWP_NOSIZE).0 == 0
      && !FRAME_GLASS_OFF.load(std::sync::atomic::Ordering::Relaxed)
    {
      let mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
      let mut mi = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
      };
      if GetMonitorInfoW(mon, &mut mi).as_bool() {
        let (ww, wh) = (mi.rcWork.right - mi.rcWork.left, mi.rcWork.bottom - mi.rcWork.top);
        if wp.cx >= ww && wp.cy >= wh {
          extend_frame(hwnd, true);
        }
      }
    }
  }
  DefSubclassProc(hwnd, msg, wparam, lparam)
}

/// 给主窗口装守卫 subclass（show 后随边框染色/帧延伸一起装，本进程只装一次）。
#[cfg(windows)]
pub fn install_main_window_guard(hwnd: windows::Win32::Foundation::HWND) {
  use windows::Win32::UI::Shell::SetWindowSubclass;
  let ok = unsafe { SetWindowSubclass(hwnd, Some(main_window_guard), 1, 0) };
  log::info!("[window_fx] 主窗口守卫 subclass（失焦欺骗 + 最大化动画防闪）安装 = {ok:?}");
}

/// 框架延伸按最大化/全屏态切换（2026-09-08 实机走查第 17 条，三轮实证定稿）：
/// 原生窗口按钮 = DWM 在「整窗玻璃 + WS_CAPTION」上画的——最大化时 tao 把客户区调成
/// 工作区（让出任务栏），顶部露出 caption 高框架条带，按钮画进条带压自定义按钮。
/// 玻璃与 caption 都是承重墙（玻璃扛云母、caption 扛边框阴影，摘/撤各自翻车实录见
/// 踩坑 071），唯一实证可行解 = **最大化/全屏态撤玻璃**（margins 0：DWM 无玻璃可画
/// 按钮；DWM 此时亦关掉云母 backdrop，前端 .window.fx-mica.maximized 同步回落实色
/// --n-g-a1-solid——微软自家应用最大化关云母同惯例），还原回 -1。常态一律 -1 不动。
/// 三个调用点：启动初始态 / Resized 完成后 / 守卫 WINDOWPOSCHANGING 动画前预撤。
#[cfg(windows)]
pub fn extend_frame(hwnd: windows::Win32::Foundation::HWND, maximized: bool) {
  use windows::Win32::Graphics::Dwm::DwmExtendFrameIntoClientArea;
  use windows::Win32::UI::Controls::MARGINS;
  FRAME_GLASS_OFF.store(maximized, std::sync::atomic::Ordering::Relaxed);
  let m = if maximized { 0 } else { -1 };
  let margins = MARGINS {
    cxLeftWidth: m,
    cxRightWidth: m,
    cyTopHeight: m,
    cyBottomHeight: m,
  };
  let r = unsafe { DwmExtendFrameIntoClientArea(hwnd, &margins) };
  log::info!("[window_fx] DwmExtendFrameIntoClientArea({m}) maximized={maximized} = {r:?}");
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
