//! 导入任务编排：四阶段并行流水线，UI 主线程全程不被阻塞。
//!
//! 1. 并行 sha256（rayon 默认线程池，锁外 IO）；2. 一次短锁全量查重（冲突分离、
//!    预分配 asset id、keep 建层级发 folders）；3. 并行 复制/移动+缩略图+metadata（锁外）；
//! 4. mpsc 消费者串行注册 Placement/索引并发 Asset 事件（到一个显示一个）。
//!
//! 分段短锁纪律不变：锁内只做查重、Placement 追加、索引插入、folders.json 节流落盘
//! （脏且距上次 ≥2s 才落，结束强制落）。cancel 置位后不再领取新单元，进行中的写完。
//! hash 冲突不自动链接（PRD 10_重复添加提示）：收集 ConflictItem 挂起，待用户决策。
//! 序列帧打包模式（SequenceMode::Pack，4.1）：扫描后先按 sequence::detect_groups 分组，
//! 管线以"单元"推进——单文件或整组序列帧；组 hash 查重、整组建一个 Asset（frames/ 归一）。

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Mutex};
use std::time::{Duration, Instant};

use rayon::prelude::*;

use crate::core::library::{self, Library, ASSTES_DIR};
use crate::core::models::{new_asset_id, AssetMeta};
use crate::services::import as ops;
use crate::services::import::{
  ConflictItem, FailedItem, ImportEvent, ImportOptions, ImportOutcome, PendingFile, SequenceFrameSource, SequenceMode,
  SequenceSource, Source, Structure, ROOT_FOLDER_ID,
};
use crate::services::sequence::{self, SequenceGroup};
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

/// 导入单元：单文件（files 下标）或一组序列帧（groups 下标，打包模式）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Unit {
  Solo(usize),
  Group(usize),
}

/// 单元显示用代表路径（失败记录 / 冲突项 source_path）：单文件为自身，序列帧组为首帧
fn unit_path(files: &[PendingFile], groups: &[SequenceGroup], unit: Unit) -> PathBuf {
  match unit {
    Unit::Solo(i) => files[i].path.clone(),
    Unit::Group(g) => files[groups[g].frames[0].file_idx].path.clone(),
  }
}

/// 单元 Placement 显示名：单文件为文件名，序列帧组为 `标题.扩展名`
fn unit_name(files: &[PendingFile], groups: &[SequenceGroup], unit: Unit) -> String {
  match unit {
    Unit::Solo(i) => ops::file_name(&files[i].path),
    Unit::Group(g) => format!("{}.{}", groups[g].name, groups[g].ext),
  }
}

/// 单元落点目录链（keep 模式）：取代表帧的目录链（同组帧同目录同链）
fn unit_chain<'a>(files: &'a [PendingFile], groups: &[SequenceGroup], unit: Unit) -> &'a [String] {
  match unit {
    Unit::Solo(i) => &files[i].chain,
    Unit::Group(g) => &files[groups[g].frames[0].file_idx].chain,
  }
}

/// 阶段 2 产物：待建档工作项（ID 与落点已预分配）
struct WorkItem {
  unit: Unit,
  hash: String,
  asset_id: String,
  target_folder: String,
}

/// 同批次重复：延后到 twin 资源注册完成（元数据齐备）再组装冲突项
struct DeferredConflict {
  unit: Unit,
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

/// 阶段 3 → 4 的消息：建档成功（工作项 + 元数据 + 格式图标就绪的扩展名）/ 失败（原因）
enum Phase3Msg {
  // AssetMeta 体积大，装箱压低 enum 判别成本（clippy large_enum_variant）
  Built { item: WorkItem, asset: Box<AssetMeta>, icon: Option<String> },
  Broken { unit: Unit, reason: String },
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

  // 阶段 1：锁外扫描展开 → 序列帧分组（打包模式）→ started → 并行 sha256
  let files = ops::collect(&opts.paths, opts.structure, &mut state.outcome.failed);
  let groups = if opts.sequence_mode == SequenceMode::Pack { sequence::detect_groups(&files) } else { Vec::new() };
  let member_count: usize = groups.iter().map(|g| g.frames.len()).sum();
  on_event(ImportEvent::Started { total: (files.len() - member_count + groups.len()) as u32 });
  let ready = hash_all(&files, cancel, &mut state);
  let units = build_units(&mut state, &files, &groups, &ready);

