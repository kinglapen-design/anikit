//! 命令 API：导入（任务化）与导入弹窗所需的树 / 列表查询。
//! 导入命令只做参数解析与短锁校验，随即 spawn 工作线程跑 services::import_task::run_import，
//! 立即返回 task_id；进度 / 结果经 on_event Channel 推送，cancel_import 可取消。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

use crate::commands::library::AppState;
use crate::core::library::{Library, CACHE_DIR};
use crate::core::models::Placement;
use crate::services::import::{ImportEvent, ImportOptions, SequenceMode, Source, Structure, ROOT_FOLDER_ID};
use crate::services::import_task::run_import;
use crate::services::views::{placement_view, PlacementView};

/// 导入任务 ID 自增源
static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(1);

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

// ---------- 序列帧检测（4.1，09 导入弹窗确认后调用） ----------

/// 检测到的序列帧组摘要（前端"监测到序列帧"弹窗依据；打包入库由 import_paths 重新检测执行）
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SequenceGroupInfo {
  /// 基础名（资产标题）
  pub name: String,
  pub ext: String,
  pub frame_count: u32,
}

/// 检测所选路径中的序列帧（后端为唯一事实来源：同目录兄弟文件按 基础名+扩展名+位宽 分组，
/// 序号连续允许少量缺帧且 ≥3，仅图片格式）；检测失败的路径静默跳过（导入阶段会进 failed）
#[tauri::command]
pub fn detect_sequences(paths: Vec<String>) -> Vec<SequenceGroupInfo> {
  let mut ignored = Vec::new();
  let raw: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
  let files = crate::services::import::collect(&raw, Structure::Flatten, &mut ignored);
  crate::services::sequence::detect_groups(&files)
    .into_iter()
    .map(|g| SequenceGroupInfo { name: g.name, ext: g.ext, frame_count: g.frames.len() as u32 })
    .collect()
}

// ---------- 导入（任务化） ----------

/// 导入文件 / 文件夹：立即返回 task_id，工作线程执行，事件经 on_event 推送；
/// sequence_mode "pack" 时检测到的序列帧整组打包为一个资源（4.1）
#[tauri::command]
pub fn import_paths(
  app: AppHandle,
  paths: Vec<String>,
  structure: String,
  source: String,
  target_folder: Option<String>,
  sequence_mode: String,
  on_event: Channel<ImportEvent>,
) -> Result<u64, String> {
  let structure = parse_structure(&structure)?;
  let source = parse_source(&source)?;
  let sequence_mode = parse_sequence_mode(&sequence_mode)?;
  let root = current_root(&app)?;
  ensure_target(&app, &target_folder)?;
  let opts = ImportOptions {
    structure,
    source,
    target_folder,
    paths: paths.iter().map(PathBuf::from).collect(),
    imported_from: None,
    sequence_mode,
  };
  Ok(spawn_import(app, root, opts, on_event, None))
}

/// 剪贴板导入：先读图片（截图场景 → PNG 落 cache/ 临时文件），无图再读文件列表
/// （资源管理器 Ctrl+C 的文件，CF_HDROP）；两者皆无报错"剪贴板里没有图片或文件"。
/// 来源恒 Copy（2026-07-27 定：剪贴板复制的文件绝不能 Move 走用户原文件；
/// 临时 PNG 由工作线程导入结束后清理，Copy 与 Move 等价且更安全）；Flatten + Scatter。
#[tauri::command]
pub fn import_clipboard(
  app: AppHandle,
  target_folder: Option<String>,
  on_event: Channel<ImportEvent>,
) -> Result<u64, String> {
  let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("剪贴板不可用：{e}"))?;
  let root = current_root(&app)?;
  ensure_target(&app, &target_folder)?;

  // 图片分支：截图 / 剪贴板图片 → PNG 落 cache/ 走管线（临时文件导入后清理）
  if let Ok(img) = clipboard.get_image() {
    let cache_dir = root.join(CACHE_DIR);
    std::fs::create_dir_all(&cache_dir).map_err(|e| format!("缓存目录创建失败：{e}"))?;
    let tmp = cache_dir.join(clipboard_png_name(&cache_dir));
    save_clipboard_png(&img, &tmp)?;
    let opts = ImportOptions {
      structure: Structure::Flatten,
      source: Source::Copy,
      target_folder,
      paths: vec![tmp.clone()],
      imported_from: None,
      sequence_mode: SequenceMode::Scatter, // 剪贴板只有单张 PNG，永不打包
    };
    return Ok(spawn_import(app, root, opts, on_event, Some(tmp)));
  }

  // 文件列表分支：资源管理器复制的文件 / 文件夹（CF_HDROP）
  let files = clipboard.get().file_list().unwrap_or_default();
  if files.is_empty() {
    return Err("剪贴板里没有图片或文件".to_string());
  }
  let opts = ImportOptions {
    structure: Structure::Flatten,
    source: Source::Copy,
    target_folder,
    paths: files,
    imported_from: None,
    sequence_mode: SequenceMode::Scatter, // 剪贴板恒分散（4.1 定案）
  };
  Ok(spawn_import(app, root, opts, on_event, None))
}

