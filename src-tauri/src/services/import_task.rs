//! 导入任务编排：工作线程执行，UI 主线程全程不被阻塞。
//!
//! 分段短锁纪律（窗口不卡的前提）：
//! - 锁外：递归扫描、sha256 流式、文件复制 / 移动、缩略图、写 metadata.json；
//! - 锁内（每次毫秒级）：hash 查重、Placement 追加（unique_name 读现有名）、
//!   文件夹 get-or-create、索引插入、folders.json 节流落盘（脏且距上次 ≥2s 才落，结束强制落）。
//!
//! 事件经 on_event 回调逐条推送（命令层接 Channel；单测收集到 Vec 断言）。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::core::library::{self, Library};
use crate::core::models::AssetMeta;
use crate::services::import as ops;
use crate::services::import::{
  FailedItem, ImportEvent, ImportOptions, ImportOutcome, PendingFile, Structure, ROOT_FOLDER_ID,
};
use crate::services::views::{placement_view, PlacementView};

/// folders.json 落盘节流间隔
const SAVE_INTERVAL: Duration = Duration::from_secs(2);

/// 任务运行期可变状态：落盘节流标记 + 结果汇总
struct TaskState {
  dirty: bool,
  last_save: Instant,
  outcome: ImportOutcome,
}

impl TaskState {
  fn new() -> Self {
    Self { dirty: false, last_save: Instant::now(), outcome: ImportOutcome { imported: 0, linked: 0, failed: Vec::new() } }
  }
}

/// 导入执行体（工作线程调；单测同步直调）。
/// lib 为共享库句柄（每次仅毫秒级短锁）；cancel 置位后在下一个文件处理前中断，
/// 已导入数量如实进 Finished 事件与返回值。
pub fn run_import(
  lib: &Mutex<Option<Library>>,
  root: PathBuf,
  opts: ImportOptions,
  on_event: impl Fn(ImportEvent) + Send,
  cancel: &AtomicBool,
) -> ImportOutcome {
  let mut state = TaskState::new();
  let target = opts.target_folder.clone().unwrap_or_else(|| ROOT_FOLDER_ID.to_string());

  // 1. 锁外扫描展开，总数可知
  let files = ops::collect(&opts.paths, opts.structure, &mut state.outcome.failed);
  on_event(ImportEvent::Started { total: files.len() as u32 });

  // 2. keep 模式：锁内批量建层级 → 发 Folders（前端重拉文件夹树）
  if opts.structure == Structure::Keep {
    if let Err(reason) = prepare_folders(&mut state, lib, &target, &files) {
      log::error!("导入层级准备失败：{reason}");
      for f in &files {
        state.outcome.failed.push(FailedItem { path: f.path.to_string_lossy().into_owned(), reason: reason.clone() });
      }
      return finish(state, lib, &on_event);
    }
    on_event(ImportEvent::Folders);
  }

  // 3. 逐文件：锁外重活 + 锁内登记，到一个推一个
  for file in &files {
    if cancel.load(Ordering::Relaxed) {
      log::info!("导入任务被取消：已处理 {} 个，剩余 {} 个", state.outcome.imported + state.outcome.linked, files.len());
      break;
    }
    match import_one(&mut state, lib, &root, file, &target, &opts) {
      Ok((view, linked)) => {
        if linked {
          state.outcome.linked += 1;
        } else {
          state.outcome.imported += 1;
        }
        on_event(ImportEvent::Asset { placement: view });
      }
      Err(reason) => {
        state.outcome.failed.push(FailedItem { path: file.path.to_string_lossy().into_owned(), reason });
      }
    }
  }
  finish(state, lib, &on_event)
}

/// 锁内批量建 keep 层级（get-or-create 幂等），有新建则标脏
fn prepare_folders(
  state: &mut TaskState,
  lib: &Mutex<Option<Library>>,
  target: &str,
  files: &[PendingFile],
) -> Result<(), String> {
  let created = with_lib(lib, state, |l| {
    files.iter().filter(|f| !f.chain.is_empty()).fold(false, |acc, f| ops::ensure_chain(l, target, &f.chain) || acc)
  })?;
  state.dirty |= created;
  Ok(())
}

