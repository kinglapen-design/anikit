//! 收件箱盘符适配器（PRD 05-01）：把库内 inbox/ 挂载为盘符，任意软件"另存为"对话框
//! 直接可见。Windows 用 DefineDosDeviceW（subst 命令同款机制，免解析 GBK 控制台输出）；
//! 非 Windows 暂不支持（mac 用符号链接 + Finder 收藏替代，适配时补）。
//!
//! 挂载为会话级 MS-DOS 设备名，无需管理员；进程崩溃不残留（重启即失效），
//! 正常退出由 services::inbox 负责卸载。

use std::path::Path;

/// 盘符候选顺序：Z→E 倒序挑不常用字母（2026-10-13 改定：A / B 移出候选——软盘位
/// 有软件对 A: 路径报错，用户实机踩坑；已挂载指向同目录的盘符仍正常复用，只影响新挂载挑选）
const CANDIDATES: &[char] = &[
  'Z', 'Y', 'X', 'W', 'V', 'U', 'T', 'S', 'R', 'Q', 'P', 'O', 'N', 'M', 'L', 'K', 'J', 'I', 'H', 'G', 'F', 'E',
];

/// 挂载 dir 为盘符，返回盘符字母；已挂载指向同一目录则复用该盘符（幂等）
#[cfg(target_os = "windows")]
pub fn mount(dir: &Path) -> Result<char, String> {
  if let Some(letter) = mounted_letter(dir) {
    return Ok(letter);
  }
  let letter = free_letter().ok_or_else(|| "没有可用盘符（Z–E 均被占用）".to_string())?;
  define(letter, &raw_target(dir)).map_err(|e| format!("盘符 {letter}: 挂载失败：{e}"))?;
  Ok(letter)
}

#[cfg(not(target_os = "windows"))]
pub fn mount(_dir: &Path) -> Result<char, String> {
  Err("当前平台暂不支持收件箱盘符（mac 适配时补符号链接方案）".to_string())
}

/// 卸载盘符；未挂载（用户手动卸过 / 重启失效）视为成功（幂等）
#[cfg(target_os = "windows")]
pub fn unmount(letter: char) -> Result<(), String> {
  let Some(target) = query(letter) else { return Ok(()) };
  remove(letter, &target).map_err(|e| format!("盘符 {letter}: 卸载失败：{e}"))
}

#[cfg(not(target_os = "windows"))]
pub fn unmount(_letter: char) -> Result<(), String> {
  Ok(())
}

/// 当前挂载指向 dir 的盘符（托盘勾选态与重挂复用判定用）；未挂载 None
pub fn mounted_letter(dir: &Path) -> Option<char> {
  let want = normalize(&raw_target(dir));
  ('A'..='Z').find(|&l| query(l).map(|t| normalize(&t) == want).unwrap_or(false))
}

/// 第一个空闲盘符（QueryDosDevice 查不到即空闲，物理盘 / 网络盘一并排除）
#[cfg(target_os = "windows")]
fn free_letter() -> Option<char> {
  CANDIDATES.iter().copied().find(|&l| query(l).is_none())
}

/// DOS 设备名查询：返回 "\??\E:\path" 形式的 raw target；未定义 None
#[cfg(target_os = "windows")]
fn query(letter: char) -> Option<String> {
  use windows::core::PCWSTR;
  use windows::Win32::Storage::FileSystem::QueryDosDeviceW;
  let name = to_wide(&format!("{letter}:"));
  let mut buf = [0u16; 1024];
  let n = unsafe { QueryDosDeviceW(PCWSTR(name.as_ptr()), Some(&mut buf)) };
  if n == 0 {
    return None;
  }
  Some(String::from_utf16_lossy(&buf[..n as usize]).trim_end_matches('\0').to_string())
}

/// 建立盘符映射（DDD_RAW_TARGET_PATH：target 传 "\??\" 前缀的原始路径）
#[cfg(target_os = "windows")]
fn define(letter: char, target: &str) -> windows::core::Result<()> {
  use windows::core::PCWSTR;
  use windows::Win32::Storage::FileSystem::{DefineDosDeviceW, DDD_RAW_TARGET_PATH};
  let name = to_wide(&format!("{letter}:"));
  let target = to_wide(target);
  unsafe { DefineDosDeviceW(DDD_RAW_TARGET_PATH, PCWSTR(name.as_ptr()), PCWSTR(target.as_ptr())) }
}

/// 移除盘符映射（移除需带原 target 精确匹配）
#[cfg(target_os = "windows")]
fn remove(letter: char, target: &str) -> windows::core::Result<()> {
  use windows::core::PCWSTR;
  use windows::Win32::Storage::FileSystem::{DefineDosDeviceW, DDD_RAW_TARGET_PATH, DDD_REMOVE_DEFINITION};
  let name = to_wide(&format!("{letter}:"));
  let target = to_wide(target);
  unsafe { DefineDosDeviceW(DDD_RAW_TARGET_PATH | DDD_REMOVE_DEFINITION, PCWSTR(name.as_ptr()), PCWSTR(target.as_ptr())) }
}

/// 路径转 raw target：`\??\` 前缀 + 反斜杠 + 去尾部分隔符（与 QueryDosDevice 返回同形）
fn raw_target(dir: &Path) -> String {
  let s = dir.to_string_lossy().replace('/', "\\");
  format!("\\??\\{}", s.trim_end_matches('\\'))
}

/// 比较归一化：全小写（Windows 路径大小写不敏感）
fn normalize(raw: &str) -> String {
  raw.to_lowercase()
}

/// UTF-16 宽字符（含 NUL 结尾，Win32 字符串惯例）
fn to_wide(s: &str) -> Vec<u16> {
  s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::path::PathBuf;

  #[test]
  fn raw_target_normalizes_separators_and_trailing_slash() {
    let dir = PathBuf::from("E:/素材库/inbox/");
    assert_eq!(raw_target(&dir), "\\??\\E:\\素材库\\inbox");
  }

  #[test]
  fn normalize_is_case_insensitive() {
    assert_eq!(normalize("\\??\\E:\\Foo"), normalize("\\??\\e:\\foo"));
    assert_ne!(normalize("\\??\\E:\\Foo"), normalize("\\??\\E:\\Bar"));
  }

  /// 候选序（2026-10-13 改定）：Z 起倒序、E 收尾；A / B 软盘位移出候选（有软件对 A: 报错）
  #[test]
  fn candidates_start_at_z_and_exclude_floppy_letters() {
    assert_eq!(CANDIDATES.first(), Some(&'Z'), "Z 起倒序优先");
    assert_eq!(CANDIDATES.last(), Some(&'E'));
    assert!(!CANDIDATES.contains(&'A') && !CANDIDATES.contains(&'B'), "A / B 不作候选");
    // 22 个字母连续倒序无重复（Z..=E）
    assert_eq!(CANDIDATES.len(), 22);
    let mut sorted = CANDIDATES.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), 22, "候选无重复");
  }
}
