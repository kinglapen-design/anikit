//! 命令 API：文本资产（5.6）。read_text_asset = txt/md 预览读取（编码回退见 services/text）；
//! write_text_asset = APP 内编辑保存（防抖自动落盘，2026-08-23 用户定）——origin 文本文件新写点，
//! 保存后同步 metadata.size / modifiedAt（先落盘再同步内存索引，规范第 7 条）；
//! modifiedAt 随结果返回（前端同步 PlacementView）。编辑不进 undo（库操作撤销体系不覆盖内容编辑）。

use serde::Serialize;
use tauri::State;

use crate::commands::library::AppState;
use crate::commands::preview::locate_asset;
use crate::core::json_io::write_json;
use crate::core::library::Library;
use crate::services::text;

/// 文本内容
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TextContent {
  pub content: String,
}

/// 保存结果（size / modifiedAt 前端同步 PlacementView 用）
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WriteTextResult {
  pub size: u64,
  pub modified_at: String,
}

/// 读取文本内容（txt / md；UTF-8 优先 GBK 回退，2MB 上限）。
/// async：文件 IO，一律走线程池（规范第 2 条）
#[tauri::command]
pub async fn read_text_asset(state: State<'_, AppState>, asset_id: String) -> Result<TextContent, String> {
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  let (_info_dir, origin, meta) = locate_asset(lib, &asset_id)?;
  if !text::is_text(&origin) {
    return Err(format!("「{}」不是文本资源", meta.name));
  }
  let content = text::read_text(&origin)?;
  Ok(TextContent { content })
}

/// 保存文本内容：覆写 origin（恒 UTF-8 无 BOM）→ metadata.size / modifiedAt 落盘 → 同步索引。
#[tauri::command]
pub async fn write_text_asset(
  state: State<'_, AppState>,
  asset_id: String,
  content: String,
) -> Result<WriteTextResult, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  write_text(lib, &asset_id, &content)
}

/// 保存文本内容：覆写 origin（恒 UTF-8 无 BOM）→ metadata.size / modifiedAt 落盘 → 同步索引。
/// pub(crate)：server/mcp（AI 外接通道二期写工具 write_text_asset）同用；
/// 不进 undo（库操作撤销体系不覆盖内容编辑，2026-08-23 定——MCP 工具描述同口径告知 Agent）
pub(crate) fn write_text(lib: &mut Library, asset_id: &str, content: &str) -> Result<WriteTextResult, String> {
  let (info_dir, origin, meta) = locate_asset(lib, asset_id)?;
  if !text::is_text(&origin) {
    return Err(format!("「{}」不是文本资源", meta.name));
  }
  if content.len() as u64 > text::MAX_TEXT_BYTES {
    return Err("内容超过 2MB，无法保存".to_string());
  }
  std::fs::write(&origin, content.as_bytes()).map_err(|e| format!("保存失败：{e}"))?;
  let mut meta = meta;
  meta.size = content.len() as u64;
  meta.modified_at = chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, false);
  let modified_at = meta.modified_at.clone();
  let size = meta.size;
  write_json(&info_dir.join("metadata.json"), &meta).map_err(|e| e.to_string())?;
  lib.index.assets.insert(asset_id.to_string(), meta);
  Ok(WriteTextResult { size, modified_at })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::json_io::read_json;
  use crate::core::library;
  use crate::core::models::AssetMeta;
  use crate::services::import::{self, PendingFile, Source};

  struct Harness {
    dir: tempfile::TempDir,
    lib: Library,
  }

  fn make_lib() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    Harness { dir, lib }
  }

  /// 造一个文本资源（真实内容入库）并登记进内存索引
  fn add_text(h: &mut Harness, id: &str, name: &str, content: &str) -> AssetMeta {
    let src = h.dir.path().join(name);
    std::fs::write(&src, content).unwrap();
    let pf = PendingFile { path: src, chain: Vec::new() };
    let meta = import::build_asset_with_id(&h.lib.assets_dir, &pf, "sha256:t", id, Source::Copy, None).unwrap();
    h.lib.index.assets.insert(id.to_string(), meta.clone());
    meta
  }

  #[test]
  fn write_text_persists_and_syncs_size_and_modified() {
    let mut h = make_lib();
    let before = add_text(&mut h, "AN-TXT", "提示词.txt", "旧内容");
    let info = h.lib.info_dir("AN-TXT");

    let r = write_text(&mut h.lib, "AN-TXT", "新内容 🎬 第二行").unwrap();
    assert_eq!(r.size, "新内容 🎬 第二行".len() as u64);
    // modifiedAt 秒级精度（同秒写入相等属正常）：断言不倒退即可
    assert!(r.modified_at >= before.modified_at, "{} < {}", r.modified_at, before.modified_at);

    // 磁盘内容 = 新内容（UTF-8）
    let origin = info.join("提示词.txt");
    assert_eq!(std::fs::read_to_string(&origin).unwrap(), "新内容 🎬 第二行");

    // metadata 落盘 + 索引同步
    let disk: AssetMeta = read_json(&info.join("metadata.json")).unwrap();
    assert_eq!(disk.size, r.size);
    assert_eq!(disk.modified_at, r.modified_at);
    assert_eq!(h.lib.index.assets["AN-TXT"].size, r.size);

    // 写后读回一致
    assert_eq!(text::read_text(&origin).unwrap(), "新内容 🎬 第二行");
  }

  #[test]
  fn text_gate_rejects_non_text_and_oversize() {
    let mut h = make_lib();
    let src = h.dir.path().join("图.png");
    image::RgbaImage::from_pixel(8, 8, image::Rgba([1, 2, 3, 255])).save(&src).unwrap();
    let pf = PendingFile { path: src, chain: Vec::new() };
    let meta = import::build_asset_with_id(&h.lib.assets_dir, &pf, "sha256:t", "AN-PNG", Source::Copy, None).unwrap();
    h.lib.index.assets.insert("AN-PNG".to_string(), meta);

    let err = write_text(&mut h.lib, "AN-PNG", "x").unwrap_err();
    assert!(err.contains("不是文本资源"), "{err}");

    add_text(&mut h, "AN-MD", "文档.md", "# hi");
    let big = "a".repeat((text::MAX_TEXT_BYTES + 1) as usize);
    let err = write_text(&mut h.lib, "AN-MD", &big).unwrap_err();
    assert!(err.contains("2MB"), "{err}");
  }
}
