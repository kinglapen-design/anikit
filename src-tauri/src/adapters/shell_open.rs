//! Shell 打开适配器：用系统默认程序打开文件（预览窗口"用默认程序打开"）。
//! Windows 走 ShellExecuteW "open"；非 Windows 暂不支持（mac 后续用 NSWorkspace，PRD 未定）。

use std::path::Path;

/// 用系统默认程序打开文件；失败返回中文人话（错误码随附，便于排查）
pub fn open(path: &Path) -> Result<(), String> {
  imp::open(path)
}

/// 其他应用打开（6.3 右键菜单）：Windows"打开方式"对话框
pub fn open_as(path: &Path) -> Result<(), String> {
  imp::open_as(path)
}

/// 在文件管理器中打开并选中（6.3 右键菜单）：explorer /select
pub fn reveal(path: &Path) -> Result<(), String> {
  imp::reveal(path)
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

  pub(super) fn open_as(path: &Path) -> Result<(), String> {
    let hpath = HSTRING::from(path.as_os_str());
    // "openas" 动词 = 系统"打开方式"对话框
    let result = unsafe { ShellExecuteW(None, w!("openas"), PCWSTR::from_raw(hpath.as_ptr()), None, None, SW_SHOW) };
    if (result.0 as usize) > 32 {
      Ok(())
    } else {
      Err(format!("打开方式对话框调用失败（{}），错误码 {}", path.display(), result.0 as usize))
    }
  }

  pub(super) fn reveal(path: &Path) -> Result<(), String> {
    // explorer /select,"路径"：经 ShellExecuteW 传 lpParameters（原样传递）。
    // **不能用 std::process::Command**——arg 含空格 / 引号时 Rust 自动加外层引号并把内层引号
    // 转义为 \"，而 explorer 不按 CommandLineToArgvW 解析（吃原始命令行），转义后路径断 →
    // 回退打开"文档"（2026-08-14 实机两轮踩坑：裸路径空格断 / 手写引号被再转义）
    let params = HSTRING::from(format!("/select,\"{}\"", path.display()));
    let result =
      unsafe { ShellExecuteW(None, w!("open"), w!("explorer.exe"), PCWSTR::from_raw(params.as_ptr()), None, SW_SHOW) };
    if (result.0 as usize) > 32 {
      Ok(())
    } else {
      Err(format!("打开文件管理器失败（{}），错误码 {}", path.display(), result.0 as usize))
    }
  }
}

#[cfg(not(windows))]
mod imp {
  use std::path::Path;

  pub(super) fn open(_path: &Path) -> Result<(), String> {
    Err("暂不支持".to_string())
  }
  pub(super) fn open_as(_path: &Path) -> Result<(), String> {
    Err("暂不支持".to_string())
  }
  pub(super) fn reveal(_path: &Path) -> Result<(), String> {
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
