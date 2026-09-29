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
//! 全程恒 -1 不按最大化/全屏切换（2026-09-08 三轮终局，见 main_window_guard ②）。
//!
//! 版本判定读注册表 HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\CurrentBuildNumber
//! （REG_SZ 字符串，注册表路径零新依赖；Win11 22H2 = 22621，DWM 系统背景自此才有 Mica）。
//! 读取失败 / 非 Windows = solid 安全回退。

use std::sync::OnceLock;

/// 主窗口实际生效的模式（"mica" / "solid"），启动时 apply_main 写入，前端查询依据。
static MAIN_MODE: OnceLock<&'static str> = OnceLock::new();

/// 当前态同步 caption 样式（幂等；2026-09-08 三轮终局核心，踩坑 071）：
/// DWM 原生按钮的必要条件 = WS_CAPTION（071 翻车②实证：常态摘 caption 按钮全灭）；
/// 而白边翻车的两个根因——DWM 不画边框后 tao shadow insets 非客户区露白、阴影/圆角
/// 体系塌——都是**常态**属性：最大化态 DWM 本就不画阴影/圆角/边框，最大化矩形的
/// 非客户区条带又在屏幕可视区外（rcWork 外扩 frame 藏屏缘后），**只在最大化摘
/// caption 零视觉损失，还原加回常态体系完整**；玻璃恒 -1 不动，云母（含最大化）
/// 保留。历轮证伪：一轮撤玻璃（按钮不依附玻璃，动画期照闪）、二轮 NCCALCSIZE 归零
/// 条带（按钮不依附条带，实机按钮仍在 + 布局回归），按钮只依附 caption 样式本身。
/// WS_POPUP（fullscreen borderless 等 popup 路径，本就无 caption）不干预，防进出
/// 全屏被误加回。判定用 WS_MAXIMIZE 样式位（不用几何预判——手动拉满屏的非最大化
/// 窗口不能误摘）。
#[cfg(windows)]
unsafe fn sync_caption_for_state(hwnd: windows::Win32::Foundation::HWND) {
  use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, SetWindowLongPtrW, GWL_STYLE, WS_CAPTION, WS_MAXIMIZE, WS_POPUP,
  };
  let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
  if style & (WS_POPUP.0 as isize) != 0 {
    return;
  }
  let want = if style & (WS_MAXIMIZE.0 as isize) != 0 {
    style & !(WS_CAPTION.0 as isize)
  } else {
    style | (WS_CAPTION.0 as isize)
  };
  if want != style {
    SetWindowLongPtrW(hwnd, GWL_STYLE, want);
  }
}

