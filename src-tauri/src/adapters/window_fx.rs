//! 窗口背景效果适配器（阶段七 7.1 十七轮 · 主窗口骨架轮）：
//! Win11 22H2+ 接云母（Mica），其余版本实色回退（前端 --n-g-a1-solid #1F1F1F 实底，
//! 2026-08-30 用户定，不做 Acrylic）。模式经 get_window_fx_mode 命令告知前端切换
//! `.window` 底色；set_effects 要求窗口 transparent（tauri.conf 已配）。
//!
//! 版本判定读注册表 HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\CurrentBuildNumber
//! （REG_SZ 字符串，注册表路径零新依赖；Win11 22H2 = 22621，DWM 系统背景自此才有 Mica）。
//! 读取失败 / 非 Windows = solid 安全回退。

use std::sync::OnceLock;

/// 主窗口实际生效的模式（"mica" / "solid"），启动时 apply_main 写入，前端查询依据。
static MAIN_MODE: OnceLock<&'static str> = OnceLock::new();

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
  match imp::current_build() {
    Some(b) => mode_for_build(b),
    None => "solid",
  }
}

/// 对窗口应用模式：mica 调 set_effects，solid 不动（前端实色底盖住）。
/// 返回实际生效模式（set_effects 失败 = solid 回退）。
pub fn apply(win: &tauri::WebviewWindow) -> &'static str {
  let mode = detect_mode();
  if mode == "mica" {
    let effects = tauri::window::EffectsBuilder::new()
      .effect(tauri::window::Effect::Mica)
      .build();
    if win.set_effects(effects).is_err() {
      return "solid";
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
