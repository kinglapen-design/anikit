//! 命令 API：MD 打包导出（2026-10-08 S6，PRD 02-09——AE 收集打包的 MD 版）：
//! 库内 md 资产 → 打包到所选目录；工作空间 md → 打包到文档旁。产物 = {文档名}-打包/
//! （md 副本引用已改写 assets/ 相对路径 + assets/ 图片）；原文档不动。
//! 不改库数据不进撤销（同 export_assets 口径）；解析与编排见 services/md_bundle.rs

use tauri::State;

use crate::commands::library::AppState;
use crate::commands::preview::locate_asset;
use crate::commands::workspace::validated_ws_path;
use crate::services::md_bundle::{self, BundleReport};
use crate::services::text;

/// 产物目录：base/{stem}-打包（重名 -2 / -3… 规避——重复打包不覆盖既有产物）
fn fresh_bundle_dir(base: &std::path::Path, stem: &str) -> Result<std::path::PathBuf, String> {
  for n in 1.. {
    let name = if n == 1 { format!("{stem}-打包") } else { format!("{stem}-打包-{n}") };
    let dir = base.join(&name);
    if !dir.exists() {
      std::fs::create_dir_all(&dir).map_err(|e| format!("创建打包目录失败：{e}"))?;
      return Ok(dir);
    }
  }
  unreachable!()
}

/// 库内 md 资产打包导出：dest_dir（前端目录选择器）下建 {资产名}-打包/
#[tauri::command]
pub async fn export_md_bundle(state: State<'_, AppState>, asset_id: String, dest_dir: String) -> Result<BundleReport, String> {
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  let (_info_dir, origin, meta) = locate_asset(lib, &asset_id)?;
  if !meta.ext.eq_ignore_ascii_case("md") {
    return Err("仅支持 md 文档打包".to_string());
  }
  let dest = std::path::PathBuf::from(&dest_dir);
  if !dest.is_dir() {
    return Err("导出位置不是有效目录".to_string());
  }
  let content = text::read_text(&origin)?;
  let out_dir = fresh_bundle_dir(&dest, &meta.name)?;
  md_bundle::export_bundle(lib, &meta.name, &content, &out_dir)
}

/// 工作空间 md 打包导出：文档旁建 {文档名}-打包/（abs_path 经 validated_ws_path 校验；
/// 引用的库资产解析需要库已打开）
#[tauri::command]
pub async fn ws_export_md_bundle(state: State<'_, AppState>, abs_path: String) -> Result<BundleReport, String> {
  let path = validated_ws_path(&state, &abs_path)?;
  if path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).as_deref() != Some("md") {
    return Err("仅支持 md 文档打包".to_string());
  }
  let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).ok_or_else(|| "文件名异常".to_string())?;
  let parent = path.parent().ok_or_else(|| "路径异常".to_string())?.to_path_buf();
  let content = text::read_text(&path)?;
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  let out_dir = fresh_bundle_dir(&parent, &stem)?;
  md_bundle::export_bundle(lib, &stem, &content, &out_dir)
}
