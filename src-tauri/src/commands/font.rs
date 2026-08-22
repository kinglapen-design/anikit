//! 命令 API：字体（5.6）。get_font_info = 预览「其它」页元数据 + origin 路径（前端 @font-face 加载）
//! + 系统安装状态；install_font / uninstall_font = 系统级安装 / 卸载（用户级，免管理员——
//! 其他软件立即可见，避免 Eagle"装了识别不到"，2026-08-23 用户痛点定案；实现见 services/font）。
//! 安装 / 卸载只动系统状态（字体目录 + 注册表），不改库数据；锁内定位、锁外系统 IO（不占库锁）。

use serde::Serialize;
use tauri::State;

use crate::commands::library::AppState;
use crate::commands::preview::locate_asset;
use crate::services::font;

/// 字体元数据 + 安装状态（预览「其它」页数据源）
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FontInfo {
  /// origin 绝对路径（前端 FontFace 加载预览字体）
  pub origin_path: String,
  /// 名称（PostScript 名）
  pub name: String,
  /// 全名
  pub full_name: String,
  /// 系列
  pub family: String,
  /// 样式
  pub style: String,
  /// 版本
  pub version: String,
  /// 字数（num_glyphs）
  pub glyph_count: u32,
  /// 是否已装到系统（用户级注册表）
  pub installed: bool,
}

/// 安装 / 卸载结果（前端切换按钮态）
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FontInstallState {
  pub installed: bool,
}

/// 读取字体元数据与安装状态。async：文件 IO + 解析，一律走线程池（规范第 2 条）
#[tauri::command]
pub async fn get_font_info(state: State<'_, AppState>, asset_id: String) -> Result<FontInfo, String> {
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  let (_info_dir, origin, meta) = locate_asset(lib, &asset_id)?;
  if !font::is_font(&origin) {
    return Err(format!("「{}」不是字体资源", meta.name));
  }
  let Some(fm) = font::probe_font(&origin) else {
    return Err("字体文件解析失败".to_string());
  };
  let installed = font::font_installed(&font::font_full_names(&origin));
  Ok(FontInfo {
    origin_path: origin.to_string_lossy().into_owned(),
    name: fm.name,
    full_name: fm.full_name,
    family: fm.family,
    style: fm.style,
    version: fm.version,
    glyph_count: fm.glyph_count,
    installed,
  })
}

/// 安装字体到系统（用户级：复制进用户字体目录 + HKCU 注册表 + 广播）。
/// 锁内定位取路径与全名清单，锁外系统 IO（复制 / 注册表，不占库锁）。
#[tauri::command]
pub async fn install_font(state: State<'_, AppState>, asset_id: String) -> Result<FontInstallState, String> {
  let (origin, full_names) = {
    let guard = state.library.lock().map_err(|e| e.to_string())?;
    let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
    let (_info_dir, origin, meta) = locate_asset(lib, &asset_id)?;
    if !font::is_font(&origin) {
      return Err(format!("「{}」不是字体资源", meta.name));
    }
    let names = font::font_full_names(&origin);
    if names.is_empty() {
      return Err("字体文件解析失败，无法安装".to_string());
    }
    (origin, names)
  };
  font::install_font(&origin, &full_names)?;
  Ok(FontInstallState { installed: true })
}

/// 从系统卸载字体（用户级：注销 + 清注册表 + 删文件；占用时文件保留并提示）。
/// 只认本应用安装口径（HKCU Fonts 项），系统级（HKLM）字体不动。
#[tauri::command]
pub async fn uninstall_font(state: State<'_, AppState>, asset_id: String) -> Result<FontInstallState, String> {
  let full_names = {
    let guard = state.library.lock().map_err(|e| e.to_string())?;
    let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
    let (_info_dir, origin, meta) = locate_asset(lib, &asset_id)?;
    if !font::is_font(&origin) {
      return Err(format!("「{}」不是字体资源", meta.name));
    }
    let names = font::font_full_names(&origin);
    if names.is_empty() {
      return Err("字体文件解析失败，无法卸载".to_string());
    }
    names
  };
  font::uninstall_font(&full_names)?;
  Ok(FontInstallState { installed: false })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::library;
  use crate::services::import::{self, PendingFile, Source};

  struct Harness {
    dir: tempfile::TempDir,
    lib: library::Library,
  }

  fn make_lib() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    Harness { dir, lib }
  }

  /// 造一个单文件资源（假字节）并登记进内存索引
  fn add_file(h: &mut Harness, id: &str, name: &str) {
    let src = h.dir.path().join(name);
    std::fs::write(&src, b"fake").unwrap();
    let pf = PendingFile { path: src, chain: Vec::new() };
    let meta = import::build_asset_with_id(&h.lib.assets_dir, &pf, "sha256:t", id, Source::Copy, None).unwrap();
    h.lib.index.assets.insert(id.to_string(), meta);
  }

  #[test]
  fn font_gate_rejects_non_font_and_garbage() {
    let mut h = make_lib();
    add_file(&mut h, "AN-PIC", "参考图.png");
    let (_info, origin, meta) = locate_asset(&h.lib, "AN-PIC").unwrap();
    assert!(!font::is_font(&origin));
    assert_eq!(meta.ext, "png");

    // 假字体（fake 字节）：解析失败 → full_names 空（命令层据此中文报错）
    add_file(&mut h, "AN-FONT", "假字体.ttf");
    let (_info, origin, _meta) = locate_asset(&h.lib, "AN-FONT").unwrap();
    assert!(font::is_font(&origin));
    assert!(font::font_full_names(&origin).is_empty());
    assert!(font::probe_font(&origin).is_none());
  }
}
