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

/// 在文件管理器中打开（6.3 右键菜单）：目录 = 直接进入本体，文件 = explorer /select 父目录选中
///（目录/文件分流 2026-10-16 用户实机定——此前一律 /select，目录停在父级）
pub fn reveal(path: &Path) -> Result<(), String> {
  imp::reveal(path)
}

/// 打开系统回收站（工作空间侧栏回收站行，2026-10-01；ws 删除走系统回收站，PRD 06）
pub fn open_recycle_bin() -> Result<(), String> {
  imp::open_recycle_bin()
}

/// 打开外部链接（默认浏览器，2026-10-03 工作空间搜索「下载 Everything」指路）：
/// 仅放行 http/https（协议白名单防 file:// 等滥用）
pub fn open_url(url: &str) -> Result<(), String> {
  imp::open_url(url)
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
      return Ok(());
    }
    // 兜底（2026-08-20 用户实机报错误码 31 = SE_ERR_NOASSOC：该文件类型未注册 openas 动词，
    // 动词缺失时系统对话框根本调不起）——rundll32 shell32.dll,OpenAs_RunDLL 不依赖动词注册，
    // 弹出同一个"打开方式"对话框；rundll32 按 CommandLineToArgvW 正常解析参数（与 explorer 不同，
    // std::process::Command 可用，含空格路径由 Rust 自动加引号）
    if open_as_rundll(path).is_ok() {
      return Ok(());
    }
    Err(format!("打开方式对话框调用失败（{}），错误码 {}", path.display(), result.0 as usize))
  }

  /// openas 动词缺失时的兜底：rundll32 调起系统"打开方式"对话框（spawn 即返，不等待对话框关闭）
  fn open_as_rundll(path: &Path) -> Result<(), String> {
    std::process::Command::new("rundll32.exe")
      .arg("shell32.dll,OpenAs_RunDLL")
      .arg(path.as_os_str())
      .spawn()
      .map(|_| ())
      .map_err(|e| format!("rundll32 调起失败：{e}"))
  }

  pub(super) fn reveal(path: &Path) -> Result<(), String> {
    // 目录 / 文件分流（2026-10-16 用户实机定，污染扫描「打开资源管理器」走查）：
    // 目录 = 直接进入本体（ShellExecuteW "open" 开目录 = Explorer 导航进该目录），
    // 文件 = explorer /select 在父目录中选中（文件无法"进入"）。
    if path.is_dir() {
      return open(path);
    }
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

  pub(super) fn open_recycle_bin() -> Result<(), String> {
    // shell:RecycleBinFolder 虚拟路径经 ShellExecuteW 原样传参（同 reveal 口径，不走 Command 转义）
    let params = HSTRING::from("shell:RecycleBinFolder");
    let result =
      unsafe { ShellExecuteW(None, w!("open"), w!("explorer.exe"), PCWSTR::from_raw(params.as_ptr()), None, SW_SHOW) };
    if (result.0 as usize) > 32 {
      Ok(())
    } else {
      Err(format!("打开系统回收站失败，错误码 {}", result.0 as usize))
    }
  }

  pub(super) fn open_url(url: &str) -> Result<(), String> {
    if !url.starts_with("https://") && !url.starts_with("http://") {
      return Err("仅支持 http(s) 链接".to_string());
    }
    let hurl = HSTRING::from(url);
    let result = unsafe { ShellExecuteW(None, w!("open"), PCWSTR::from_raw(hurl.as_ptr()), None, None, SW_SHOW) };
    if (result.0 as usize) > 32 {
      Ok(())
    } else {
      Err(format!("打开链接失败（{url}），错误码 {}", result.0 as usize))
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
  pub(super) fn open_recycle_bin() -> Result<(), String> {
    Err("暂不支持".to_string())
  }
  pub(super) fn open_url(_url: &str) -> Result<(), String> {
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