/// 取消导入任务：置位取消标记，工作线程在下一个文件处理前中断（数量如实进 Finished）
#[tauri::command]
pub fn cancel_import(state: State<AppState>, task_id: u64) -> Result<(), String> {
  let registry = state.imports.lock().map_err(|e| e.to_string())?;
  match registry.get(&task_id) {
    Some(flag) => {
      flag.store(true, Ordering::Relaxed);
      Ok(())
    }
    None => Err("导入任务不存在或已结束".to_string()),
  }
}

/// 冲突逐项决策（PRD 10_重复添加提示）：导入挂起的 hash 冲突，前端逐个弹窗收集
/// 用户决策后一次性提交；逐项执行、单项失败不阻塞，folders.json 一次落盘
#[tauri::command]
pub fn resolve_conflicts(
  state: State<AppState>,
  items: Vec<crate::services::resolve::ConflictResolution>,
  source: String,
) -> Result<crate::services::resolve::ResolveReport, String> {
  let source = parse_source(&source)?;
  let mut guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_mut().ok_or_else(|| "请先打开素材库".to_string())?;
  Ok(crate::services::resolve::resolve_conflicts(lib, &items, source))
}

/// 短锁拿库 root 并校验库存在（命令唯一需要的共享状态）
pub(crate) fn current_root(app: &AppHandle) -> Result<PathBuf, String> {
  let state = app.state::<AppState>();
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  guard.as_ref().map(|l| l.root.clone()).ok_or_else(|| "请先打开素材库再导入".to_string())
}

/// 短锁校验目标文件夹存在（None / "root" 直接放行），失败在 spawn 前即时报错
fn ensure_target(app: &AppHandle, target_folder: &Option<String>) -> Result<(), String> {
  let Some(id) = target_folder else { return Ok(()) };
  if id == ROOT_FOLDER_ID {
    return Ok(());
  }
  let state = app.state::<AppState>();
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库再导入".to_string())?;
  if lib.index.folders.contains_key(id) {
    Ok(())
  } else {
    Err("目标文件夹不存在，可能已被删除".to_string())
  }
}

/// 注册任务 + 起工作线程：线程内 run_import（分段短锁），结束注销任务、清理临时文件
pub(crate) fn spawn_import(
  app: AppHandle,
  root: PathBuf,
  opts: ImportOptions,
  on_event: Channel<ImportEvent>,
  cleanup: Option<PathBuf>,
) -> u64 {
  let task_id = NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed);
  let cancel = Arc::new(AtomicBool::new(false));
  match app.state::<AppState>().imports.lock() {
    Ok(mut registry) => {
      registry.insert(task_id, cancel.clone());
    }
    Err(_) => log::error!("导入任务 {task_id} 注册失败（锁冲突），该任务将无法取消"),
  }
  std::thread::spawn(move || {
    let state = app.state::<AppState>();
    run_import(&state.library, root, opts, move |ev| {
      if let Err(e) = on_event.send(ev) {
        log::debug!("导入事件推送失败（前端可能已关闭）：{e}");
      }
    }, &cancel);
    if let Some(p) = cleanup.filter(|p| p.exists()) {
      let _ = std::fs::remove_file(&p);
    }
    if let Ok(mut registry) = state.imports.lock() {
      registry.remove(&task_id);
    };
  });
  task_id
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

