//! CEP 插件安装服务（8.1 二轮 2b）：
//! 插件文件复制到用户级 CEP extensions 目录 + PlayerDebugMode 注册表确保（未签名 CEP 扩展
//! 的加载前提，HKCU 写入免管理员）；卸载 = 删除 extensions 下本插件目录（幂等）。
//! 插件源：dev = 项目 plugins/animate-cep；prod = exe 同目录
//! plugins/animate-cep（tauri resources 携带，同 sidecar_path 双候选模式）。

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

/// 插件目录名（CEP extensions 下）与随包版本（manifest ExtensionBundleVersion 同步）
pub const PLUGIN_DIR_NAME: &str = "com.anikit.cep";
pub const BUNDLED_VERSION: &str = "0.4.0";

/// PlayerDebugMode 目标键（CSXS 版本族——Animate 2019+ 覆盖；宽写无害）
const CSXS_KEYS: &[&str] = &["CSXS.9", "CSXS.10", "CSXS.11"];

/// 安装状态（设置页 / 菜单展示）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CepInstallStatus {
  /// 插件源可用（dev 项目目录 / prod resources 任一命中）
  pub source_available: bool,
  /// 已安装（extensions 下 manifest 存在）
  pub installed: bool,
  /// 已装版本（读已装 manifest 的 ExtensionBundleVersion；未读出来为 None）
  pub installed_version: Option<String>,
  /// 随包版本（装/升级的目标）
  pub bundled_version: String,
  /// PlayerDebugMode 全部目标键就绪
  pub debug_mode_ready: bool,
  /// 插件安装目录绝对路径（extensions 下本插件目录；无法定位系统配置目录时为空串）
  pub install_dir: String,
}

/// CEP 用户级 extensions 根（%APPDATA%\Adobe\CEP\extensions）
fn extensions_root() -> Option<PathBuf> {
  Some(dirs::config_dir()?.join("Adobe").join("CEP").join("extensions"))
}

/// 插件源目录：dev = CARGO_MANIFEST_DIR/../plugins/animate-cep；prod = exe 同目录 plugins/animate-cep
fn plugin_source() -> Option<PathBuf> {
  let mut candidates: Vec<PathBuf> = Vec::new();
  let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
  candidates.push(manifest.join("..").join("plugins").join("animate-cep"));
  if let Ok(exe) = std::env::current_exe() {
    if let Some(dir) = exe.parent() {
      candidates.push(dir.join("plugins").join("animate-cep"));
    }
  }
  candidates.into_iter().find(|p| p.join("CSXS").join("manifest.xml").is_file())
}

/// 读 manifest 的 ExtensionBundleVersion（纯文本提取，不引 XML 解析器）
fn manifest_version(manifest: &Path) -> Option<String> {
  let text = fs::read_to_string(manifest).ok()?;
  let key = "ExtensionBundleVersion=\"";
  let start = text.find(key)? + key.len();
  let end = text[start..].find('"')? + start;
  Some(text[start..end].to_string())
}

/// 递归复制目录（目标先删后建，保证纯净覆盖）
fn copy_dir(src: &Path, dest: &Path) -> Result<(), String> {
  if dest.exists() {
    fs::remove_dir_all(dest).map_err(|e| format!("旧版本清理失败（{}）：{e}", dest.display()))?;
  }
  fs::create_dir_all(dest).map_err(|e| format!("目标目录创建失败：{e}"))?;
  copy_inner(src, dest)
}

fn copy_inner(src: &Path, dest: &Path) -> Result<(), String> {
  for entry in fs::read_dir(src).map_err(|e| format!("插件源读取失败：{e}"))? {
    let entry = entry.map_err(|e| e.to_string())?;
    let (s, d) = (entry.path(), dest.join(entry.file_name()));
    if s.is_dir() {
      fs::create_dir_all(&d).map_err(|e| e.to_string())?;
      copy_inner(&s, &d)?;
    } else if entry.file_name() != "install.bat" {
      fs::copy(&s, &d).map_err(|e| format!("复制失败（{}）：{e}", s.display()))?;
    }
  }
  Ok(())
}

pub fn status() -> CepInstallStatus {
  let source = plugin_source();
  let dest = extensions_root().map(|r| r.join(PLUGIN_DIR_NAME));
  let installed_version = dest
    .as_ref()
    .and_then(|d| manifest_version(&d.join("CSXS").join("manifest.xml")));
  CepInstallStatus {
    source_available: source.is_some(),
    installed: installed_version.is_some(),
    installed_version,
    bundled_version: BUNDLED_VERSION.to_string(),
    debug_mode_ready: debug_mode_ready(),
    install_dir: dest.map(|d| d.to_string_lossy().into_owned()).unwrap_or_default(),
  }
}

/// 一键安装：复制插件 → PlayerDebugMode 确保 → 返回最新状态
pub fn install() -> Result<CepInstallStatus, String> {
  let source = plugin_source().ok_or_else(|| "插件源缺失（开发目录或安装包损坏）".to_string())?;
  let root = extensions_root().ok_or_else(|| "无法定位 CEP extensions 目录".to_string())?;
  let dest = root.join(PLUGIN_DIR_NAME);
  copy_dir(&source, &dest)?;
  ensure_debug_mode()?;
  Ok(status())
}