  // 阶段 2：keep 建层级发 folders（层级先行）→ 一次短锁全量查重
  if opts.structure == Structure::Keep {
    if let Err(reason) = prepare_folders(&mut state, lib, &target, &files) {
      return abort(state, lib, &on_event, &files, &groups, &units, reason, "导入层级准备失败");
    }
    on_event(ImportEvent::Folders);
  }
  let mut phase2 =
    match dedup_phase(&mut state, lib, &root, &target, &units, &files, &groups, opts.imported_from.as_deref()) {
      Ok(p) => p,
      Err(reason) => return abort(state, lib, &on_event, &files, &groups, &units, reason, "导入查重失败"),
    };

  // 自动合并（inbox 渠道撞 hash）：改名 + 丢弃重复源，先于管线发 Asset 事件
  apply_merges(&mut state, lib, &root, &files, std::mem::take(&mut phase2.merges), &on_event);

  // 阶段 3+4：并行建档 → 消费者串行注册 + 事件
  pipeline(&mut state, lib, &root, opts.source, opts.imported_from.as_deref(), &files, &groups, phase2, &on_event, cancel);
  finish(state, lib, &on_event)
}

/// 阶段 1 收尾：把就绪哈希装配成导入单元。打包模式：组成员不单独进管线，
/// 各帧哈希按序号升序串联出组 hash；有帧哈希失败的组整组不进（已就绪帧逐个记失败，不吞文件）
fn build_units(
  state: &mut TaskState,
  files: &[PendingFile],
  groups: &[SequenceGroup],
  ready: &[(usize, String)],
) -> Vec<(Unit, String)> {
  let hash_of: HashMap<usize, &str> = ready.iter().map(|(i, h)| (*i, h.as_str())).collect();
  let mut units = Vec::new();
  for (gi, g) in groups.iter().enumerate() {
    let hashes: Vec<Option<&str>> = g.frames.iter().map(|f| hash_of.get(&f.file_idx).copied()).collect();
    if hashes.iter().all(Option::is_some) {
      units.push((Unit::Group(gi), sequence::group_hash(&hashes.into_iter().flatten().collect::<Vec<_>>())));
    } else {
      for f in &g.frames {
        if hash_of.contains_key(&f.file_idx) {
          state.outcome.failed.push(FailedItem {
            path: files[f.file_idx].path.to_string_lossy().into_owned(),
            reason: format!("序列帧《{}》有帧读取失败，整组未导入", g.name),
          });
        }
      }
    }
  }
  let member: HashSet<usize> = groups.iter().flat_map(|g| g.frames.iter().map(|f| f.file_idx)).collect();
  for (idx, hash) in ready {
    if !member.contains(idx) {
      units.push((Unit::Solo(*idx), hash.clone()));
    }
  }
  units
}

/// 阶段整体失败：已就绪单元全部进 failed（原因中文），收尾落盘并发 Finished
#[allow(clippy::too_many_arguments)]
fn abort(
  mut state: TaskState,
  lib: &Mutex<Option<Library>>,
  on_event: &impl Fn(ImportEvent),
  files: &[PendingFile],
  groups: &[SequenceGroup],
  units: &[(Unit, String)],
  reason: String,
  log_prefix: &str,
) -> ImportOutcome {
  log::error!("{log_prefix}：{reason}");
  for (unit, _) in units {
    state
      .outcome
      .failed
      .push(FailedItem { path: unit_path(files, groups, *unit).to_string_lossy().into_owned(), reason: reason.clone() });
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
    // 不用 any：ensure_chain 有建文件夹副作用，必须每条链都跑到，不能短路
    let mut created = false;
    for f in files.iter().filter(|f| !f.chain.is_empty()) {
      created |= ops::ensure_chain(l, target, &f.chain);
    }
    created
  })?;
  state.dirty |= created;
  Ok(())
}

