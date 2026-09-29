//! 命令 API：字体（5.6）。get_font_info = 预览「其它」页元数据 + origin 路径（前端 @font-face 加载）+
//! 系统安装状态；install_font / uninstall_font = 系统级安装 / 卸载（用户级，免管理员——
//! 其他软件立即可见，避免 Eagle"装了识别不到"，2026-08-23 用户痛点定案；实现见 services/font）。
//! 安装 / 卸载只动系统状态（字体目录 + 注册表），不改库数据；锁内定位、锁外系统 IO（不占库锁）。

use serde::Serialize;
use tauri::State;

use crate::commands::library::AppState;
use crate::commands::preview::locate_asset;
use crate::core::space::WORKSPACE_DIR;
use crate::services::font;
use crate::services::workspace as ws;

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
  /// cmap 是否覆盖中文（'永' U+6C38）——字体卡片示例字分流（2026-09-22 R6 font 接线）：
  /// true → 中文四行「永和九年 / 岁在癸丑 / AaBaCcDd / 01234567」，false → 拉丁三行
  pub has_cjk: bool,
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
    has_cjk: fm.has_cjk,
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

// ---------- 按真实路径读取（2026-09-26 工作空间三类填充卡对接 · 字体卡） ----------

/// 按真实路径读字体元数据与安装状态（工作空间字体卡）：形状同 get_font_info
///（originPath = ws 源文件绝对路径，前端 FontFace 经 convertFileSrc 加载）；
/// 越界 / 非字体 / 解析失败中文报错（前端卡片 catch 兜底系统字体渲染）。
/// 安装 / 卸载为 assetId 命令 + 系统级写入，ws 不开按路径版（前端 ws 下不出安装钮）。
/// async：文件 IO + 解析，一律走线程池（规范第 2 条，同 get_font_info）
#[tauri::command]
pub async fn get_font_info_by_path(state: State<'_, AppState>, abs_path: String) -> Result<FontInfo, String> {
  let space = state.space.lock().map_err(|e| e.to_string())?.clone().ok_or_else(|| "请先打开储存空间".to_string())?;
  let ws_root = space.root.join(WORKSPACE_DIR);
  font_info_by_path(&ws_root, std::path::Path::new(&abs_path))
}

/// 按路径读取实现（命令层薄壳，便于单测）：存在性 + workspace 越界校验同
/// get_media_source_by_path 口径；ws 无 metadata，错误文案取名自路径尾
fn font_info_by_path(ws_root: &std::path::Path, origin: &std::path::Path) -> Result<FontInfo, String> {
  if !origin.is_file() || !ws::is_under(ws_root, origin) {
    return Err("路径不在当前工作空间内（或文件已不存在）".to_string());
  }
  let name = || origin.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| origin.to_string_lossy().into_owned());
  if !font::is_font(origin) {
    return Err(format!("「{}」不是字体资源", name()));
  }
  let Some(fm) = font::probe_font(origin) else {
    return Err("字体文件解析失败".to_string());
  };
  let installed = font::font_installed(&font::font_full_names(origin));
  Ok(FontInfo {
    origin_path: origin.to_string_lossy().into_owned(),
    name: fm.name,
    full_name: fm.full_name,
    family: fm.family,
    style: fm.style,
    version: fm.version,
    glyph_count: fm.glyph_count,
    installed,
    has_cjk: fm.has_cjk,
  })
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

  /// 按路径读取（2026-09-26 工作空间字体卡）：越界 / 不存在 / 非字体 / 假字体一律中文错
  ///（前端卡片 catch 兜底系统字体渲染）；真字体正路径 = services/font 层职责
  ///（ANIKIT_TEST_FONT 目检测试），此处不重复
  #[test]
  fn font_info_by_path_gates() {
    let dir = tempfile::tempdir().unwrap();
    let ws_root = dir.path().join("workspace");
    std::fs::create_dir_all(&ws_root).unwrap();

    // 假字体：is_font 过扩展名，probe 失败 → 「字体文件解析失败」
    let fake = ws_root.join("假字体.ttf");
    std::fs::write(&fake, b"not a font").unwrap();
    let err = font_info_by_path(&ws_root, &fake).unwrap_err();
    assert!(err.contains("解析失败"), "{err}");

    // 非字体（扩展名路由）中文错
    let txt = ws_root.join("笔记.txt");
    std::fs::write(&txt, b"x").unwrap();
    let err = font_info_by_path(&ws_root, &txt).unwrap_err();
    assert!(err.contains("不是字体资源"), "{err}");

    // 越界 / 不存在
    let outside = dir.path().join("外.ttf");
    std::fs::write(&outside, b"fake").unwrap();
    assert!(font_info_by_path(&ws_root, &outside).unwrap_err().contains("不在当前工作空间"));
    assert!(font_info_by_path(&ws_root, &ws_root.join("不存在.ttf")).unwrap_err().contains("不在当前工作空间"));
  }
}
