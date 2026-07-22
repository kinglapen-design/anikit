//! 导入任务编排：四阶段并行流水线，UI 主线程全程不被阻塞。
//!
//! 1. 并行 sha256（rayon 默认线程池，锁外 IO）；2. 一次短锁全量查重（冲突分离、
//!    预分配 asset id、keep 建层级发 folders）；3. 并行 复制/移动+缩略图+metadata（锁外）；
//! 4. mpsc 消费者串行注册 Placement/索引并发 Asset 事件（到一个显示一个）。
//!
//! 分段短锁纪律不变：锁内只做查重、Placement 追加、索引插入、folders.json 节流落盘
//! （脏且距上次 ≥2s 才落，结束强制落）。cancel 置位后不再领取新文件，进行中的写完。
//! hash 冲突不自动链接（PRD 10_重复添加提示）：收集 ConflictItem 挂起，待用户决策。

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Mutex};
use std::time::{Duration, Instant};

use rayon::prelude::*;

use crate::core::library::{self, Library};
use crate::core::models::{new_asset_id, AssetMeta};
use crate::services::import as ops;
use crate::services::import::{
  ConflictItem, FailedItem, ImportEvent, ImportOptions, ImportOutcome, PendingFile, Source, Structure,
  ROOT_FOLDER_ID,
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

/// 阶段 2 产物：待建档工作项（ID 与落点已预分配）
struct WorkItem {
  file_idx: usize,
  hash: String,
  asset_id: String,
  target_folder: String,
}

/// 同批次重复：延后到 twin 资源注册完成（元数据齐备）再组装冲突项
struct DeferredConflict {
  file_idx: usize,
  target_folder: String,
  twin_id: String,
}

/// 自动合并项（PRD 01-2.3 importedFrom）：inbox 渠道导入撞 hash 且已有资产同渠道，
/// 不建冲突、不改名弹窗——已有资产改名为新文件真实名，新文件作为重复丢弃
struct MergeItem {
  file_idx: usize,
  asset_id: String,
}

struct Phase2 {
  work: Vec<WorkItem>,
  deferred: Vec<DeferredConflict>,
  merges: Vec<MergeItem>,
}

/// 阶段 3 → 4 的消息：建档成功（工作项 + 元数据）/ 失败（原因）
enum Phase3Msg {
  Built { item: WorkItem, asset: AssetMeta },
  Broken { file_idx: usize, reason: String },
}

/// 导入执行体（工作线程调；单测同步直调）。四阶段并行流水线，语义与串行版一致。
pub fn run_import(
  lib: &Mutex<Option<Library>>,
  root: PathBuf,
  opts: ImportOptions,
  on_event: impl Fn(ImportEvent) + Sync,
  cancel: &AtomicBool,
) -> ImportOutcome {
  let mut state = TaskState::new();
  let target = opts.target_folder.clone().unwrap_or_else(|| ROOT_FOLDER_ID.to_string());

  // 阶段 1：锁外扫描展开 → started（扫描+哈希即"扫描中"）→ 并行 sha256
  let files = ops::collect(&opts.paths, opts.structure, &mut state.outcome.failed);
  on_event(ImportEvent::Started { total: files.len() as u32 });
  let ready = hash_all(&files, cancel, &mut state);

  // 阶段 2：keep 建层级发 folders（层级先行）→ 一次短锁全量查重
  if opts.structure == Structure::Keep {
    if let Err(reason) = prepare_folders(&mut state, lib, &target, &files) {
      return abort(state, lib, &on_event, &files, &ready, reason, "导入层级准备失败");
    }
    on_event(ImportEvent::Folders);
  }
  let mut phase2 = match dedup_phase(&mut state, lib, &root, &target, &ready, &files, opts.imported_from.as_deref()) {
    Ok(p) => p,
    Err(reason) => return abort(state, lib, &on_event, &files, &ready, reason, "导入查重失败"),
  };

  // 自动合并（inbox 渠道撞 hash）：改名 + 丢弃重复源，先于管线发 Asset 事件
  apply_merges(&mut state, lib, &root, &files, std::mem::take(&mut phase2.merges), &on_event);

  // 阶段 3+4：并行建档 → 消费者串行注册 + 事件
  pipeline(&mut state, lib, &root, opts.source, opts.imported_from.as_deref(), &files, phase2, &on_event, cancel);
  finish(state, lib, &on_event)
}

/// 阶段整体失败：已就绪文件全部进 failed（原因中文），收尾落盘并发 Finished
fn abort(
  mut state: TaskState,
  lib: &Mutex<Option<Library>>,
  on_event: &impl Fn(ImportEvent),
  files: &[PendingFile],
  ready: &[(usize, String)],
  reason: String,
  log_prefix: &str,
) -> ImportOutcome {
  log::error!("{log_prefix}：{reason}");
  for (idx, _) in ready {
    state
      .outcome
      .failed
      .push(FailedItem { path: files[*idx].path.to_string_lossy().into_owned(), reason: reason.clone() });
  }
  finish(state, lib, on_event)
}

/// 阶段 1：并行 sha256（锁外 IO 密集）；cancel 置位的项不再领取（None）。
/// 返回就绪清单（文件索引 + hash）；哈希失败的进 failed。
fn hash_all(files: &[PendingFile], cancel: &AtomicBool, state: &mut TaskState) -> Vec<(usize, String)> {
  let hashed: Vec<Option<Result<String, String>>> = files
    .par_iter()
    .map(|f| {
      if cancel.load(Ordering::Relaxed) {
        return None;
      }
      Some(ops::sha256_file(&f.path).map_err(|e| format!("文件读取失败：{e}")))
    })
    .collect();
  let mut ready = Vec::new();
  for (idx, h) in hashed.into_iter().enumerate() {
    match h {
      None => {}
      Some(Ok(hash)) => ready.push((idx, hash)),
      Some(Err(reason)) => {
        state.outcome.failed.push(FailedItem { path: files[idx].path.to_string_lossy().into_owned(), reason })
      }
    }
  }
  ready
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

/// 阶段 2（一次短锁）：全量查重。命中库内 → 同渠道（inbox）自动合并，否则立即组装冲突
/// （源侧锁外补）；命中同批次 → 延后冲突；未命中 → 预分配 ID + 解析落点生成工作项
#[allow(clippy::too_many_arguments)]
fn dedup_phase(
  state: &mut TaskState,
  lib: &Mutex<Option<Library>>,
  root: &Path,
  target: &str,
  ready: &[(usize, String)],
  files: &[PendingFile],
  merge_mark: Option<&str>,
) -> Result<Phase2, String> {
  let root_buf = root.to_path_buf();
  let mut phase = Phase2 { work: Vec::new(), deferred: Vec::new(), merges: Vec::new() };
  let mut immediate: Vec<(usize, ConflictItem)> = Vec::new();
  let mut no_folder: Vec<usize> = Vec::new();
  with_lib(lib, state, |l| {
    let by_hash: HashMap<&str, &AssetMeta> = l.index.assets.values().map(|a| (a.hash.as_str(), a)).collect();
    let mut batch: HashMap<String, String> = HashMap::new(); // hash → 同批次 twin asset id
    for (idx, hash) in ready {
      match classify_one(l, &by_hash, &mut batch, &root_buf, target, &files[*idx], *idx, hash, merge_mark) {
        Classified::Conflict(i, c) => immediate.push((i, c)),
        Classified::Merge(i, id) => phase.merges.push(MergeItem { file_idx: i, asset_id: id }),
        Classified::Deferred(d) => phase.deferred.push(d),
        Classified::Work(w) => phase.work.push(w),
        Classified::NoFolder(i) => no_folder.push(i),
      }
    }
  })?;
  for idx in no_folder {
    state.outcome.failed.push(FailedItem {
      path: files[idx].path.to_string_lossy().into_owned(),
      reason: "目标文件夹不存在，可能已被删除".to_string(),
    });
  }
  for (idx, mut c) in immediate {
    fill_source_side(&mut c, &files[idx]);
    state.outcome.conflicts.push(c);
  }
  Ok(phase)
}

/// 查重分类结果（阶段 2 锁内）
enum Classified {
  Conflict(usize, ConflictItem),
  Merge(usize, String),
  Deferred(DeferredConflict),
  Work(WorkItem),
  NoFolder(usize),
}

/// 查重分类单项：命中库内 → 同渠道（inbox）自动合并，否则冲突；命中同批次 → 延后冲突；
/// 未命中 → 预分配 ID 建工作项
#[allow(clippy::too_many_arguments)]
fn classify_one(
  l: &Library,
  by_hash: &HashMap<&str, &AssetMeta>,
  batch: &mut HashMap<String, String>,
  root: &Path,
  target: &str,
  file: &PendingFile,
  idx: usize,
  hash: &str,
  merge_mark: Option<&str>,
) -> Classified {
  let Some(folder_id) = ops::find_folder(l, target, &file.chain) else {
    return Classified::NoFolder(idx);
  };
  if let Some(meta) = by_hash.get(hash) {
    // 自动合并判定：本次导入与已有资产同为 inbox 渠道（正式资产绝不被自动改名）
    if merge_mark.is_some() && meta.imported_from.as_deref() == merge_mark {
      Classified::Merge(idx, meta.id.clone())
    } else {
      Classified::Conflict(idx, conflict_of(l, root, meta, folder_id))
    }
  } else if let Some(twin) = batch.get(hash) {
    Classified::Deferred(DeferredConflict { file_idx: idx, target_folder: folder_id, twin_id: twin.clone() })
  } else {
    let asset_id = new_asset_id();
    batch.insert(hash.to_string(), asset_id.clone());
    Classified::Work(WorkItem { file_idx: idx, hash: hash.to_string(), asset_id, target_folder: folder_id })
  }
}

/// 阶段 3+4：rayon 并行建档（锁外重活）→ mpsc 消费者串行注册 + Asset 事件流
#[allow(clippy::too_many_arguments)]
fn pipeline(
  state: &mut TaskState,
  lib: &Mutex<Option<Library>>,
  root: &Path,
  source: Source,
  imported_from: Option<&str>,
  files: &[PendingFile],
  phase2: Phase2,
  on_event: &(impl Fn(ImportEvent) + Sync),
  cancel: &AtomicBool,
) {
  let (tx, rx) = mpsc::channel::<Phase3Msg>();
  let mut deferred = group_deferred(phase2.deferred);
  std::thread::scope(|s| {
    let consumer = s.spawn(move || {
      for msg in rx {
        match msg {
          Phase3Msg::Built { item, asset } => register_one(state, lib, root, files, item, asset, &mut deferred, on_event),
          Phase3Msg::Broken { file_idx, reason } => state.outcome.failed.push(FailedItem {
            path: files[file_idx].path.to_string_lossy().into_owned(),
            reason,
          }),
        }
      }
      drain_orphan_deferred(state, files, deferred);
    });
    produce(phase2.work, root, source, imported_from, files, &tx, cancel);
    drop(tx); // 关闭通道，消费者收完尾部结果后退出
    if consumer.join().is_err() {
      log::error!("导入注册线程异常（panic）");
    }
  });
}

/// 阶段 3 生产者：并行执行 复制/移动 + 缩略图 + metadata.json；cancel 后不再领新文件
fn produce(
  work: Vec<WorkItem>,
  root: &Path,
  source: Source,
  imported_from: Option<&str>,
  files: &[PendingFile],
  tx: &mpsc::Sender<Phase3Msg>,
  cancel: &AtomicBool,
) {
  work.into_par_iter().for_each(|item| {
    if cancel.load(Ordering::Relaxed) {
      return;
    }
    let msg = match ops::build_asset_with_id(root, &files[item.file_idx], &item.hash, &item.asset_id, source, imported_from) {
      Ok(asset) => Phase3Msg::Built { item, asset },
      Err(reason) => Phase3Msg::Broken { file_idx: item.file_idx, reason },
    };
    if tx.send(msg).is_err() {
      log::error!("导入结果通道异常，结果丢失");
    }
  });
}

/// 阶段 4 单项：锁内注册 Placement + 索引 → 发 Asset；twin 完成即组装其延后冲突
fn register_one(
  state: &mut TaskState,
  lib: &Mutex<Option<Library>>,
  root: &Path,
  files: &[PendingFile],
  item: WorkItem,
  asset: AssetMeta,
  deferred: &mut HashMap<String, Vec<DeferredConflict>>,
  on_event: &impl Fn(ImportEvent),
) {
  state.dirty = true; // Placement 必改 folders.json，先标脏让节流落盘生效
  let root_buf = root.to_path_buf();
  let name = ops::file_name(&files[item.file_idx].path);
  let asset_id = asset.id.clone();
  let result = with_lib(lib, state, move |l| {
    let placement = ops::add_placement(l, asset_id.clone(), item.target_folder.clone(), name, true);
    let view = placement_view(&root_buf, &asset, &placement);
    let twins = deferred.remove(&asset_id).unwrap_or_default();
    let conflicts =
      twins.into_iter().map(|d| (d.file_idx, conflict_of(l, &root_buf, &asset, d.target_folder))).collect::<Vec<_>>();
    l.index.assets.insert(asset_id, asset);
    (view, conflicts)
  });
  collect_registered(state, files, item.file_idx, result, on_event);
}

/// 注册结果入帐：成功发 Asset 事件并补源侧冲突数据；失败进 failed
fn collect_registered(
  state: &mut TaskState,
  files: &[PendingFile],
  file_idx: usize,
  result: Result<(PlacementView, Vec<(usize, ConflictItem)>), String>,
  on_event: &impl Fn(ImportEvent),
) {
  match result {
    Ok((view, conflicts)) => {
      state.outcome.imported += 1;
      on_event(ImportEvent::Asset { placement: view });
      for (idx, mut c) in conflicts {
        fill_source_side(&mut c, &files[idx]);
        state.outcome.conflicts.push(c);
      }
    }
    Err(reason) => state
      .outcome
      .failed
      .push(FailedItem { path: files[file_idx].path.to_string_lossy().into_owned(), reason }),
  }
}

/// twin 建档失败的延后冲突：内容不在库内，提示用户重试导入即可正常建档
fn drain_orphan_deferred(state: &mut TaskState, files: &[PendingFile], deferred: HashMap<String, Vec<DeferredConflict>>) {
  for ds in deferred.into_values() {
    for d in ds {
      state.outcome.failed.push(FailedItem {
        path: files[d.file_idx].path.to_string_lossy().into_owned(),
        reason: "同批次的相同文件入库失败，该文件请重试导入".to_string(),
      });
    }
  }
}

fn group_deferred(list: Vec<DeferredConflict>) -> HashMap<String, Vec<DeferredConflict>> {
  let mut map: HashMap<String, Vec<DeferredConflict>> = HashMap::new();
  for d in list {
    map.entry(d.twin_id.clone()).or_default().push(d);
  }
  map
}

/// 自动合并（PRD 01-2.3 importedFrom）：锁外改名 + 丢弃重复源 → 短锁更新索引与
/// origin Placement 显示名（同文件夹唯一化）→ 发 Asset 事件（收件箱视图与进度照常流入）。
/// 同名纯重复过境：merge_rename 已静默丢弃，无可见变化、不发事件。
fn apply_merges(
  state: &mut TaskState,
  lib: &Mutex<Option<Library>>,
  root: &Path,
  files: &[PendingFile],
  merges: Vec<MergeItem>,
  on_event: &impl Fn(ImportEvent),
) {
  for m in merges {
    let path = files[m.file_idx].path.to_string_lossy().into_owned();
    let meta = match ops::merge_rename(root, &files[m.file_idx], &m.asset_id) {
      Ok((meta, renamed)) => {
        if !renamed {
          continue;
        }
        meta
      }
      Err(reason) => {
        state.outcome.failed.push(FailedItem { path, reason });
        continue;
      }
    };
    state.dirty = true; // origin Placement 改名改 folders.json
    let root_buf = root.to_path_buf();
    let view = with_lib(lib, state, |l| {
      l.index.assets.insert(m.asset_id.clone(), meta.clone());
      rename_origin_placement(l, &m.asset_id, &meta.origin_name).map(|p| placement_view(&root_buf, &meta, &p))
    });
    match view {
      Ok(Some(v)) => {
        state.outcome.imported += 1;
        on_event(ImportEvent::Asset { placement: v });
      }
      // 异常数据（缺 origin Placement）：metadata 已改名，仅记日志不中断
      Ok(None) => log::error!("自动合并：资源 {} 缺少 origin Placement", m.asset_id),
      Err(reason) => state.outcome.failed.push(FailedItem { path, reason }),
    }
  }
}

/// origin Placement 显示名改为新文件名（同文件夹唯一化），重建 Placement 索引；
/// 返回改名后的 Placement（缺 origin 为 None，异常数据防御）
fn rename_origin_placement(l: &mut Library, asset_id: &str, new_name: &str) -> Option<crate::core::models::Placement> {
  let pos = l.folders.items.iter().position(|p| p.asset == asset_id && p.origin)?;
  let folder = l.folders.items[pos].folder.clone();
  let existing: HashSet<String> = l
    .folders
    .items
    .iter()
    .enumerate()
    .filter(|(i, p)| *i != pos && p.folder == folder)
    .map(|(_, p)| p.name.clone())
    .collect();
  l.folders.items[pos].name = library::unique_name(&existing, new_name);
  l.index.rebuild_placements(&l.folders.items);
  Some(l.folders.items[pos].clone())
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
#[path = "import_task/tests.rs"]
mod tests;
