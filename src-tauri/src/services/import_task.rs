//! 导入任务编排：工作线程执行，UI 主线程全程不被阻塞。
//!
//! 分段短锁纪律（窗口不卡的前提）：
//! - 锁外：递归扫描、sha256 流式、文件复制 / 移动、缩略图、写 metadata.json；
//! - 锁内（每次毫秒级）：hash 查重、Placement 追加（unique_name 读现有名）、
//!   文件夹 get-or-create、索引插入、folders.json 节流落盘（脏且距上次 ≥2s 才落，结束强制落）。
//!
//! hash 冲突不自动链接（PRD 10_重复添加提示）：收集 ConflictItem 挂起，
//! finished 后由前端逐项弹窗决策（resolve_conflicts）。
//! 事件经 on_event 回调逐条推送（命令层接 Channel；单测收集到 Vec 断言）。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::core::library::{self, Library};
use crate::core::models::AssetMeta;
use crate::services::import as ops;
use crate::services::import::{
  ConflictItem, FailedItem, ImportEvent, ImportOptions, ImportOutcome, PendingFile, Structure, ROOT_FOLDER_ID,
};
use crate::services::thumbnail;
use crate::services::views::{placement_view, thumb_path, PlacementView};

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
    Self {
      dirty: false,
      last_save: Instant::now(),
      outcome: ImportOutcome { imported: 0, conflicts: Vec::new(), failed: Vec::new() },
    }
  }
}

/// 导入执行体（工作线程调；单测同步直调）。
/// lib 为共享库句柄（每次仅毫秒级短锁）；cancel 置位后在下一个文件处理前中断，
/// 已导入数量与已收集冲突如实进 Finished 事件与返回值。
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

  // 3. 逐文件：锁外重活 + 锁内登记，到一个推一个；冲突挂起不发 Asset
  for file in &files {
    if cancel.load(Ordering::Relaxed) {
      log::info!("导入任务被取消：已导入 {} 个，冲突挂起 {} 个", state.outcome.imported, state.outcome.conflicts.len());
      break;
    }
    match import_one(&mut state, lib, &root, file, &target, &opts) {
      Ok(Some(view)) => {
        state.outcome.imported += 1;
        on_event(ImportEvent::Asset { placement: view });
      }
      Ok(None) => {} // 冲突已挂起收集，待 finished 后用户决策
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

/// 单文件：锁外哈希 → 锁内查重 →（命中：挂起冲突）/（未命中：锁外建档 → 锁内登记）
fn import_one(
  state: &mut TaskState,
  lib: &Mutex<Option<Library>>,
  root: &Path,
  file: &PendingFile,
  target: &str,
  opts: &ImportOptions,
) -> Result<Option<PlacementView>, String> {
  let hash = ops::sha256_file(&file.path).map_err(|e| format!("文件读取失败：{e}"))?;
  // hash 命中：收集冲突项挂起（库侧数据锁内取，源侧数据锁外补），源文件保留不动
  let hit = with_lib(lib, state, |l| {
    l.index.assets.values().find(|a| a.hash == hash).map(|meta| {
      let folder_id =
        ops::find_folder(l, target, &file.chain).ok_or_else(|| "目标文件夹不存在，可能已被删除".to_string())?;
      Ok::<_, String>(conflict_of(l, root, meta, folder_id))
    })
  })?;
  let Some(hit) = hit else {
    // 未命中：锁外重活（复制 / 移动 + 缩略图 + metadata.json），路径由新 ID 推导不依赖共享状态
    let asset = ops::build_asset(root, file, &hash, opts.source)?;
    let view = register_new_asset(state, lib, root, file, target, asset)?;
    return Ok(Some(view));
  };
  let mut item = hit?;
  fill_source_side(&mut item, file);
  state.outcome.conflicts.push(item);
  Ok(None)
}

/// 锁内组装冲突项的库侧数据（已有资源 + 落点）；源文件侧字段留空由 fill_source_side 补
fn conflict_of(l: &Library, root: &Path, meta: &AssetMeta, folder_id: String) -> ConflictItem {
  // 显示名与所在文件夹：优先 origin Placement，无则任一 Placement（正常数据必有 origin）
  let placements = l.index.placements_by_asset.get(&meta.id).map(Vec::as_slice).unwrap_or(&[]);
  let chosen = placements.iter().find(|p| p.origin).or_else(|| placements.first());
  ConflictItem {
    source_path: PathBuf::new(),
    source_name: String::new(),
    source_size: 0,
    source_w: None,
    source_h: None,
    target_folder_name: folder_display_name(l, &folder_id),
    target_folder: folder_id,
    existing_asset_id: meta.id.clone(),
    existing_name: chosen.map(|p| p.name.clone()).unwrap_or_else(|| meta.name.clone()),
    existing_w: meta.width,
    existing_h: meta.height,
    existing_size: meta.size,
    existing_ext: meta.ext.clone(),
    existing_thumb_path: thumb_path(root, meta),
    existing_folder_name: chosen.map(|p| folder_display_name(l, &p.folder)).unwrap_or_default(),
  }
}

/// 文件夹显示名："root" → "根目录"；未知 ID → "未知文件夹"（防御，正常不出现）
fn folder_display_name(l: &Library, folder_id: &str) -> String {
  if folder_id == ROOT_FOLDER_ID {
    return "根目录".to_string();
  }
  l.index.folders.get(folder_id).map(|f| f.name.clone()).unwrap_or_else(|| "未知文件夹".to_string())
}

/// 锁外补冲突项的源文件侧数据：文件名 / 字节数 / 图片宽高（probe 失败即非图片，None）
fn fill_source_side(item: &mut ConflictItem, file: &PendingFile) {
  item.source_path = file.path.clone();
  item.source_name = ops::file_name(&file.path);
  item.source_size = std::fs::metadata(&file.path).map(|m| m.len()).unwrap_or(0);
  let (w, h) = thumbnail::probe(&file.path).map(|(w, h)| (Some(w), Some(h))).unwrap_or((None, None));
  item.source_w = w;
  item.source_h = h;
}

/// 锁内登记新资源：文件夹定位 → origin Placement 追加 → 索引插入 → 标脏 + 节流落盘
fn register_new_asset(
  state: &mut TaskState,
  lib: &Mutex<Option<Library>>,
  root: &Path,
  file: &PendingFile,
  target: &str,
  asset: AssetMeta,
) -> Result<PlacementView, String> {
  state.dirty = true; // Placement 必改 folders.json，先标脏让节流落盘生效
  let root_buf = root.to_path_buf();
  with_lib(lib, state, move |l| {
    let folder_id =
      ops::find_folder(l, target, &file.chain).ok_or_else(|| "目标文件夹不存在，可能已被删除".to_string())?;
    let placement = ops::add_placement(l, asset.id.clone(), folder_id, ops::file_name(&file.path), true);
    let view = placement_view(&root_buf, &asset, &placement);
    l.index.assets.insert(asset.id.clone(), asset);
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
  log::info!(
    "导入结束：新导入 {}、冲突挂起 {}、失败 {}",
    outcome.imported,
    outcome.conflicts.len(),
    outcome.failed.len()
  );
  on_event(ImportEvent::Finished {
    imported: outcome.imported,
    conflicts: outcome.conflicts.clone(),
    failed: outcome.failed.clone(),
  });
  outcome
}

#[cfg(test)]
mod tests;