fn parse_sequence_mode(s: &str) -> Result<SequenceMode, String> {
  match s {
    "pack" => Ok(SequenceMode::Pack),
    "scatter" => Ok(SequenceMode::Scatter),
    _ => Err(format!("未知的序列帧模式：{s}（可选 pack / scatter）")),
  }
}

// ---------- 文件夹树与 Placement 列表（导入弹窗 / 中间面板） ----------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderNode {
  pub id: String,
  pub name: String,
  /// 该文件夹递归 Placement 总数（含全部子文件夹；2026-07-25 用户定：整洁层级下直接数会大面积为 0）
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
      .map(|f| {
        let children = children_of(lib, Some(&f.id));
        let direct = lib.index.placements_by_folder.get(&f.id).map(|v| v.len()).unwrap_or(0) as u32;
        let recursive = direct + children.iter().map(|c| c.count).sum::<u32>();
        FolderNode { id: f.id.clone(), name: f.name.clone(), count: recursive, children }
      })
      .collect()
  }
  children_of(lib, None)
}

/// 列出 Placement：folder=None 全部；Some(id) 指定文件夹（"root" 表示根目录）
#[tauri::command]
pub fn list_placements(state: State<AppState>, folder: Option<String>) -> Result<Vec<PlacementView>, String> {
  let guard = state.library.lock().map_err(|e| e.to_string())?;
  let lib = guard.as_ref().ok_or_else(|| "请先打开素材库".to_string())?;
  Ok(placement_views(lib, folder.as_deref()))
}