/// 主窗口守卫 subclass：
/// ① 失焦欺骗（2026-08-30 用户方案）——失焦时 DWM 把云母退化为实色、窗口阴影收成小圈，
///    两者都绑 DWM 的"窗口激活态"渲染判定；拦截 WM_NCACTIVATE 把 wParam 恒改 TRUE 再下放
///    默认链（lParam -1 = 阻止默认流程重绘非客户区），骗 DWM 恒按激活渲染帧/backdrop/阴影。
///    焦点/activation 本身不变（WM_ACTIVATE、前后台焦点事件不受影响）。
/// ② 最大化摘 caption、还原加回（2026-09-08 三轮终局，原理见 sync_caption_for_state）——
///    三个挂点统一走幂等 sync：WM_WINDOWPOSCHANGING = 最大化/还原的 SetWindowPos 动画
///    **启动前**（WS_MAXIMIZE 样式位此时已是目标态；WM_SIZE 类事后通知太晚，动画期
///    caption 还在会照画按钮——一轮"动画前撤玻璃"同病）、WM_NCCALCSIZE 紧随其后双保险、
///    WM_STYLECHANGED = tao apply_diff 运行期整体重写 GWL_STYLE（setFullscreen/置顶/
///    setResizable 等路径）带回 caption 的复检点（sync 幂等，改样式触发的 STYLECHANGED
///    重入一次即终止，e793efe 同款机制）。样式 SetWindowLongPtr 直改即可——最大化的
///    SetWindowPos 随后自重算 frame（NCCALCSIZE 紧接），样式同帧生效无中间帧，无需
///    SWP_FRAMECHANGED 嵌套 SetWindowPos（重入风险）。
/// ③ 最大化矩形约束回 rcWork（2026-09-17，实机走查「全屏后中间面板纵向没填满」根治）——
///    ②摘掉 caption 后 Windows 按「无 caption 窗最大化 = 盖满整屏」布窗（盖住任务栏），
///    而客户区仍按 rcWork 排布 → 窗口底部多出一条任务栏高的死带（任务栏被自家窗口挡住）。
///    修法 = WM_GETMINMAXINFO 时把 ptMaxPosition/ptMaxSize 约束到当前显示器的 rcWork
///    （带 caption 时 Windows 默认即此值，覆盖无行为变化；WS_POPUP 全屏路径跳过不干预）。
///    注意必须在 DefSubclassProc **之后**覆写——tao 处理同一消息（最小宽高等），
///    先链后写才是最终值。
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
  use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
  };
  use windows::Win32::UI::Shell::DefSubclassProc;
  use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, GWL_STYLE, MINMAXINFO, WM_GETMINMAXINFO, WM_NCACTIVATE, WM_NCCALCSIZE,
    WM_STYLECHANGED, WM_WINDOWPOSCHANGING, WS_POPUP,
  };
  if msg == WM_NCACTIVATE {
    return DefSubclassProc(hwnd, msg, WPARAM(1), LPARAM(-1));
  }
  if msg == WM_GETMINMAXINFO {
    // 先链（tao 可能写 ptMinSize 等），后覆写最大化矩形 = rcWork（见注释③）
    let r = DefSubclassProc(hwnd, msg, wparam, lparam);
    let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
    if style & (WS_POPUP.0 as isize) == 0 {
      let mmi = lparam.0 as *mut MINMAXINFO;
      if !mmi.is_null() {
        let mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO {
          cbSize: std::mem::size_of::<MONITORINFO>() as u32,
          ..Default::default()
        };
        if GetMonitorInfoW(mon, &mut mi).as_bool() {
          let m = unsafe { &mut *mmi };
          m.ptMaxPosition.x = mi.rcWork.left - mi.rcMonitor.left;
          m.ptMaxPosition.y = mi.rcWork.top - mi.rcMonitor.top;
          m.ptMaxSize.x = mi.rcWork.right - mi.rcWork.left;
          m.ptMaxSize.y = mi.rcWork.bottom - mi.rcWork.top;
        }
      }
    }
    return r;
  }
  if msg == WM_WINDOWPOSCHANGING || msg == WM_NCCALCSIZE || msg == WM_STYLECHANGED {
    sync_caption_for_state(hwnd);
  }
  DefSubclassProc(hwnd, msg, wparam, lparam)
}

/// 给主窗口装守卫 subclass（show 后随边框染色/帧延伸一起装，本进程只装一次）。
/// 装完立刻 sync 一次：窗口几何恢复（lib.rs）发生在守卫安装之前，启动即最大化的
/// 场景 caption 摘取靠不到消息驱动，按当前态补一刀。
#[cfg(windows)]
pub fn install_main_window_guard(hwnd: windows::Win32::Foundation::HWND) {
  use windows::Win32::UI::Shell::SetWindowSubclass;
  let ok = unsafe { SetWindowSubclass(hwnd, Some(main_window_guard), 1, 0) };
  unsafe { sync_caption_for_state(hwnd) };
  log::info!("[window_fx] 主窗口守卫 subclass（失焦欺骗 + 最大化摘 caption）安装 = {ok:?}");
}

/// 框架延伸整窗（margins -1），启动 show 后调一次、全程不再动（2026-09-08 三轮终局）：
/// -1 = DWM 帧/backdrop 填充覆盖整窗——云母载体（063 六轮定稿配置），DWM 最大化时
/// 亦不关 backdrop，云母最大化保留（Codex 实机先例；微软自家应用最大化同样不关）。
/// 历史：一轮按最大化/全屏态撤玻璃杀原生按钮（踩坑 071，动画期照闪）、二轮守卫接管
/// NCCALCSIZE/GETMINMAXINFO 归零条带（按钮不依附条带，实机证伪 + 布局回归）——三轮
/// 起杀按钮职责归「最大化摘 caption」（sync_caption_for_state），玻璃不再承担。
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

/// 常态边框色（COLORREF 0x00BBGGRR）= 通用/六级 --n-g6 #333——与实色模式前端自绘
/// 边框同色，云母/实色两模式全窗统一。
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
/// WebView2 内容只剩云母渐变；热切换不吞；透明窗口初始化期的 DWM 背景材料与 webview
/// 合成冲突，窗口显示后再设则两者兼容）。
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
