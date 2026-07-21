//! 命令 API：导入（文件 / 文件夹 / 剪贴板）与导入弹窗所需的树 / 列表查询。
//! 命令层只做参数解析与状态取用，业务全部在 services（01_开发规范 第 1 条）。

use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::State;

use crate::commands::library::AppState;
use crate::core::library::{Library, ASSTES_DIR, CACHE_DIR};
use crate::core::models::Placement;
use crate::services::import as svc;
use crate::services::import::{ImportOptions, ImportOutcome, Source, Structure};

// ---------- 导入前预估 ----------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassifyResult {
  /// "files"（纯文件）/ "folder"（仅一个文件夹）/ "mixed"（多选含文件夹）
  pub kind: String,
  /// 展开后的文件总数（弹窗预估用）
  pub file_count: u32,
}

/// 分类所选路径并预估展开文件总数（导入弹窗第一步）
#[tauri::command]
pub fn classify_paths(paths: Vec<String>) -> ClassifyResult {
  let mut dir_count = 0usize;
  let mut file_count = 0u32;
  for p in &paths {
    let path = PathBuf::from(p);
    if path.is_dir() {
      dir_count += 1;
      file_count = file_count.saturating_add(count_files(&path));
    } else if path.is_file() {
      file_count = file_count.saturating_add(1);
    }
  }
  let kind = if dir_count == 0 {
    "files"
  } else if paths.len() == 1 {
    "folder"
  } else {
    "mixed"
  };
  ClassifyResult { kind: kind.to_string(), file_count }
}

/// 递归统计文件夹内文件数（仅弹窗预估，读不了的目录按 0 计）
fn count_files(dir: &Path) -> u32 {
  let mut count = 0u32;
  if let Ok(entries) = std::fs::read_dir(dir) {
    for entry in entries.filter_map(|e| e.ok()) {
      let path = entry.path();
      if path.is_dir() {
        count = count.saturating_add(count_files(&path));
      } else if path.is_file() {
        count = count.saturating_add(1);
      }
    }
  }
  count
}

// ---------- 导入 ----------

/// 导入文件 / 文件夹：structure "flatten"/"keep"，source "copy"/"move"
#[tauri::command]
pub fn import_paths(
  state: State<AppState>,
  paths: Vec<String>,
  structure: String,
  source: String,
  target_folder: Option<String>,
) -> Result<ImportOutcome, String> {
  let structure = parse_structure(&structure)?;
  let source = parse_source(&source)?;
  let paths: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库再导入".to_string())?;
  Ok(svc::import_paths(lib, &paths, &ImportOptions { structure, source, target_folder }))
}

/// 剪贴板导入：取剪贴板位图 → 落 cache/ 临时 PNG → 以 Move + Flatten 走标准导入管线
/// （剪贴板来源没有目录概念，永远剔除层级）
#[tauri::command]
pub fn import_clipboard(state: State<AppState>, target_folder: Option<String>) -> Result<ImportOutcome, String> {
  let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("剪贴板不可用：{e}"))?;
  let img = clipboard.get_image().map_err(|_| "剪贴板里没有图片".to_string())?;

  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库再导入".to_string())?;

  let cache_dir = lib.root.join(CACHE_DIR);
  std::fs::create_dir_all(&cache_dir).map_err(|e| format!("缓存目录创建失败：{e}"))?;
  let tmp = cache_dir.join(clipboard_png_name(&cache_dir));
  save_clipboard_png(&img, &tmp)?;

  let opts = ImportOptions { structure: Structure::Flatten, source: Source::Move, target_folder };
  let outcome = svc::import_paths(lib, std::slice::from_ref(&tmp), &opts);
  // 查重命中时管线不移动实体，临时文件自行清理，cache/ 不留残留
  if tmp.exists() {
    let _ = std::fs::remove_file(&tmp);
  }
  Ok(outcome)
}

/// 剪贴板临时文件名：剪贴板-yyyyMMdd-HHmm.png，重名追加 -序号
fn clipboard_png_name(cache_dir: &Path) -> String {
  let base = format!("剪贴板-{}", chrono::Local::now().format("%Y%m%d-%H%M"));
  let mut name = format!("{base}.png");
  let mut n = 1;
  while cache_dir.join(&name).exists() {
    name = format!("{base}-{n}.png");
    n += 1;
  }
  name
}