/// 卸载：删除 extensions 下本插件目录（目录不存在 = 幂等成功），返回最新状态
pub fn uninstall() -> Result<CepInstallStatus, String> {
  let root = extensions_root().ok_or_else(|| "无法定位 CEP extensions 目录".to_string())?;
  let dest = root.join(PLUGIN_DIR_NAME);
  if dest.exists() {
    fs::remove_dir_all(&dest).map_err(|e| format!("插件目录删除失败（{}）：{e}", dest.display()))?;
  }
  Ok(status())
}

// —— PlayerDebugMode（HKCU\Software\Adobe\CSXS.*，免管理员）——

#[cfg(windows)]
fn debug_mode_ready() -> bool {
  use windows::core::HSTRING;
  use windows::Win32::System::Registry::{HKEY_CURRENT_USER, KEY_READ, REG_VALUE_TYPE, RegOpenKeyExW, RegQueryValueExW};
  CSXS_KEYS.iter().all(|sub| {
    let path = HSTRING::from(format!("Software\\Adobe\\{sub}"));
    let mut hkey = windows::Win32::System::Registry::HKEY::default();
    let r = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, &path, Some(0), KEY_READ, &mut hkey) };
    if !r.is_ok() {
      return false;
    }
    let name = windows::core::HSTRING::from("PlayerDebugMode");
    let mut ty = REG_VALUE_TYPE::default();
    let mut buf = [0u8; 8];
    let mut len = buf.len() as u32;
    let r2 = unsafe {
      RegQueryValueExW(hkey, &name, None, Some(&mut ty), Some(buf.as_mut_ptr()), Some(&mut len))
    };
    let _ = unsafe { windows::Win32::System::Registry::RegCloseKey(hkey) };
    r2.is_ok() && buf.starts_with(b"1\0")
  })
}

#[cfg(not(windows))]
fn debug_mode_ready() -> bool {
  true // 非 Windows 无此概念（CEP PlayerDebugMode 仅 Windows 注册表语义）
}

#[cfg(windows)]
fn ensure_debug_mode() -> Result<(), String> {
  use windows::core::HSTRING;
  use windows::Win32::System::Registry::{HKEY_CURRENT_USER, KEY_WRITE, REG_SZ, RegCreateKeyExW, RegSetValueExW};
  for sub in CSXS_KEYS {
    let path = HSTRING::from(format!("Software\\Adobe\\{sub}"));
    let mut hkey = windows::Win32::System::Registry::HKEY::default();
    let r = unsafe { RegCreateKeyExW(HKEY_CURRENT_USER, &path, None, None, Default::default(), KEY_WRITE, None, &mut hkey, None) };
    if !r.is_ok() {
      return Err(format!("注册表打开失败（{sub}）：{}", r.0));
    }
    let name = HSTRING::from("PlayerDebugMode");
    let data: Vec<u16> = "1".encode_utf16().chain(std::iter::once(0)).collect();
    let bytes = unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 2) };
    let r2 = unsafe { RegSetValueExW(hkey, &name, None, REG_SZ, Some(bytes)) };
    let _ = unsafe { windows::Win32::System::Registry::RegCloseKey(hkey) };
    if !r2.is_ok() {
      return Err(format!("PlayerDebugMode 写入失败（{sub}）：{}", r2.0));
    }
  }
  Ok(())
}

#[cfg(not(windows))]
fn ensure_debug_mode() -> Result<(), String> {
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn copy_dir_clean_overwrite() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src");
    fs::create_dir_all(src.join("CSXS")).unwrap();
    fs::create_dir_all(src.join("js")).unwrap();
    fs::write(src.join("CSXS").join("manifest.xml"), "<x ExtensionBundleVersion=\"0.2.0\"/>").unwrap();
    fs::write(src.join("js").join("a.js"), "1").unwrap();
    fs::write(src.join("install.bat"), "bat").unwrap();
    let dest = dir.path().join("dest");
    fs::create_dir_all(&dest).unwrap();
    fs::write(dest.join("stale.txt"), "old").unwrap(); // 旧残留应被清
    copy_dir(&src, &dest).unwrap();
    assert!(!dest.join("stale.txt").exists(), "旧文件清除");
    assert!(dest.join("CSXS").join("manifest.xml").is_file());
    assert!(dest.join("js").join("a.js").is_file());
    assert!(!dest.join("install.bat").exists(), "安装脚本不带走");
    assert_eq!(manifest_version(&dest.join("CSXS").join("manifest.xml")).as_deref(), Some("0.2.0"));
  }

  #[test]
  fn plugin_source_finds_dev_dir() {
    // dev 环境（本仓库）应命中项目 plugins/animate-cep
    let src = plugin_source().expect("dev 插件源应存在");
    assert!(src.join("CSXS").join("manifest.xml").is_file());
    assert_eq!(
      manifest_version(&src.join("CSXS").join("manifest.xml")).as_deref(),
      Some(BUNDLED_VERSION),
      "manifest 版本与 BUNDLED_VERSION 同步"
    );
  }
}
