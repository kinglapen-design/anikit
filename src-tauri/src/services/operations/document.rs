//! 新建文档入库（3.1 二轮通用化，2026-08-15 用户定：放弃 shell 检测模式，固定七类可选项）：
//! md/txt 纯文本直接建档；fla/psd/ai/docx/xlsx 以 assets/templates/ 空白模板为内容建档
//!（模板编译期内嵌 include_bytes!，用户真实空白文件覆盖同名文件即生效）。
//! 均不走导入管线——info 目录 + 模板内容 + metadata.json + 前插 origin Placement
//! + 格式图标保底（cache/thumbs/{ext}.webp，shell 图标，4.2 链路）。

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

use sha2::Digest;

use crate::core::json_io::write_json;
use crate::core::library::{self, Library, CACHE_DIR};
use crate::core::models::{new_asset_id, AssetMeta, DATA_VERSION};
use crate::services::import::{add_placement, ROOT_FOLDER_ID};
use crate::services::operations::{check_folder, finish, now};
use crate::services::thumbnail;
use crate::services::views::{placement_view, PlacementView};

/// 新建文档的显示名词干（唯一化经 library::unique_name，与导入/新建文件夹同口径）
const DOC_STEM: &str = "新建文档";

/// 文档类型 →（扩展名, 初始内容）。AE 不建档（工程文件素材为链接路径、需打包工程，
/// 属工作空间而非资源库，2026-08-15 用户定）；二进制模板 = assets/templates/ 空白文件
fn doc_template(kind: &str) -> Option<(&'static str, &'static [u8])> {
  Some(match kind {
    "md" => ("md", b"".as_slice()),
    "txt" => ("txt", b"".as_slice()),
    "fla" => ("fla", include_bytes!("../../../../assets/templates/blank.fla").as_slice()),
    "psd" => ("psd", include_bytes!("../../../../assets/templates/blank.psd").as_slice()),
    "ai" => ("ai", include_bytes!("../../../../assets/templates/blank.ai").as_slice()),
    "docx" => ("docx", include_bytes!("../../../../assets/templates/blank.docx").as_slice()),
    "xlsx" => ("xlsx", include_bytes!("../../../../assets/templates/blank.xlsx").as_slice()),
    _ => return None,
  })
}

/// 新建文档（kind 见 doc_template）入库；target_folder None / "root" = 根目录。
/// 返回（PlacementView, info 目录路径, 撤销备份路径）——后两个供命令层登记磁盘逆操作
///（撤销 = info 目录移入 cache/tmp 备份，重做 = 移回；Delete 会丢内容无法重做）
pub fn create_document(
  lib: &mut Library,
  kind: &str,
  target_folder: Option<&str>,
) -> Result<(PlacementView, PathBuf, PathBuf), String> {
  let (ext, content) = doc_template(kind).ok_or_else(|| format!("不支持的新建文档类型：{kind}"))?;
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
  let info_dir = lib.info_dir(&id);
  fs::create_dir_all(&info_dir).map_err(|e| format!("info 文件夹创建失败：{e}"))?;
  let dest = info_dir.join(&name);
  if let Err(e) = fs::write(&dest, content) {
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
    size: content.len() as u64,
    hash: format!("sha256:{}", hex_lower(&sha2::Sha256::digest(content))),
    width: None,
    height: None,
    thumb_width: None,
    thumb_height: None,
    duration: None,
    bpm: None,
    rating: 0,
    links: Vec::new(),
    imported_at: now.clone(),
    imported_from: None,
    created_at: Some(now.clone()), // 新建文档无源文件，建档时刻即创建时间
    modified_at: now,
    tags: Vec::new(),
    note: String::new(),
    descriptions: Vec::new(),
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
  Ok((placement_view(lib, &meta, &placement, &lib.format_icons), info_dir, backup))
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
    let lib = library::create_test(dir.path());
    Harness { _dir: dir, lib }
  }

  #[test]
  fn create_md_document_builds_asset_and_origin_placement() {
    let mut h = make_lib();
    let (view, info_dir, _backup) = create_document(&mut h.lib, "md", None).unwrap();
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
    let (v1, _, _) = create_document(&mut h.lib, "txt", None).unwrap();
    let (v2, _, _) = create_document(&mut h.lib, "txt", None).unwrap();
    assert_eq!(v1.name, "新建文档.txt");
    assert_ne!(v1.name, v2.name, "同文件夹撞名唯一化");
    assert!(v2.name.starts_with("新建文档-"));
  }

  #[test]
  fn create_binary_document_writes_template_content() {
    let mut h = make_lib();
    for (kind, ext) in [("fla", "fla"), ("psd", "psd"), ("ai", "ai"), ("docx", "docx"), ("xlsx", "xlsx")] {
      let (view, info_dir, _) = create_document(&mut h.lib, kind, None).unwrap();
      let template = doc_template(kind).unwrap().1;
      assert_eq!(view.ext, ext);
      let meta: AssetMeta = read_json(&info_dir.join("metadata.json")).unwrap();
      assert_eq!(meta.size, template.len() as u64, "{kind} 大小 = 模板字节数");
      assert_eq!(std::fs::read(info_dir.join(&view.name)).unwrap(), template, "{kind} 落盘 = 模板内容");
    }
  }

  #[test]
  fn rejects_unsupported_kind_and_missing_folder() {
    let mut h = make_lib();
    assert!(create_document(&mut h.lib, "aep", None).is_err(), "AE 不建档（2026-08-15 用户定）");
    assert!(create_document(&mut h.lib, "md", Some("F不存在")).is_err());
  }
}