/// arboard RGBA 数据编码为 PNG 落盘
fn save_clipboard_png(img: &arboard::ImageData, path: &Path) -> Result<(), String> {
  let rgba = image::RgbaImage::from_raw(img.width as u32, img.height as u32, img.bytes.to_vec())
    .ok_or_else(|| "剪贴板图片数据无效".to_string())?;
  image::DynamicImage::ImageRgba8(rgba).save(path).map_err(|e| format!("剪贴板图片保存失败：{e}"))
}

fn parse_structure(s: &str) -> Result<Structure, String> {
  match s {
    "flatten" => Ok(Structure::Flatten),
    "keep" => Ok(Structure::Keep),
    _ => Err(format!("未知的层级模式：{s}（可选 flatten / keep）")),
  }
}

fn parse_source(s: &str) -> Result<Source, String> {
  match s {
    "copy" => Ok(Source::Copy),
    "move" => Ok(Source::Move),
    _ => Err(format!("未知的来源模式：{s}（可选 copy / move）")),
  }
}

// ---------- 文件夹树与 Placement 列表（导入弹窗 / 中间面板） ----------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderNode {
  pub id: String,
  pub name: String,
  /// 该文件夹直接 Placement 数（不含子文件夹）
  pub count: u32,
  pub children: Vec<FolderNode>,
}

/// 文件夹树（导入弹窗选目标文件夹用），按 Folder.order 排序
#[tauri::command]
pub fn list_folder_tree(state: State<AppState>) -> Result<Vec<FolderNode>, String> {
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  Ok(folder_tree(lib))
}