/// 阶段 2（一次短锁）：全量查重。命中库内 → 同渠道（inbox）自动合并（仅单文件单元），
/// 否则立即组装冲突（源侧锁外补）；命中同批次 → 延后冲突；未命中 → 预分配 ID + 解析落点生成工作项
#[allow(clippy::too_many_arguments)]
fn dedup_phase(
  state: &mut TaskState,
  lib: &Mutex<Option<Library>>,
  root: &Path,
  target: &str,
  units: &[(Unit, String)],
  files: &[PendingFile],
  groups: &[SequenceGroup],
  merge_mark: Option<&str>,
) -> Result<Phase2, String> {
  let root_buf = root.to_path_buf();
  let mut phase = Phase2 { work: Vec::new(), deferred: Vec::new(), merges: Vec::new() };
  let mut immediate: Vec<(Unit, ConflictItem)> = Vec::new();
  let mut no_folder: Vec<Unit> = Vec::new();
  with_lib(lib, state, |l| {
    let by_hash: HashMap<&str, &AssetMeta> = l.index.assets.values().map(|a| (a.hash.as_str(), a)).collect();
    let mut batch: HashMap<String, String> = HashMap::new(); // hash → 同批次 twin asset id
    for (unit, hash) in units {
      let chain = unit_chain(files, groups, *unit);
      match classify_one(l, &by_hash, &mut batch, &root_buf, target, chain, *unit, hash, merge_mark) {
        Classified::Conflict(u, c) => immediate.push((u, *c)),
        Classified::Merge(i, id) => phase.merges.push(MergeItem { file_idx: i, asset_id: id }),
        Classified::Deferred(d) => phase.deferred.push(d),
        Classified::Work(w) => phase.work.push(w),
        Classified::NoFolder(u) => no_folder.push(u),
      }
    }
  })?;
  for unit in no_folder {
    state.outcome.failed.push(FailedItem {
      path: unit_path(files, groups, unit).to_string_lossy().into_owned(),
      reason: "目标文件夹不存在，可能已被删除".to_string(),
    });
  }
  for (unit, mut c) in immediate {
    fill_source_side(&mut c, &root_buf, files, groups, unit);
    state.outcome.conflicts.push(c);
  }
  Ok(phase)
}

/// 查重分类结果（阶段 2 锁内）
enum Classified {
  // ConflictItem 体积大，装箱压低 enum 判别成本（clippy large_enum_variant）
  Conflict(Unit, Box<ConflictItem>),
  Merge(usize, String),
  Deferred(DeferredConflict),
  Work(WorkItem),
  NoFolder(Unit),
}

