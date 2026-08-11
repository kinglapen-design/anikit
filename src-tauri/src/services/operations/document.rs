//! 新建文本文档入库（1.3，07 新增菜单"MarkDown 文档 / 文本文档"点亮，2026-08-11 用户定）：
//! 纯文本直接建档（不走导入管线）——info 目录 + 空文件 + metadata.json + 前插 origin Placement
//! + 格式图标保底（cache/thumbs/{ext}.webp，shell 图标，4.2 链路）。

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

use sha2::Digest;

use crate::core::json_io::write_json;
use crate::core::library::{self, Library, ASSTES_DIR, CACHE_DIR};
use crate::core::models::{new_asset_id, AssetMeta, DATA_VERSION};
use crate::services::import::{add_placement, ROOT_FOLDER_ID};
use crate::services::operations::{check_folder, finish, now};
use crate::services::thumbnail;
use crate::services::views::{placement_view, PlacementView};

/// 新建文档的显示名词干（唯一化经 library::unique_name，与导入/新建文件夹同口径）
const DOC_STEM: &str = "新建文档";

/// 新建文本文档（kind = "md" | "txt"）入库；target_folder None / "root" = 根目录。
/// 返回（PlacementView, info 目录路径, 撤销备份路径）——后两个供命令层登记磁盘逆操作
///（撤销 = info 目录移入 cache/tmp 备份，重做 = 移回；Delete 会丢内容无法重做）
pub fn create_text_document(
  lib: &mut Library,
  kind: &str,
  target_folder: Option<&str>,
) -> Result<(PlacementView, PathBuf, PathBuf), String> {
  let ext = match kind {
    "md" => "md",
    "txt" => "txt",
    _ => return Err(format!("不支持的新建文档类型：{kind}")),
  };
  let folder_id = target_folder.unwrap_or(ROOT_FOLDER_ID);
  check_folder(lib, folder_id)?;

  // 唯一显示名（与 add_placement 内部同函数，先算定好写文件；add_placement 再验幂等）
  let existing: HashSet<String> = lib
    .index
    .placements_by_folder
    .get(folder_id)
    .map(|v| v.iter().map(|p| p.name.clone()).collect())
    .unwrap_or_default();
  let name = library::unique_name(&existing, &format!("{DOC_STEM}.{ext}"));
  let stem = name.strip_suffix(&format!(".{ext}")).unwrap_or(DOC_STEM).to_string();

  let id = new_asset_id();
  let info_dir = lib.root.join(ASSTES_DIR).join(format!("{id}.info"));
  fs::create_dir_all(&info_dir).map_err(|e| format!("info 文件夹创建失败：{e}"))?;
  let dest = info_dir.join(&name);
  if let Err(e) = fs::write(&dest, b"") {
    let _ = fs::remove_dir_all(&info_dir); // 半截清理，不留孤儿 info
    return Err(format!("文档创建失败：{e}"));
  }

  let now = now();
  let meta = AssetMeta {
    version: DATA_VERSION,
    id: id.clone(),
    name: stem,
    origin_name: name.clone(),
    ext: ext.to_string(),
    size: 0,
    hash: format!("sha256:{}", hex_lower(&sha2::Sha256::digest([]))),
    width: None,
    height: None,
    duration: None,
    bpm: None,
    rating: 0,
    links: Vec::new(),
    imported_at: now.clone(),
    imported_from: None,
    modified_at: now,
    tags: Vec::new(),
    note: String::new(),
    note_count: None,
    thumbnail: None,
    fps: None,
    frame_count: None,
    thumb_frame: None,
    r#type: None,
    source: None,
    start: None,
    end: None,
  };
  if let Err(e) = write_json(&info_dir.join("metadata.json"), &meta) {
    let _ = fs::remove_dir_all(&info_dir);
    return Err(e.to_string());
  }
  // 格式图标保底（shell，进程级串行锁在 shell_thumb 内）：就绪则登记内存集，卡片即显系统图标
  if thumbnail::ensure_format_icon(&lib.root, &dest, ext) {
    lib.format_icons.insert(ext.to_string());
  }

  let placement = add_placement(lib, id.clone(), folder_id.to_string(), name, true, &mut HashMap::new());
  lib.index.assets.insert(id.clone(), meta.clone());
  finish(lib)?;

  // 撤销备份位（cache/tmp 会话级，undo 栈同为会话级，重启清空无孤儿残留）
  let backup_dir = lib.root.join(CACHE_DIR).join("tmp");
  fs::create_dir_all(&backup_dir).map_err(|e| format!("临时目录创建失败：{e}"))?;
  let backup = backup_dir.join(format!("undo-{id}.info"));
  Ok((placement_view(&lib.root, &meta, &placement, &lib.format_icons), info_dir, backup))
}

/// 小写 hex（不引 hex crate，摘要仅 32 字节）
fn hex_lower(bytes: &[u8]) -> String {
  bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::json_io::read_json;
  use crate::core::models::{AssetMeta, FoldersFile};
  use crate::core::library::FOLDERS_FILE;

  struct Harness {
    _dir: tempfile::TempDir,
    lib: Library,
  }

  fn make_lib() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("测试库");
    let lib = library::create(&root).unwrap();
    Harness { _dir: dir, lib }
  }

  #[test]
  fn create_md_document_builds_asset_and_origin_placement() {
    let mut h = make_lib();
    let (view, info_dir, _backup) = create_text_document(&mut h.lib, "md", None).unwrap();
    assert_eq!(view.name, "新建文档.md");
    assert_eq!(view.ext, "md");
    assert!(view.origin);
    assert_eq!(view.folder_id, ROOT_FOLDER_ID);
    assert!(info_dir.join("新建文档.md").exists(), "空文档落盘");
    let meta: AssetMeta = read_json(&info_dir.join("metadata.json")).unwrap();
    assert_eq!(meta.name, "新建文档");
    assert_eq!(meta.origin_name, "新建文档.md");
    assert!(meta.hash.starts_with("sha256:"));
    assert!(meta.thumbnail.is_none() && meta.width.is_none());
    // folders.json 落盘 + origin placement 前插
    let disk: FoldersFile = read_json(&h.lib.root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(disk.items.len(), 1);
    assert!(disk.items[0].origin && disk.items[0].asset == view.asset_id);
    // 格式图标保底（Windows shell：md 有无关联不定，有则缓存；不强制断言缓存命中）
    assert!(h.lib.index.assets.contains_key(&view.asset_id));
  }

  #[test]
  fn create_txt_document_uniquifies_name() {
    let mut h = make_lib();
    let (v1, _, _) = create_text_document(&mut h.lib, "txt", None).unwrap();
    let (v2, _, _) = create_text_document(&mut h.lib, "txt", None).unwrap();
    assert_eq!(v1.name, "新建文档.txt");
    assert_ne!(v1.name, v2.name, "同文件夹撞名唯一化");
    assert!(v2.name.starts_with("新建文档-"));
  }

  #[test]
  fn rejects_unsupported_kind_and_missing_folder() {
    let mut h = make_lib();
    assert!(create_text_document(&mut h.lib, "docx", None).is_err());
    assert!(create_text_document(&mut h.lib, "md", Some("F不存在")).is_err());
  }
}
