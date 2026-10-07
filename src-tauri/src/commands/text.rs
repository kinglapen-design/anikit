//! 命令 API：文本资产（5.6）。read_text_asset = txt/md 预览读取（编码回退见 services/text）；
//! write_text_asset = APP 内编辑保存（防抖自动落盘，2026-08-23 用户定）——origin 文本文件新写点，
//! 保存后同步 metadata.size / modifiedAt / hash（先落盘再同步内存索引，规范第 7 条；
//! hash 回写 = 内容指纹跟上内容，2026-09-22 用户指出漂移隐患）；
//! modifiedAt 随结果返回（前端同步 PlacementView）。编辑不进 undo（库操作撤销体系不覆盖内容编辑）。

use serde::Serialize;
use tauri::State;

use crate::commands::library::AppState;
use crate::commands::preview::locate_asset;
use crate::core::library::Library;
use crate::core::space::WORKSPACE_DIR;
use crate::services::text;
use crate::services::workspace as ws;

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
/// 2026-10-08 origin 外部编辑自愈（哈希漂移治理第 2 层）：文本卡正文懒拉即触点——
/// 外部编辑器（Typora 等）回写后 size/mtime 漂移 → 自愈回写 + 广播 asset-origin-healed，
/// 卡片 modifiedAt 版本戳失效自动重拉，手动「刷新缩略图」之外的自动托底
#[tauri::command]
pub async fn read_text_asset(app: tauri::AppHandle, state: State<'_, AppState>, asset_id: String) -> Result<TextContent, String> {
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  if let Some((size, modified_at)) = crate::services::origin_sync::heal_if_drifted(lib, &asset_id) {
    crate::services::origin_sync::emit_healed(&app, &asset_id, size, &modified_at);
  }
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

/// 保存文本内容：覆写 origin（恒 UTF-8 无 BOM）→ metadata.size / modifiedAt / **hash** 落盘 → 同步索引。
/// pub(crate)：server/mcp（AI 外接通道二期写工具 write_text_asset）同用；
/// 不进 undo（库操作撤销体系不覆盖内容编辑，2026-08-23 定——MCP 工具描述同口径告知 Agent）。
/// 2026-09-26 提示词轮：实现下迁 services::text::write_origin（commands 调 services 为正方向；
/// hash 回写口径随之下迁，2026-09-22 用户指出「内容变了账面照旧」隐患的修复语义不变）
pub(crate) fn write_text(lib: &mut Library, asset_id: &str, content: &str) -> Result<WriteTextResult, String> {
  let (size, modified_at) = text::write_origin(lib, asset_id, content)?;
  Ok(WriteTextResult { size, modified_at })
}

// ---------- 按真实路径读取（2026-09-26 工作空间三类填充卡对接 · 文本卡） ----------

/// 按真实路径读文本内容（工作空间文本卡正文）：越界 / 非文本 / 超上限中文报错
/// （前端卡片 catch 兜底空串）。未开空间报「请先打开储存空间」（同写操作口径——
/// 调用方必为已展示的 ws 条目，空返回无意义）。
/// async：文件 IO，一律走线程池（规范第 2 条，同 read_text_asset）
#[tauri::command]
pub async fn read_text_by_path(state: State<'_, AppState>, abs_path: String) -> Result<TextContent, String> {
  let space = state.space.lock().map_err(|e| e.to_string())?.clone().ok_or_else(|| "请先打开储存空间".to_string())?;
  let ws_root = space.root.join(WORKSPACE_DIR);
  text_by_path(&ws_root, std::path::Path::new(&abs_path))
}

/// 按路径读取实现（命令层薄壳，便于单测）：存在性 + workspace 越界校验同
/// get_media_source_by_path 口径；ws 无 metadata，错误文案取名自路径尾
fn text_by_path(ws_root: &std::path::Path, origin: &std::path::Path) -> Result<TextContent, String> {
  if !origin.is_file() || !ws::is_under(ws_root, origin) {
    return Err("路径不在当前工作空间内（或文件已不存在）".to_string());
  }
  if !text::is_text(origin) {
    let name = origin.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| origin.to_string_lossy().into_owned());
    return Err(format!("「{name}」不是文本资源"));
  }
  let content = text::read_text(origin)?;
  Ok(TextContent { content })
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

    // hash 回写（2026-09-22）：内容指纹跟上内容——导入时旧 hash，保存后 = 新内容 hash
    let expect_hash = import::sha256_bytes("新内容 🎬 第二行".as_bytes());
    assert_ne!(before.hash, expect_hash, "前置：编辑前后内容不同 hash 应不同");
    assert_eq!(disk.hash, expect_hash);
    assert_eq!(h.lib.index.assets["AN-TXT"].hash, expect_hash);

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

  /// 按路径读取（2026-09-26 工作空间文本卡）：workspace 内 txt 正常读回；越界 / 不存在 /
  /// 非文本一律中文错（前端卡片 catch 兜底空串）
  #[test]
  fn text_by_path_reads_inside_and_rejects_outside() {
    let dir = tempfile::tempdir().unwrap();
    let ws_root = dir.path().join("workspace");
    std::fs::create_dir_all(&ws_root).unwrap();

    let txt = ws_root.join("笔记.txt");
    std::fs::write(&txt, "你好\n第二行 🎬").unwrap();
    assert_eq!(text_by_path(&ws_root, &txt).unwrap(), TextContent { content: "你好\n第二行 🎬".to_string() });

    // 非文本（扩展名路由）中文错
    let png = ws_root.join("图.png");
    std::fs::write(&png, b"fake").unwrap();
    let err = text_by_path(&ws_root, &png).unwrap_err();
    assert!(err.contains("不是文本资源"), "{err}");

    // 越界 / 不存在
    let outside = dir.path().join("外.txt");
    std::fs::write(&outside, b"x").unwrap();
    assert!(text_by_path(&ws_root, &outside).unwrap_err().contains("不在当前工作空间"));
    assert!(text_by_path(&ws_root, &ws_root.join("不存在.txt")).unwrap_err().contains("不在当前工作空间"));
  }
}