/// 单文件：锁外哈希 → 锁内查重 →（命中：锁内登记链接）/（未命中：锁外建档 → 锁内登记）
fn import_one(
  state: &mut TaskState,
  lib: &Mutex<Option<Library>>,
  root: &Path,
  file: &PendingFile,
  target: &str,
  opts: &ImportOptions,
) -> Result<(PlacementView, bool), String> {
  let hash = ops::sha256_file(&file.path).map_err(|e| format!("文件读取失败：{e}"))?;
  let linked_id = with_lib(lib, state, |l| l.index.assets.values().find(|a| a.hash == hash).map(|a| a.id.clone()))?;

  if let Some(id) = linked_id {
    let view = register_placement(state, lib, root, file, target, id, None)?;
    return Ok((view, true));
  }
  // 未命中：锁外重活（复制 / 移动 + 缩略图 + metadata.json），路径由新 ID 推导不依赖共享状态
  let asset = ops::build_asset(root, file, &hash, opts.source)?;
  let view = register_placement(state, lib, root, file, target, asset.id.clone(), Some(asset))?;
  Ok((view, false))
}

/// 锁内登记：文件夹定位 → Placement 追加 → 索引同步 → 标脏 + 节流落盘；返回视图供事件推送
fn register_placement(
  state: &mut TaskState,
  lib: &Mutex<Option<Library>>,
  root: &Path,
  file: &PendingFile,
  target: &str,
  asset_id: String,
  new_asset: Option<AssetMeta>,
) -> Result<PlacementView, String> {
  state.dirty = true; // Placement 必改 folders.json，先标脏让节流落盘生效
  let origin = new_asset.is_some();
  let root_buf = root.to_path_buf();
  with_lib(lib, state, move |l| {
    let folder_id =
      ops::find_folder(l, target, &file.chain).ok_or_else(|| "目标文件夹不存在，可能已被删除".to_string())?;
    let placement = ops::add_placement(l, asset_id, folder_id, ops::file_name(&file.path), origin);
    let view = match new_asset {
      Some(meta) => {
        let v = placement_view(&root_buf, &meta, &placement);
        l.index.assets.insert(meta.id.clone(), meta);
        v
      }
      None => {
        let meta = l.index.assets.get(&placement.asset).expect("链接的资源必在索引中");
        placement_view(&root_buf, meta, &placement)
      }
    };
    Ok::<PlacementView, String>(view)
  })?
}

/// 短锁执行：锁中毒 / 库已关闭给中文错；f 执行后按节流尝试落盘
fn with_lib<T>(lib: &Mutex<Option<Library>>, state: &mut TaskState, f: impl FnOnce(&mut Library) -> T) -> Result<T, String> {
  let mut guard = lib.lock().map_err(|_| "应用状态异常（锁冲突），请重试".to_string())?;
  let lib_ref = guard.as_mut().ok_or_else(|| "素材库已关闭，无法导入".to_string())?;
  let out = f(lib_ref);
  save_if_due(lib_ref, state);
  Ok(out)
}

/// 节流落盘：脏且距上次 ≥ SAVE_INTERVAL 才写；失败记日志不中断（结束还会强制落）
fn save_if_due(lib: &Library, state: &mut TaskState) {
  if state.dirty && state.last_save.elapsed() >= SAVE_INTERVAL {
    match library::save_folders(lib) {
      Ok(()) => {
        state.dirty = false;
        state.last_save = Instant::now();
      }
      Err(e) => log::error!("folders.json 落盘失败：{e}"),
    }
  }
}

