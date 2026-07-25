//! Shell 打开适配器：用系统默认程序打开文件（预览窗口"用默认程序打开"）。
//! Windows 走 ShellExecuteW "open"；非 Windows 暂不支持（mac 后续用 NSWorkspace，PRD 未定）。

use std::path::Path;

/// 用系统默认程序打开文件；失败返回中文人话（错误码随附，便于排查）
pub fn open(path: &Path) -> Result<(), String> {
  imp::open(path)
}

#[cfg(windows)]
mod imp {
  use std::path::Path;

  use windows::core::{w, HSTRING, PCWSTR};
  use windows::Win32::UI::Shell::ShellExecuteW;
  use windows::Win32::UI::WindowsAndMessaging::SW_SHOW;

  pub(super) fn open(path: &Path) -> Result<(), String> {
    let hpath = HSTRING::from(path.as_os_str());
    // ShellExecuteW 返回值 > 32 为成功（历史遗留 HINSTANCE 语义），≤ 32 为错误码
    let result = unsafe { ShellExecuteW(None, w!("open"), PCWSTR::from_raw(hpath.as_ptr()), None, None, SW_SHOW) };
    if (result.0 as usize) > 32 {
      Ok(())
    } else {
      Err(format!("系统默认程序打开失败（{}），错误码 {}", path.display(), result.0 as usize))
    }
  }
}

#[cfg(not(windows))]
mod imp {
  use std::path::Path;

  pub(super) fn open(_path: &Path) -> Result<(), String> {
    Err("暂不支持".to_string())
  }
}

#[cfg(all(test, windows))]
mod tests {
  use super::*;

  #[test]
  fn open_missing_file_returns_err() {
    let dir = tempfile::tempdir().unwrap();
    let ghost = dir.path().join("不存在.png");
    // 文件不存在时 ShellExecuteW 返回错误码（≤32），不弹窗、无副作用
    assert!(open(&ghost).is_err());
  }
}
