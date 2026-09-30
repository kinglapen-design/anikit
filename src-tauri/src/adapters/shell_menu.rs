//! Windows 原生（系统）右键菜单适配器：工作空间「系统菜单」项（压缩 / 解压 / TortoiseGit 等
//! shell 扩展全量出现——7-Zip 系扩展注册的是经典 IContextMenu 动词，本实现即经典菜单；
//! Win11 新样式菜单的 IExplorerCommand 现代扩展不可托管，不在其中）。
//! 桌面 IShellFolder + 全 PIDL 的 GetUIObjectOf：跨目录多选同 Explorer 口径天然成立。
//! 消息转发：7-Zip 等扩展的子菜单与自绘项依赖 IContextMenu2/3 消息回填——菜单挂在自建
//! 消息窗上，WndProc 把 WM_INITMENUPOPUP / WM_DRAWITEM / WM_MEASUREITEM / WM_MENUSELECT /
//! WM_MENUCHAR 转发给 HandleMenuMsg(2)，不转发则子菜单不展开、自绘项空白。
//! 线程纪律：须 STA 线程（命令层 spawn_blocking 内调用，本模块自行 CoInitializeEx）；
//! TrackPopupMenuEx 模态循环在本线程自泵消息，主线程不被占用（webview 不冻结）。
//! 非 Windows 暂不支持（PRD 未定 macOS 口径）。

use std::path::PathBuf;

/// 在光标处弹出系统右键菜单并执行选中命令；用户取消 = Ok(())；失败返回中文人话
pub fn show(paths: &[PathBuf]) -> Result<(), String> {
  imp::show(paths)
}

#[cfg(windows)]
mod imp {
  use std::path::PathBuf;
  use std::ptr::null_mut;

