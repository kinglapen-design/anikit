//! 命令 API：docx 预览（PRD 04-04 条 6）。
//! read_docx_binary = 预览给原始字节（2026-10-10 mammoth 换引擎：前端 mammoth.js 转 HTML，
//! 表格带框线 / 图片内嵌）；read_docx_asset = 库内文本卡段落结构提取（卡片要轻文本、不渲染 HTML
//! 不吃 mammoth——图文大文档按字节拉进卡片太贵，2026-10-10 换引擎时实证存活）；read_docx_by_path =
//! 工作空间文本卡同口径按路径版。
//! 只读不编辑，无写命令（编辑走默认应用打开，「软件内部不内置编辑器」口径）。

use tauri::State;

use crate::commands::library::AppState;
use crate::commands::preview::locate_asset;
use crate::core::space::WORKSPACE_DIR;
use crate::services::docx::{self, DocxContent};
use crate::services::workspace as ws;

/// 读取 docx 原始字节（预览 mammoth 链路；64MB 容量闸在 services::docx::read_docx_bytes）。
/// 返回原始二进制 IPC 响应（前端 invoke 得 ArrayBuffer，不经 JSON 序列化）。
/// async：文件 IO，一律走线程池（规范第 2 条）
/// origin 外部编辑自愈触点（同 read_text_asset 口径——Word 等默认应用回写后 size/mtime
/// 漂移自愈 + 广播 asset-origin-healed）
#[tauri::command]
pub async fn read_docx_binary(app: tauri::AppHandle, state: State<'_, AppState>, asset_id: String) -> Result<tauri::ipc::Response, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  if let Some((size, modified_at)) = crate::services::origin_sync::heal_if_drifted(lib, &asset_id) {
    crate::services::origin_sync::emit_healed(&app, &asset_id, size, &modified_at);
  }
  let (_info_dir, origin, _meta) = locate_asset(lib, &asset_id)?;
  let bytes = docx::read_docx_bytes(&origin)?;
  Ok(tauri::ipc::Response::new(bytes))
}

/// 读取 docx 预览内容（段落结构 + 截断标记；XML 32MB / 文本 2MB 上限见 services/docx）。
/// 消费方 = 库内文本卡正文懒加载（lib/cardText.ts；预览已换 mammoth 不走此路）。
/// async：文件 IO，一律走线程池（规范第 2 条）
/// 2026-10-08 origin 外部编辑自愈（同 read_text_asset 触点口径）
#[tauri::command]
pub async fn read_docx_asset(app: tauri::AppHandle, state: State<'_, AppState>, asset_id: String) -> Result<DocxContent, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  if let Some((size, modified_at)) = crate::services::origin_sync::heal_if_drifted(lib, &asset_id) {
    crate::services::origin_sync::emit_healed(&app, &asset_id, size, &modified_at);
  }
  let (_info_dir, origin, meta) = locate_asset(lib, &asset_id)?;
  if !docx::is_docx(&origin) {
    return Err(format!("「{}」不是 docx 资源", meta.name));
  }
  docx::read_docx(&origin)
}

// ---------- 按真实路径读取（2026-09-26 工作空间三类填充卡对接 · 文本卡 docx 支路） ----------

/// 按真实路径读 docx 预览内容（工作空间文本卡）：越界 / 非 docx / 损坏中文报错
/// （前端卡片 catch 兜底空串）。未开空间报「请先打开储存空间」。
/// async：文件 IO，一律走线程池（规范第 2 条，同 read_docx_asset）
#[tauri::command]
pub async fn read_docx_by_path(state: State<'_, AppState>, abs_path: String) -> Result<DocxContent, String> {
  let space = state.space.lock().map_err(|e| e.to_string())?.clone().ok_or_else(|| "请先打开储存空间".to_string())?;
  let ws_root = space.root.join(WORKSPACE_DIR);
  docx_by_path(&ws_root, std::path::Path::new(&abs_path))
}

/// 按路径读取实现（命令层薄壳，便于单测）：存在性 + workspace 越界校验同
/// get_media_source_by_path 口径；ws 无 metadata，错误文案取名自路径尾
fn docx_by_path(ws_root: &std::path::Path, origin: &std::path::Path) -> Result<DocxContent, String> {
  if !origin.is_file() || !ws::is_under(ws_root, origin) {
    return Err("路径不在当前工作空间内（或文件已不存在）".to_string());
  }
  if !docx::is_docx(origin) {
    let name = origin.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| origin.to_string_lossy().into_owned());
    return Err(format!("「{name}」不是 docx 资源"));
  }
  docx::read_docx(origin)
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::Write;

  /// 造最小 docx（zip 内一份 document.xml；同 services/docx 测试先例）
  fn make_docx(dir: &std::path::Path, name: &str, xml: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    let f = std::fs::File::create(&path).unwrap();
    let mut zw = zip::ZipWriter::new(f);
    zw.start_file("word/document.xml", zip::write::SimpleFileOptions::default()).unwrap();
    zw.write_all(xml.as_bytes()).unwrap();
    zw.finish().unwrap();
    path
  }

  /// 按路径读取（2026-09-26 工作空间文本卡 docx 支路）：workspace 内 docx 正常解析；
  /// 越界 / 不存在 / 非 docx 一律中文错（前端卡片 catch 兜底空串）
  #[test]
  fn docx_by_path_reads_inside_and_rejects_outside() {
    let dir = tempfile::tempdir().unwrap();
    let ws_root = dir.path().join("workspace");
    std::fs::create_dir_all(&ws_root).unwrap();

    let path = make_docx(
      &ws_root,
      "剧本.docx",
      r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>
<w:p><w:r><w:t>第一幕 🎬</w:t></w:r></w:p>
</w:body></w:document>"#,
    );
    let c = docx_by_path(&ws_root, &path).unwrap();
    assert_eq!(c.paragraphs[0].runs[0].text, "第一幕 🎬");

    // 非 docx（扩展名路由）中文错
    let txt = ws_root.join("笔记.txt");
    std::fs::write(&txt, b"x").unwrap();
    let err = docx_by_path(&ws_root, &txt).unwrap_err();
    assert!(err.contains("不是 docx 资源"), "{err}");

    // 越界 / 不存在
    let outside = make_docx(
      dir.path(),
      "外.docx",
      r#"<?xml version="1.0"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body/></w:document>"#,
    );
    assert!(docx_by_path(&ws_root, &outside).unwrap_err().contains("不在当前工作空间"));
    assert!(docx_by_path(&ws_root, &ws_root.join("不存在.docx")).unwrap_err().contains("不在当前工作空间"));
  }
}