/// 查重分类单项：命中库内 → 单文件单元且同渠道（inbox）自动合并，否则冲突；命中同批次 → 延后冲突；
/// 未命中 → 预分配 ID 建工作项。序列帧组永不自动合并（打包模式不带渠道标记，双保险）
#[allow(clippy::too_many_arguments)]
fn classify_one(
  l: &Library,
  by_hash: &HashMap<&str, &AssetMeta>,
  batch: &mut HashMap<String, String>,
  root: &Path,
  target: &str,
  chain: &[String],
  unit: Unit,
  hash: &str,
  merge_mark: Option<&str>,
) -> Classified {
  let Some(folder_id) = ops::find_folder(l, target, chain) else {
    return Classified::NoFolder(unit);
  };
  if let Some(meta) = by_hash.get(hash) {
    // 自动合并判定：本次导入与已有资产同为 inbox 渠道（正式资产绝不被自动改名）
    if let (Unit::Solo(idx), Some(mark)) = (unit, merge_mark) {
      if meta.imported_from.as_deref() == Some(mark) {
        return Classified::Merge(idx, meta.id.clone());
      }
    }
    Classified::Conflict(unit, Box::new(conflict_of(l, root, meta, folder_id)))
  } else if let Some(twin) = batch.get(hash) {
    Classified::Deferred(DeferredConflict { unit, target_folder: folder_id, twin_id: twin.clone() })
  } else {
    let asset_id = new_asset_id();
    batch.insert(hash.to_string(), asset_id.clone());
    Classified::Work(WorkItem { unit, hash: hash.to_string(), asset_id, target_folder: folder_id })
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
  groups: &[SequenceGroup],
  phase2: Phase2,
  on_event: &(impl Fn(ImportEvent) + Sync),
  cancel: &AtomicBool,
) {
  let (tx, rx) = mpsc::channel::<Phase3Msg>();
  let mut deferred = group_deferred(phase2.deferred);
  std::thread::scope(|s| {
    let consumer = s.spawn(move || {
      // 前插锚点（2026-07-25 用户定：新导入排最前）：任务级，批内按到达顺序保序
      let mut front_anchor: HashMap<String, usize> = HashMap::new();
      for msg in rx {
        match msg {
          Phase3Msg::Built { item, asset, icon } => {
            register_one(state, lib, root, files, groups, item, *asset, icon, &mut deferred, &mut front_anchor, on_event)
          }
          Phase3Msg::Broken { unit, reason } => state.outcome.failed.push(FailedItem {
            path: unit_path(files, groups, unit).to_string_lossy().into_owned(),
            reason,
          }),
        }
      }
      drain_orphan_deferred(state, files, groups, deferred);
    });
    produce(phase2.work, root, source, imported_from, files, groups, &tx, cancel);
    drop(tx); // 关闭通道，消费者收完尾部结果后退出
    if consumer.join().is_err() {
      log::error!("导入注册线程异常（panic）");
    }
  });
}

/// 阶段 3 生产者：并行执行 复制/移动 + 缩略图 + metadata.json；cancel 后不再领新单元
#[allow(clippy::too_many_arguments)]
fn produce(
  work: Vec<WorkItem>,
  root: &Path,
  source: Source,
  imported_from: Option<&str>,
  files: &[PendingFile],
  groups: &[SequenceGroup],
  tx: &mpsc::Sender<Phase3Msg>,
  cancel: &AtomicBool,
) {
  work.into_par_iter().for_each(|item| {
    if cancel.load(Ordering::Relaxed) {
      return;
    }
    let msg = match build_one(root, source, imported_from, files, groups, &item) {
      Ok((asset, icon)) => Phase3Msg::Built { item, asset: Box::new(asset), icon },
      Err(reason) => Phase3Msg::Broken { unit: item.unit, reason },
    };
    if tx.send(msg).is_err() {
      log::error!("导入结果通道异常，结果丢失");
    }
  });
}

/// 阶段 3 单元建档：单文件走 build_asset_with_id；序列帧组整组打包（frames/ 归一重命名）。
/// 返回（元数据, 格式图标就绪的扩展名）：无内容缩略图的单元锁外补 shell 格式图标缓存（4.2）
fn build_one(
  root: &Path,
  source: Source,
  imported_from: Option<&str>,
  files: &[PendingFile],
  groups: &[SequenceGroup],
  item: &WorkItem,
) -> Result<(AssetMeta, Option<String>), String> {
  let asset = match item.unit {
    Unit::Solo(idx) => ops::build_asset_with_id(root, &files[idx], &item.hash, &item.asset_id, source, imported_from)?,
    Unit::Group(gi) => {
      let g = &groups[gi];
      let frames: Vec<(PathBuf, u32)> =
        g.frames.iter().map(|f| (files[f.file_idx].path.clone(), f.number)).collect();
      sequence::build_sequence_asset(
        root,
        &g.name,
        &g.ext,
        g.num_width,
        &frames,
        &item.hash,
        &item.asset_id,
        source,
        imported_from,
      )?
    }
  };
  let icon = ensure_unit_icon(root, &asset);
  Ok((asset, icon))
}

/// 无内容缩略图的单元：补系统格式图标缓存（按格式一份，cache/thumbs，**不进 info**）。
/// 图标取自 info 内已入库文件（剪切语义下源文件已移走）；就绪返回扩展名交消费者登记内存集
fn ensure_unit_icon(root: &Path, asset: &AssetMeta) -> Option<String> {
  if asset.thumbnail.is_some() {
    return None;
  }
  let origin = root.join(ASSTES_DIR).join(format!("{}.info", asset.id)).join(&asset.origin_name);
  thumbnail::ensure_format_icon(root, &origin, &asset.ext).then(|| asset.ext.clone())
}

/// 阶段 4 单项：锁内注册 Placement + 索引 → 发 Asset；twin 完成即组装其延后冲突；
/// 格式图标就绪的扩展名同步登记进内存集（4.2，视图层回退用）
#[allow(clippy::too_many_arguments)]
fn register_one(
  state: &mut TaskState,
  lib: &Mutex<Option<Library>>,
  root: &Path,
  files: &[PendingFile],
  groups: &[SequenceGroup],
  item: WorkItem,
  asset: AssetMeta,
  icon: Option<String>,
  deferred: &mut HashMap<String, Vec<DeferredConflict>>,
  front_anchor: &mut HashMap<String, usize>,
  on_event: &impl Fn(ImportEvent),
) {
  state.dirty = true; // Placement 必改 folders.json，先标脏让节流落盘生效
  let root_buf = root.to_path_buf();
  let name = unit_name(files, groups, item.unit);
  let asset_id = asset.id.clone();
  // 动图（2026-07-26 定案）：注册成功后提交解包队列——逐帧高清图持久化 info/frames/
  let unpack = crate::services::anim::is_anim(&asset);
  // 视频（2026-07-28 定案）：注册成功后提交 scrub 帧预抽——解码成本摊到导入期，悬停毫秒级命中
  let scrub = crate::services::media::VIDEO_EXTS.contains(&asset.ext.as_str());
  let result = with_lib(lib, state, move |l| {
    if let Some(ext) = icon {
      l.format_icons.insert(ext);
    }
    let placement = ops::add_placement(l, asset_id.clone(), item.target_folder.clone(), name, true, front_anchor);
    let view = placement_view(&root_buf, &asset, &placement, &l.format_icons);
    let twins = deferred.remove(&asset_id).unwrap_or_default();
    let conflicts =
      twins.into_iter().map(|d| (d.unit, conflict_of(l, &root_buf, &asset, d.target_folder))).collect::<Vec<_>>();
    l.index.assets.insert(asset_id, asset);
    (view, conflicts)
  });
  if unpack && result.is_ok() {
    crate::services::unpack_queue::global()
      .submit(crate::services::unpack_queue::UnpackTask { root: root.to_path_buf(), asset_id: item.asset_id.clone() });
  }
  if scrub && result.is_ok() {
    crate::services::unpack_queue::global().submit_scrub(root.to_path_buf(), item.asset_id.clone());
  }
  collect_registered(state, root, files, groups, item.unit, result, on_event);
}

/// 注册结果入帐：成功发 Asset 事件并补源侧冲突数据；失败进 failed
fn collect_registered(
  state: &mut TaskState,
  root: &Path,
  files: &[PendingFile],
  groups: &[SequenceGroup],
  unit: Unit,
  result: Result<(PlacementView, Vec<(Unit, ConflictItem)>), String>,
  on_event: &impl Fn(ImportEvent),
) {
  match result {
    Ok((view, conflicts)) => {
      state.outcome.imported += 1;
      on_event(ImportEvent::Asset { placement: view });
      for (u, mut c) in conflicts {
        fill_source_side(&mut c, root, files, groups, u);
        state.outcome.conflicts.push(c);
      }
    }
    Err(reason) => state
      .outcome
      .failed
      .push(FailedItem { path: unit_path(files, groups, unit).to_string_lossy().into_owned(), reason }),
  }
}

/// twin 建档失败的延后冲突：内容不在库内，提示用户重试导入即可正常建档
fn drain_orphan_deferred(
  state: &mut TaskState,
  files: &[PendingFile],
  groups: &[SequenceGroup],
  deferred: HashMap<String, Vec<DeferredConflict>>,
) {
  for ds in deferred.into_values() {
    for d in ds {
      state.outcome.failed.push(FailedItem {
        path: unit_path(files, groups, d.unit).to_string_lossy().into_owned(),
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
      rename_origin_placement(l, &m.asset_id, &meta.origin_name).map(|p| placement_view(&root_buf, &meta, &p, &l.format_icons))
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
    source_thumb_path: None,
    target_folder_name: folder_display_name(l, &folder_id),
    target_folder: folder_id,
    existing_asset_id: meta.id.clone(),
    existing_name: chosen.map(|p| p.name.clone()).unwrap_or_else(|| meta.name.clone()),
    existing_w: meta.width,
    existing_h: meta.height,
    existing_size: meta.size,
    existing_ext: meta.ext.clone(),
    existing_thumb_path: thumb_path(root, meta, &l.format_icons),
    existing_folder_name: chosen.map(|p| folder_display_name(l, &p.folder)).unwrap_or_default(),
    sequence: None, // 源侧信息由 fill_source_side 补（序列帧组覆盖为组信息）
  }
}

/// 文件夹显示名："root" → "根目录"；未知 ID → "未知文件夹"（防御，正常不出现）
fn folder_display_name(l: &Library, folder_id: &str) -> String {
  if folder_id == ROOT_FOLDER_ID {
    return "根目录".to_string();
  }
  l.index.folders.get(folder_id).map(|f| f.name.clone()).unwrap_or_else(|| "未知文件夹".to_string())
}

/// 前端 webview 可直接渲染的图片格式（与 DuplicateDialog 的 IMAGE_EXTS 对齐）：
/// 这些格式弹窗直读源文件，其余（PSD/PSB/SVG 等）走后端预览缓存
const WEBVIEW_DIRECT_EXTS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif", "bmp"];

/// 预览缓存键：路径 + 大小的散列（同路径同大小命中同一份缓存，cache/tmp 幂等）
fn preview_key(path: &Path, size: u64) -> String {
  use std::hash::{Hash, Hasher};
  let mut h = std::collections::hash_map::DefaultHasher::new();
  path.hash(&mut h);
  size.hash(&mut h);
  format!("{:x}", h.finish())
}

/// 锁外补冲突项的源文件侧数据：文件名 / 字节数 / 图片宽高（probe 失败即非图片，None）；
/// 序列帧组：显示名 = `标题.扩展名`，source_path 取首帧（弹窗预览首帧），并携带组信息
/// （前端原样回传给 resolve_conflicts，"保留两个"按此重新打包）；
/// webview 直渲不了的格式生成预览缓存（cache/tmp）供弹窗"新的文件"对照显示（4.2）
fn fill_source_side(item: &mut ConflictItem, root: &Path, files: &[PendingFile], groups: &[SequenceGroup], unit: Unit) {
  match unit {
    Unit::Solo(idx) => {
      let file = &files[idx];
      item.source_path = file.path.clone();
      item.source_name = ops::file_name(&file.path);
      item.source_size = std::fs::metadata(&file.path).map(|m| m.len()).unwrap_or(0);
      let (w, h) = thumbnail::probe(&file.path).map(|(w, h)| (Some(w), Some(h))).unwrap_or((None, None));
      item.source_w = w;
      item.source_h = h;
      let ext = file.path.extension().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
      if !WEBVIEW_DIRECT_EXTS.contains(&ext.as_str()) {
        item.source_thumb_path = thumbnail::generate_preview(root, &file.path, &preview_key(&file.path, item.source_size));
      }
    }
    Unit::Group(gi) => {
      let g = &groups[gi];
      let frames: Vec<SequenceFrameSource> =
        g.frames.iter().map(|f| SequenceFrameSource { path: files[f.file_idx].path.clone(), number: f.number }).collect();
      item.source_path = frames[0].path.clone();
      item.source_name = format!("{}.{}", g.name, g.ext);
      item.source_size = frames.iter().map(|f| std::fs::metadata(&f.path).map(|m| m.len()).unwrap_or(0)).sum();
      let (w, h) = thumbnail::probe(&frames[0].path).map(|(w, h)| (Some(w), Some(h))).unwrap_or((None, None));
      item.source_w = w;
      item.source_h = h;
      item.sequence =
        Some(SequenceSource { name: g.name.clone(), ext: g.ext.clone(), width: g.num_width as u32, frames });
    }
  }
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