/// 收尾：强制落盘（有脏才写）→ 日志摘要 → Finished 事件
fn finish(state: TaskState, lib: &Mutex<Option<Library>>, on_event: &impl Fn(ImportEvent)) -> ImportOutcome {
  if state.dirty {
    match lib.lock() {
      Ok(mut guard) => {
        if let Some(l) = guard.as_mut() {
          if let Err(e) = library::save_folders(l) {
            log::error!("folders.json 最终落盘失败：{e}");
          }
        }
      }
      Err(_) => log::error!("folders.json 最终落盘失败：应用状态锁冲突"),
    }
  }
  let outcome = state.outcome;
  log::info!("导入结束：新导入 {}、链接 {}、失败 {}", outcome.imported, outcome.linked, outcome.failed.len());
  on_event(ImportEvent::Finished {
    imported: outcome.imported,
    linked: outcome.linked,
    failed: outcome.failed.clone(),
  });
  outcome
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::json_io::read_json;
  use crate::core::library::{ASSTES_DIR, FOLDERS_FILE};
  use crate::core::models::FoldersFile;
  use crate::services::import::Source;
  use std::fs;

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

  fn opts(structure: Structure, source: Source, paths: Vec<PathBuf>) -> ImportOptions {
    ImportOptions { structure, source, target_folder: None, paths }
  }

  /// 同步跑导入并收集事件
  fn run(h: &Harness, opts: ImportOptions) -> (ImportOutcome, Vec<ImportEvent>) {
    let events = Mutex::new(Vec::new());
    let cancel = AtomicBool::new(false);
    let outcome = run_import(&h.lib, h.root.clone(), opts, |e| events.lock().unwrap().push(e), &cancel);
    (outcome, events.into_inner().unwrap())
  }

  /// 事件类型序列（断言顺序用）
  fn kinds(events: &[ImportEvent]) -> Vec<&'static str> {
    events
      .iter()
      .map(|e| match e {
        ImportEvent::Started { .. } => "started",
        ImportEvent::Folders => "folders",
        ImportEvent::Asset { .. } => "asset",
        ImportEvent::Finished { .. } => "finished",
      })
      .collect()
  }

  fn with_lib<R>(h: &Harness, f: impl FnOnce(&Library) -> R) -> R {
    f(h.lib.lock().unwrap().as_ref().unwrap())
  }

  fn info_dir_count(h: &Harness) -> usize {
    fs::read_dir(h.root.join(ASSTES_DIR)).unwrap().count()
  }

  /// 造两文件源树：素材/a.txt + 素材/子目录/b.txt
  fn make_tree(h: &Harness) -> PathBuf {
    let src = h._dir.path().join("素材");
    fs::create_dir_all(src.join("子目录")).unwrap();
    fs::write(src.join("a.txt"), "aaa").unwrap();
    fs::write(src.join("子目录/b.txt"), "bbb").unwrap();
    src
  }

  #[test]
  fn flatten_imports_flat_and_streams_events() {
    let h = make_lib();
    let src = make_tree(&h);
    let (out, events) = run(&h, opts(Structure::Flatten, Source::Copy, vec![src]));

    assert_eq!((out.imported, out.linked, out.failed.len()), (2, 0, 0), "failed: {:?}", out.failed);
    // 平铺：不建 Folder，两条 Placement 落根目录
    assert!(with_lib(&h, |l| l.folders.folders.is_empty()));
    assert_eq!(with_lib(&h, |l| l.index.placements_by_folder[ROOT_FOLDER_ID].len()), 2);
    // 事件序列：started → asset×2 → finished；flatten 无 folders
    assert_eq!(kinds(&events), vec!["started", "asset", "asset", "finished"]);
    assert!(matches!(events[0], ImportEvent::Started { total: 2 }));
    assert!(matches!(events.last().unwrap(), ImportEvent::Finished { imported: 2, .. }));
  }

  #[test]
  fn keep_builds_hierarchy_first_then_streams_assets() {
    let h = make_lib();
    let src = h._dir.path().join("参考");
    fs::create_dir_all(src.join("人物")).unwrap();
    fs::write(src.join("封面.png"), "x").unwrap();
    fs::write(src.join("人物/走路.fla"), "y").unwrap();

    let (out, events) = run(&h, opts(Structure::Keep, Source::Copy, vec![src]));
    assert_eq!(out.imported, 2);
    // folders 事件先于全部 asset 事件
    assert_eq!(kinds(&events), vec!["started", "folders", "asset", "asset", "finished"]);
    // 层级含根文件夹本身：参考（顶层）→ 人物（子级），Placement 落末级
    with_lib(&h, |l| {
      let top = l.folders.folders.iter().find(|f| f.name == "参考" && f.parent.is_none()).unwrap();
      let sub = l.folders.folders.iter().find(|f| f.name == "人物").unwrap();
      assert_eq!(sub.parent.as_deref(), Some(top.id.as_str()));
      assert_eq!(l.index.placements_by_folder[&sub.id].len(), 1);
      assert_eq!(l.index.placements_by_folder[&top.id].len(), 1);
    });
  }

  #[test]
  fn copy_keeps_source_move_removes_source() {
    let h = make_lib();
    let f1 = h._dir.path().join("复制.txt");
    let f2 = h._dir.path().join("剪切.txt");
    fs::write(&f1, "1").unwrap();
    fs::write(&f2, "2").unwrap();

    run(&h, opts(Structure::Flatten, Source::Copy, vec![f1.clone()]));
    assert!(f1.exists(), "复制后源文件应仍在");
    run(&h, opts(Structure::Flatten, Source::Move, vec![f2.clone()]));
    assert!(!f2.exists(), "剪切后源文件应消失");
  }

  #[test]
  fn same_hash_links_without_duplicating_entity() {
    let h = make_lib();
    let f1 = h._dir.path().join("甲.txt");
    let f2 = h._dir.path().join("乙.txt");
    fs::write(&f1, "相同内容").unwrap();
    fs::write(&f2, "相同内容").unwrap();

    let (out, _) = run(&h, opts(Structure::Flatten, Source::Copy, vec![f1, f2]));
    assert_eq!((out.imported, out.linked), (1, 1));
    assert_eq!(info_dir_count(&h), 1);
    with_lib(&h, |l| {
      let origins = l.folders.items.iter().filter(|p| p.origin).count();
      assert_eq!((l.folders.items.len(), origins), (2, 1));
    });
  }

  #[test]
  fn duplicate_display_name_gets_counter_suffix() {
    let h = make_lib();
    let d1 = h._dir.path().join("源一");
    let d2 = h._dir.path().join("源二");
    fs::create_dir_all(&d1).unwrap();
    fs::create_dir_all(&d2).unwrap();
    fs::write(d1.join("男人.fla"), "内容一").unwrap();
    fs::write(d2.join("男人.fla"), "内容二").unwrap();

    let (out, _) = run(&h, opts(Structure::Flatten, Source::Copy, vec![d1, d2]));
    assert_eq!(out.imported, 2);
    with_lib(&h, |l| {
      let names: Vec<&str> = l.folders.items.iter().map(|p| p.name.as_str()).collect();
      assert!(names.contains(&"男人.fla"));
      assert!(names.contains(&"男人-1.fla"));
    });
  }

  #[test]
  fn mixed_files_and_folder_and_bad_path() {
    let h = make_lib();
    let single = h._dir.path().join("单文件.txt");
    fs::write(&single, "s").unwrap();
    let folder = h._dir.path().join("夹");
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join("内.txt"), "i").unwrap();
    let missing = h._dir.path().join("不存在.txt");

    let (out, _) = run(&h, opts(Structure::Keep, Source::Copy, vec![single, folder, missing]));
    assert_eq!(out.imported, 2);
    assert_eq!(out.failed.len(), 1);
    assert!(out.failed[0].reason.contains("不存在"));
  }

  #[test]
  fn import_writes_metadata_and_persists_folders() {
    let h = make_lib();
    let png = h._dir.path().join("图.PNG");
    image::RgbaImage::from_pixel(64, 32, image::Rgba([0, 255, 0, 255])).save(&png).unwrap();

    let (out, events) = run(&h, opts(Structure::Flatten, Source::Copy, vec![png]));
    assert_eq!(out.imported, 1);
    // Asset 事件带完整视图（宽高 / 缩略图绝对路径）
    let ImportEvent::Asset { placement } = &events[1] else { panic!("应为 asset 事件") };
    assert_eq!((placement.w, placement.h), (Some(64), Some(32)));
    assert!(placement.thumb_path.as_ref().unwrap().ends_with("_thumbnail.webp"));
    assert!(placement.origin);

    with_lib(&h, |l| {
      let asset = l.index.assets.values().next().unwrap();
      assert_eq!(asset.ext, "png");
      assert!(asset.hash.starts_with("sha256:"));
      let info = h.root.join(ASSTES_DIR).join(format!("{}.info", asset.id));
      assert!(info.join("metadata.json").is_file());
      assert!(info.join(asset.thumbnail.as_ref().unwrap()).is_file());
    });
    // folders.json 结束强制落盘
    let back: FoldersFile = read_json(&h.root.join(FOLDERS_FILE)).unwrap();
    assert_eq!(back.items.len(), 1);
    assert!(back.items[0].origin);
  }

  #[test]
  fn cancel_stops_mid_import_and_reports_partial_counts() {    let h = make_lib();
    let paths: Vec<PathBuf> = (0..3)
      .map(|i| {
        let p = h._dir.path().join(format!("f{i}.txt"));
        fs::write(&p, format!("内容{i}")).unwrap();
        p
      })
      .collect();

    // 第一个 asset 事件即置位取消 → 后续文件不再处理
    let cancel = AtomicBool::new(false);
    let events = Mutex::new(Vec::new());
    let outcome = run_import(
      &h.lib,
      h.root.clone(),
      opts(Structure::Flatten, Source::Copy, paths),
      |e| {
        if matches!(e, ImportEvent::Asset { .. }) {
          cancel.store(true, Ordering::Relaxed);
        }
        events.lock().unwrap().push(e);
      },
      &cancel,
    );

    assert_eq!((outcome.imported, outcome.linked, outcome.failed.len()), (1, 0, 0));
    assert_eq!(kinds(&events.into_inner().unwrap()), vec!["started", "asset", "finished"]);
    // 已导入的 1 条正常落盘
    assert_eq!(with_lib(&h, |l| l.folders.items.len()), 1);
  }

  #[test]
  fn import_event_json_shape_is_stable() {
    // 前端 TS 类型按此形状写，序列化契约变更会打爆这个用例
    let started = serde_json::to_value(ImportEvent::Started { total: 3 }).unwrap();
    assert_eq!(started, serde_json::json!({ "type": "started", "total": 3 }));
    let folders = serde_json::to_value(ImportEvent::Folders).unwrap();
    assert_eq!(folders, serde_json::json!({ "type": "folders" }));

    let view = PlacementView {
      asset_id: "AN-X".into(),
      folder_id: "root".into(),
      name: "a.png".into(),
      w: Some(1),
      h: None,
      ext: "png".into(),
      thumb_path: None,
      origin: true,
    };
    let asset = serde_json::to_value(ImportEvent::Asset { placement: view }).unwrap();
    assert_eq!(
      asset,
      serde_json::json!({
        "type": "asset",
        "placement": {
          "assetId": "AN-X", "folderId": "root", "name": "a.png",
          "w": 1, "h": null, "ext": "png", "thumbPath": null, "origin": true
        }
      })
    );

    let finished = serde_json::to_value(ImportEvent::Finished {
      imported: 1,
      linked: 2,
      failed: vec![FailedItem { path: "p".into(), reason: "r".into() }],
    })
    .unwrap();
    assert_eq!(
      finished,
      serde_json::json!({ "type": "finished", "imported": 1, "linked": 2, "failed": [{ "path": "p", "reason": "r" }] })
    );
  }
}