  use windows::core::{w, Interface, PCSTR, PCWSTR, HSTRING};
  use windows::Win32::Foundation::{ERROR_CLASS_ALREADY_EXISTS, GetLastError, HWND, LPARAM, LRESULT, POINT, WPARAM};
  use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoTaskMemFree, CoUninitialize};
  use windows::Win32::System::LibraryLoader::GetModuleHandleW;
  use windows::Win32::UI::Shell::Common::ITEMIDLIST;
  use windows::Win32::UI::Shell::{
    CMF_NORMAL, CMINVOKECOMMANDINFO, CMINVOKECOMMANDINFOEX, IContextMenu, IContextMenu2, IContextMenu3,
    IShellFolder, SHGetDesktopFolder, SHParseDisplayName,
  };
  use windows::Win32::UI::WindowsAndMessaging::{
    CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow, GetCursorPos, GetWindowLongPtrW, PostMessageW,
    RegisterClassExW, SetForegroundWindow, SetWindowLongPtrW, TrackPopupMenuEx, GWLP_USERDATA, HMENU, HWND_MESSAGE, SW_SHOWNORMAL,
    TPM_RETURNCMD, TPM_RIGHTBUTTON, WNDCLASSEXW, WM_DRAWITEM, WM_INITMENUPOPUP, WM_MEASUREITEM, WM_MENUCHAR, WM_MENUSELECT,
    WM_NULL, WINDOW_EX_STYLE, WINDOW_STYLE,
  };

  /// 菜单命令 id 起始（QueryContextMenu 占用区间 [1, 0x7FFF]）
  const ID_CMD_FIRST: u32 = 1;
  /// CMIC_MASK_UNICODE（windows crate 0.62 未收该常量，按 shellapi.h 原值定义）
  const CMIC_MASK_UNICODE: u32 = 0x0000_4000;

  /// 消息转发对象（栈上持有，生命周期覆盖模态菜单全程）：优先 IContextMenu3
  ///（HandleMenuMsg2 多处理 WM_MENUCHAR 键盘加速），无则退 IContextMenu2
  struct Forwarder {
    cm3: Option<IContextMenu3>,
    cm2: Option<IContextMenu2>,
  }

  pub(super) fn show(paths: &[PathBuf]) -> Result<(), String> {
    if paths.is_empty() {
      return Err("没有可操作的条目".to_string());
    }
    unsafe {
      let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
      if hr.is_err() {
        return Err(format!("COM 初始化失败：{}", hr.message()));
      }
      let r = show_sta(paths);
      CoUninitialize();
      r
    }
  }

  /// STA 线程内主流程：全 PIDL 解析 → 菜单构建与模态弹出 → 命令执行；PIDL 泄漏防护统一收尾
  fn show_sta(paths: &[PathBuf]) -> Result<(), String> {
    let mut pidls: Vec<*mut ITEMIDLIST> = Vec::with_capacity(paths.len());
    let mut result = Ok(());
    for p in paths {
      let h = HSTRING::from(p.as_os_str());
      let mut pidl: *mut ITEMIDLIST = null_mut();
      let r = unsafe { SHParseDisplayName(PCWSTR::from_raw(h.as_ptr()), None, &mut pidl, 0, None) };
      if r.is_err() || pidl.is_null() {
        result = Err(format!("路径解析失败：{}", p.display()));
        break;
      }
      pidls.push(pidl);
    }
    if result.is_ok() {
      result = unsafe { popup_and_invoke(&pidls) };
    }
    for pidl in pidls {
      unsafe { CoTaskMemFree(Some(pidl as *const core::ffi::c_void)) };
    }
    result
  }

  /// 菜单构建 → 模态弹出（光标处）→ 选中命令 InvokeCommand；取消 / 空菜单 = Ok
  unsafe fn popup_and_invoke(pidls: &[*mut ITEMIDLIST]) -> Result<(), String> {
    let hwnd = create_forward_window()?;
    let result = menu_at_cursor(hwnd, pidls);
    let _ = DestroyWindow(hwnd);
    result
  }

  unsafe fn menu_at_cursor(hwnd: HWND, pidls: &[*mut ITEMIDLIST]) -> Result<(), String> {
    let desktop: IShellFolder = SHGetDesktopFolder().map_err(|e| format!("Shell 桌面对象获取失败：{e}"))?;
    // 桌面 IShellFolder 的 GetUIObjectOf 直接吃全 PIDL 切片——跨目录多选同 Explorer 口径天然成立
    let apidl: Vec<*const ITEMIDLIST> = pidls.iter().map(|p| *p as *const ITEMIDLIST).collect();
    let cm: IContextMenu = desktop
      .GetUIObjectOf(hwnd, &apidl, None)
      .map_err(|e| format!("右键菜单对象获取失败：{e}"))?;

    let menu = CreatePopupMenu().map_err(|e| format!("菜单创建失败：{e}"))?;
    let result = populate_and_track(hwnd, &cm, menu);
    let _ = DestroyMenu(menu);
    result
  }

  unsafe fn populate_and_track(hwnd: HWND, cm: &IContextMenu, menu: HMENU) -> Result<(), String> {
    let hr = cm.QueryContextMenu(menu, 0, ID_CMD_FIRST, 0x7FFF, CMF_NORMAL);
    if hr.is_err() {
      return Err(format!("菜单项装配失败：{}", hr.message()));
    }
    // 返回码 = 最后使用的命令 id + 1（0/1 = 一项都没装上）
    if hr.0 as u32 <= ID_CMD_FIRST {
      return Err("该条目没有可用的系统菜单项".to_string());
    }

    // 转发对象挂窗口：模态循环期间 WndProc 取回（栈指针，菜单返回后清零再销窗）
    let fwd = Forwarder { cm3: cm.cast::<IContextMenu3>().ok(), cm2: cm.cast::<IContextMenu2>().ok() };
    SetWindowLongPtrW(hwnd, GWLP_USERDATA, &fwd as *const Forwarder as isize);

    let mut pt = POINT::default();
    let _ = GetCursorPos(&mut pt);
    // 标准配对（MSDN 弹窗菜单样本）：前台化保菜单正常失焦关闭，结束后 WM_NULL 收尾
    let _ = SetForegroundWindow(hwnd);
    let cmd = TrackPopupMenuEx(menu, (TPM_RETURNCMD | TPM_RIGHTBUTTON).0, pt.x, pt.y, hwnd, None).0 as u32;
    let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
    SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);

    if cmd == 0 {
      return Ok(()); // 用户取消
    }
    let info = CMINVOKECOMMANDINFOEX {
      cbSize: size_of::<CMINVOKECOMMANDINFOEX>() as u32,
      fMask: CMIC_MASK_UNICODE,
      hwnd,
      lpVerb: PCSTR((cmd - ID_CMD_FIRST) as usize as *const u8),
      nShow: SW_SHOWNORMAL.0,
      ..Default::default()
    };
    cm.InvokeCommand(&info as *const CMINVOKECOMMANDINFOEX as *const CMINVOKECOMMANDINFO)
      .map_err(|e| format!("菜单命令执行失败：{e}"))
  }

  /// 菜单宿主窗（HWND_MESSAGE 消息窗）：唯一职责 = 把菜单消息转发给 IContextMenu2/3
  fn create_forward_window() -> Result<HWND, String> {
    unsafe {
      let hinstance = GetModuleHandleW(None).map_err(|e| format!("模块句柄获取失败：{e}"))?;
      let wc = WNDCLASSEXW {
        cbSize: size_of::<WNDCLASSEXW>() as u32,
        lpfnWndProc: Some(forward_wnd_proc),
        hInstance: hinstance.into(),
        lpszClassName: w!("anikit-shell-menu-forward"),
        ..Default::default()
      };
      if RegisterClassExW(&wc) == 0 && GetLastError() != ERROR_CLASS_ALREADY_EXISTS {
        return Err(format!("菜单窗口类注册失败：{:?}", GetLastError()));
      }
      let hwnd = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        w!("anikit-shell-menu-forward"),
        w!(""),
        WINDOW_STYLE(0),
        0,
        0,
        0,
        0,
        Some(HWND_MESSAGE),
        None,
        Some(hinstance.into()),
        None,
      )
      .map_err(|e| format!("菜单窗口创建失败：{e}"))?;
      Ok(hwnd)
    }
  }

  /// 消息转发 WndProc：仅菜单相关五消息转 IContextMenu2/3（扩展子菜单 / 自绘项靠它），
  /// 其余一律 DefWindowProcW；GWLP_USERDATA 未挂 / 已清零时纯默认处理
  unsafe extern "system" fn forward_wnd_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if matches!(msg, WM_INITMENUPOPUP | WM_DRAWITEM | WM_MEASUREITEM | WM_MENUSELECT | WM_MENUCHAR) {
      let fwd = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Forwarder;
      if let Some(f) = fwd.as_ref() {
        if let Some(cm3) = &f.cm3 {
          let mut res = LRESULT(0);
          let _ = cm3.HandleMenuMsg2(msg, w, l, Some(&mut res));
          return res;
        }
        if let Some(cm2) = &f.cm2 {
          let _ = cm2.HandleMenuMsg(msg, w, l);
          return LRESULT(0);
        }
      }
    }
    DefWindowProcW(hwnd, msg, w, l)
  }

  #[cfg(test)]
  mod tests {
    use super::*;

    /// PIDL 解析：存在文件成功非空、可释放；不存在路径报错
    #[test]
    fn parse_pidls_for_existing_file() {
      let dir = tempfile::tempdir().unwrap();
      let f = dir.path().join("图.png");
      std::fs::write(&f, b"x").unwrap();
      unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let h = HSTRING::from(f.as_os_str());
        let mut pidl: *mut ITEMIDLIST = null_mut();
        let r = SHParseDisplayName(PCWSTR::from_raw(h.as_ptr()), None, &mut pidl, 0, None);
        assert!(r.is_ok() && !pidl.is_null(), "存在文件应解析出 PIDL");
        CoTaskMemFree(Some(pidl as *const core::ffi::c_void));
        let ghost = HSTRING::from(dir.path().join("不存在.png").as_os_str());
        let mut bad: *mut ITEMIDLIST = null_mut();
        let r2 = SHParseDisplayName(PCWSTR::from_raw(ghost.as_ptr()), None, &mut bad, 0, None);
        assert!(r2.is_err(), "不存在路径应解析失败");
        CoUninitialize();
      }
    }
  }
}

#[cfg(not(windows))]
mod imp {
  use std::path::PathBuf;

  pub(super) fn show(_paths: &[PathBuf]) -> Result<(), String> {
    Err("暂不支持".to_string())
  }
}