/// Placement → 视图：资源在索引中缺失的脏 Placement 跳过
fn placement_views(lib: &Library, folder: Option<&str>) -> Vec<PlacementView> {
  let items: Vec<&Placement> = match folder {
    Some(id) => lib.index.placements_by_folder.get(id).map(|v| v.iter().collect()).unwrap_or_default(),
    None => lib.folders.items.iter().collect(),
  };
  items.into_iter().filter_map(|p| lib.index.assets.get(&p.asset).map(|m| placement_view(&lib.root, m, p, &lib.format_icons))).collect()
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::library;
  use std::sync::Mutex;

  struct Harness {
    _dir: tempfile::TempDir,
    lib: Mutex<Option<Library>>,
    root: PathBuf,
  }

  fn make_lib() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("测试库");
    let lib = library::create(&root).unwrap();
    Harness { _dir: dir, lib: Mutex::new(Some(lib)), root }
  }

  /// 同步跑导入（事件丢弃）
  fn import(h: &Harness, paths: Vec<PathBuf>, structure: Structure) {
    let opts = ImportOptions {
      structure,
      source: Source::Copy,
      target_folder: None,
      paths,
      imported_from: None,
      sequence_mode: SequenceMode::Scatter,
    };
    run_import(&h.lib, h.root.clone(), opts, |_| {}, &AtomicBool::new(false));
  }

  fn with_lib<R>(h: &Harness, f: impl FnOnce(&Library) -> R) -> R {
    f(h.lib.lock().unwrap().as_ref().unwrap())
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
  fn folder_tree_nests_and_counts_recursive_placements() {
    let h = make_lib();
    let src = h._dir.path().join("参考");
    std::fs::create_dir_all(src.join("人物")).unwrap();
    std::fs::write(src.join("a.txt"), "a").unwrap();
    std::fs::write(src.join("人物/b.txt"), "b").unwrap();
    import(&h, vec![src], Structure::Keep);

    let tree = with_lib(&h, folder_tree);
    assert_eq!(tree.len(), 1);
    // count = 递归总数：父级"参考"= 自身 a.txt 1 + 子文件夹"人物" b.txt 1
    assert_eq!((tree[0].name.as_str(), tree[0].count), ("参考", 2));
    assert_eq!((tree[0].children[0].name.as_str(), tree[0].children[0].count), ("人物", 1));
  }

  #[test]
  fn placement_views_join_meta_and_skip_missing() {
    let h = make_lib();
    let png = h._dir.path().join("图.png");
    image::RgbaImage::from_pixel(10, 20, image::Rgba([0, 0, 0, 255])).save(&png).unwrap();
    import(&h, vec![png], Structure::Flatten);

    let views = with_lib(&h, |l| placement_views(l, Some(ROOT_FOLDER_ID)));
    assert_eq!(views.len(), 1);
    assert_eq!((views[0].w, views[0].h), (Some(10), Some(20)));
    assert_eq!(views[0].ext, "png");
    assert!(views[0].origin);
    assert!(views[0].thumb_path.as_ref().unwrap().ends_with("_thumbnail.webp"));
    assert_eq!(views[0].thumb_kind, Some(crate::services::views::ThumbKind::Asset));

    // 封面帧透传（5.3 卡片悬停播放范围起点）：未设置为 None；设置后随视图下发
    assert_eq!(views[0].thumb_frame, None);
    let id = views[0].asset_id.clone();
    h.lib.lock().unwrap().as_mut().unwrap().index.assets.get_mut(&id).unwrap().thumb_frame =
      Some("t:3.500".into());
    let views = with_lib(&h, |l| placement_views(l, Some(ROOT_FOLDER_ID)));
    assert_eq!(views[0].thumb_frame.as_deref(), Some("t:3.500"));

    // 资源在索引中缺失的脏 Placement 跳过
    h.lib.lock().unwrap().as_mut().unwrap().folders.items.push(Placement {
      asset: "AN-不存在".into(),
      folder: ROOT_FOLDER_ID.into(),
      name: "幽灵.txt".into(),
      order: 1,
      origin: false,
    });
    assert_eq!(with_lib(&h, |l| placement_views(l, None).len()), 1);
  }

  #[cfg(windows)]
  #[test]
  fn import_non_image_falls_back_to_format_icon() {
    let h = make_lib();
    let fla = h._dir.path().join("工程.fla");
    std::fs::write(&fla, "fake fla bytes").unwrap();
    import(&h, vec![fla], Structure::Flatten);

    let views = with_lib(&h, |l| placement_views(l, Some(ROOT_FOLDER_ID)));
    assert_eq!(views.len(), 1);
    // 无内容缩略图 → 回退系统格式图标缓存（cache/thumbs，info 内无 shell 产物）
    let thumb = views[0].thumb_path.as_deref().unwrap();
    assert!(thumb.ends_with("fla.webp") && thumb.contains("thumbs"), "应回退格式图标缓存：{thumb}");
    assert_eq!(views[0].thumb_kind, Some(crate::services::views::ThumbKind::FormatIcon));
    assert!(h.root.join("cache/thumbs/fla.webp").is_file());
    assert!(with_lib(&h, |l| l.format_icons.contains("fla")));
    // 重开库后内存集从缓存扫描恢复
    let reopened = library::open(&h.root).unwrap();
    assert!(reopened.format_icons.contains("fla"));
  }

  #[test]
  fn parse_modes_reject_unknown_values() {
    assert!(parse_structure("flatten").is_ok());
    assert!(parse_structure("树形").is_err());
    assert!(parse_source("move").is_ok());
    assert!(parse_source("拖拽").is_err());
    assert!(parse_sequence_mode("pack").is_ok());
    assert!(parse_sequence_mode("打包").is_err());
  }

  #[test]
  fn detect_sequences_groups_frames_and_skips_non_sequences() {
    let dir = tempfile::tempdir().unwrap();
    let png = |name: &str| {
      let p = dir.path().join(name);
      image::RgbaImage::from_pixel(4, 4, image::Rgba([0, 0, 0, 255])).save(&p).unwrap();
    };
    for n in 1..=3 {
      png(&format!("走路{n:03}.png"));
    }
    png("单张.png");
    let sub = dir.path().join("子");
    std::fs::create_dir_all(&sub).unwrap();
    for n in 1..=4 {
      image::RgbaImage::from_pixel(4, 4, image::Rgba([1, 1, 1, 255])).save(sub.join(format!("跑_{n}.png"))).unwrap();
    }

    let s = |p: &Path| p.to_string_lossy().into_owned();
    let groups = detect_sequences(vec![s(dir.path())]);
    assert_eq!(groups.len(), 2, "同目录成组、子目录递归展开：{groups:?}");
    let walk = groups.iter().find(|g| g.name == "走路").unwrap();
    assert_eq!((walk.ext.as_str(), walk.frame_count), ("png", 3));
    assert_eq!(groups.iter().find(|g| g.name == "跑").unwrap().frame_count, 4);
    // 无序列帧 → 空（走 09 现状流程）
    assert!(detect_sequences(vec![s(&dir.path().join("单张.png"))]).is_empty());
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