/// 组装文件夹树：parent=None 为顶层，递归子级
fn folder_tree(lib: &Library) -> Vec<FolderNode> {
  fn children_of(lib: &Library, parent: Option<&str>) -> Vec<FolderNode> {
    let mut folders: Vec<&crate::core::models::Folder> =
      lib.folders.folders.iter().filter(|f| f.parent.as_deref() == parent).collect();
    folders.sort_by_key(|f| f.order);
    folders
      .into_iter()
      .map(|f| FolderNode {
        id: f.id.clone(),
        name: f.name.clone(),
        count: lib.index.placements_by_folder.get(&f.id).map(|v| v.len()).unwrap_or(0) as u32,
        children: children_of(lib, Some(&f.id)),
      })
      .collect()
  }
  children_of(lib, None)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacementView {
  pub asset_id: String,
  pub folder_id: String,
  pub name: String,
  pub w: Option<u32>,
  pub h: Option<u32>,
  pub ext: String,
  /// 缩略图绝对路径（无缩略图为 null）
  pub thumb_path: Option<String>,
  pub origin: bool,
}

/// 列出 Placement：folder=None 全部；Some(id) 指定文件夹（"root" 表示根目录）
#[tauri::command]
pub fn list_placements(state: State<AppState>, folder: Option<String>) -> Result<Vec<PlacementView>, String> {
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  Ok(placement_views(lib, folder.as_deref()))
}

/// Placement → 视图：按索引反查资源补宽高 / 缩略图；资源缺失的 Placement 跳过
fn placement_views(lib: &Library, folder: Option<&str>) -> Vec<PlacementView> {
  let items: Vec<&Placement> = match folder {
    Some(id) => lib.index.placements_by_folder.get(id).map(|v| v.iter().collect()).unwrap_or_default(),
    None => lib.folders.items.iter().collect(),
  };
  items.into_iter().filter_map(|p| to_view(lib, p)).collect()
}

fn to_view(lib: &Library, p: &Placement) -> Option<PlacementView> {
  let meta = lib.index.assets.get(&p.asset)?;
  let thumb_path = meta.thumbnail.as_ref().map(|t| {
    lib.root.join(ASSTES_DIR).join(format!("{}.info", meta.id)).join(t).to_string_lossy().into_owned()
  });
  Some(PlacementView {
    asset_id: p.asset.clone(),
    folder_id: p.folder.clone(),
    name: p.name.clone(),
    w: meta.width,
    h: meta.height,
    ext: meta.ext.clone(),
    thumb_path,
    origin: p.origin,
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::library;
  use crate::services::import::ROOT_FOLDER_ID;

  fn make_lib() -> (tempfile::TempDir, Library) {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create(&dir.path().join("测试库")).unwrap();
    (dir, lib)
  }

  fn import(lib: &mut Library, paths: &[PathBuf], structure: Structure) {
    let opts = ImportOptions { structure, source: Source::Copy, target_folder: None };
    svc::import_paths(lib, paths, &opts);
  }

  #[test]
  fn classify_distinguishes_kinds_and_counts() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("a.txt");
    std::fs::write(&f, "x").unwrap();
    let folder = dir.path().join("夹");
    std::fs::create_dir_all(folder.join("子")).unwrap();
    std::fs::write(folder.join("1.txt"), "1").unwrap();
    std::fs::write(folder.join("子/2.txt"), "2").unwrap();

    let s = |p: &Path| p.to_string_lossy().into_owned();
    let only_files = classify_paths(vec![s(&f)]);
    assert_eq!((only_files.kind.as_str(), only_files.file_count), ("files", 1));
    let only_folder = classify_paths(vec![s(&folder)]);
    assert_eq!((only_folder.kind.as_str(), only_folder.file_count), ("folder", 2));
    let mixed = classify_paths(vec![s(&f), s(&folder)]);
    assert_eq!((mixed.kind.as_str(), mixed.file_count), ("mixed", 3));
  }

  #[test]
  fn folder_tree_nests_and_counts_direct_placements() {
    let (dir, mut lib) = make_lib();
    let src = dir.path().join("参考");
    std::fs::create_dir_all(src.join("人物")).unwrap();
    std::fs::write(src.join("a.txt"), "a").unwrap();
    std::fs::write(src.join("人物/b.txt"), "b").unwrap();
    import(&mut lib, &[src], Structure::Keep);

    let tree = folder_tree(&lib);
    assert_eq!(tree.len(), 1);
    assert_eq!((tree[0].name.as_str(), tree[0].count), ("参考", 1));
    assert_eq!((tree[0].children[0].name.as_str(), tree[0].children[0].count), ("人物", 1));
  }

  #[test]
  fn placement_views_join_meta_and_skip_missing() {
    let (dir, mut lib) = make_lib();
    let png = dir.path().join("图.png");
    image::RgbaImage::from_pixel(10, 20, image::Rgba([0, 0, 0, 255])).save(&png).unwrap();
    import(&mut lib, &[png], Structure::Flatten);

    let views = placement_views(&lib, Some(ROOT_FOLDER_ID));
    assert_eq!(views.len(), 1);
    assert_eq!((views[0].w, views[0].h), (Some(10), Some(20)));
    assert_eq!(views[0].ext, "png");
    assert!(views[0].origin);
    assert!(views[0].thumb_path.as_ref().unwrap().ends_with("_thumbnail.webp"));

    // 资源在索引中缺失的 Placement 跳过
    lib.folders.items.push(Placement {
      asset: "AN-不存在".into(),
      folder: ROOT_FOLDER_ID.into(),
      name: "幽灵.txt".into(),
      order: 1,
      origin: false,
    });
    assert_eq!(placement_views(&lib, None).len(), 1);
  }

  #[test]
  fn parse_modes_reject_unknown_values() {
    assert!(parse_structure("flatten").is_ok());
    assert!(parse_structure("树形").is_err());
    assert!(parse_source("move").is_ok());
    assert!(parse_source("拖拽").is_err());
  }

  #[test]
  fn clipboard_png_name_dedupes_with_counter() {
    let dir = tempfile::tempdir().unwrap();
    let first = clipboard_png_name(dir.path());
    std::fs::write(dir.path().join(&first), "x").unwrap();
    let second = clipboard_png_name(dir.path());
    assert_ne!(first, second);
    assert!(second.ends_with("-1.png"));
  }
}
