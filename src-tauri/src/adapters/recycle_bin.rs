//! 系统回收站删除适配器（PRD 02 步骤 2）：Windows 走 SHFileOperationW（FO_DELETE +
//! FOF_ALLOWUNDO 语义），文件进系统回收站、不进库内回收站（PRD 06 已定）；
//! 多路径一次调用；失败返回中文人话。非 Windows 暂不支持（PRD 目标平台 Windows 优先）。

use std::path::PathBuf;

/// 删除到系统回收站；任一路径失败/中止返回中文错（部分成功时未成功的路径不可知，
/// 调用方按整批失败处理并刷新实况）
pub fn delete_to_recycle(paths: &[PathBuf]) -> Result<(), String> {
  imp::delete(paths)
}

#[cfg(windows)]
mod imp {
  use std::path::PathBuf;

  use windows::core::{BOOL, PCWSTR};
  use windows::Win32::UI::Shell::{SHFileOperationW, FO_DELETE, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT, SHFILEOPSTRUCTW};

  use crate::adapters::shell_path::to_shell_string;

  pub(super) fn delete(paths: &[PathBuf]) -> Result<(), String> {
    // pFrom = 双 null 结尾列表：多路径以单个 null 分隔，列表末尾再补一个 null
    let mut from: Vec<u16> = Vec::new();
    for p in paths {
      let s = to_shell_string(p);
      from.extend(s.encode_utf16());
      from.push(0);
    }
    from.push(0);
    let mut op = SHFILEOPSTRUCTW {
      hwnd: windows::Win32::Foundation::HWND::default(),
      wFunc: FO_DELETE,
      pFrom: PCWSTR::from_raw(from.as_ptr()),
      pTo: PCWSTR::null(),
      fFlags: (FOF_ALLOWUNDO | FOF_SILENT | FOF_NOCONFIRMATION | FOF_NOERRORUI).0 as u16,
      fAnyOperationsAborted: BOOL(0),
      hNameMappings: std::ptr::null_mut(),
      lpszProgressTitle: PCWSTR::null(),
    };
    // 返回值非 0 为错误码（GetLastError 语义）；FOF_ALLOWUNDO 使删除进回收站
    let code = unsafe { SHFileOperationW(&raw mut op) };
    if code != 0 {
      return Err(format!("删除到回收站失败（错误码 {code}），文件可能正被占用"));
    }
    if op.fAnyOperationsAborted.as_bool() {
      return Err("删除操作被系统中止".to_string());
    }
    Ok(())
  }
}

#[cfg(not(windows))]
mod imp {
  use std::path::PathBuf;

  pub(super) fn delete(_paths: &[PathBuf]) -> Result<(), String> {
    Err("当前平台不支持回收站删除（Windows 优先，PRD 02）".to_string())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 空清单不碰 shell（也避免在测试机上污染回收站）：直接成功，无操作
  #[test]
  fn empty_list_is_noop() {
    assert!(delete_to_recycle(&[]).is_ok());
  }

  /// 不存在的路径：Windows 下 SHFileOperationW 弹不出 UI（FOF_NOERRORUI）但会返回非 0 /
  /// 置中止位——两种失败形态都要兜成中文错，不许 panic
  #[cfg(windows)]
  #[test]
  fn missing_path_errors_in_chinese() {
    let ghost = std::env::temp_dir().join("anikit-recycle-test-不存在的文件-7834e2b1.txt");
    let result = delete_to_recycle(&[ghost]);
    assert!(result.is_err());
    let msg = result.unwrap_err();
    assert!(!msg.is_empty() && msg.chars().any(|c| '\u{4e00}' <= c && c <= '\u{9fff}'), "{msg}");
  }
}
