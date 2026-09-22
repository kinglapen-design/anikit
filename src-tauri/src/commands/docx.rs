//! 命令 API：docx 预览（PRD 04-04 条 6，2026-09-22 R6 新功能验证首例）。
//! read_docx_asset = docx 只读预览内容提取（services/docx 解析段落结构，非版式级渲染）；
//! 只读不编辑，无写命令（编辑走默认应用打开，「软件内部不内置编辑器」口径）。

use tauri::State;

use crate::commands::library::AppState;
use crate::commands::preview::locate_asset;
use crate::services::docx::{self, DocxContent};

/// 读取 docx 预览内容（段落结构 + 截断标记；XML 32MB / 文本 2MB 上限见 services/docx）。
/// async：文件 IO，一律走线程池（规范第 2 条）
#[tauri::command]
pub async fn read_docx_asset(state: State<'_, AppState>, asset_id: String) -> Result<DocxContent, String> {
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  let (_info_dir, origin, meta) = locate_asset(lib, &asset_id)?;
  if !docx::is_docx(&origin) {
    return Err(format!("「{}」不是 docx 资源", meta.name));
  }
  docx::read_docx(&origin)
}
