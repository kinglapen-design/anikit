//! 收件箱服务（PRD 05-01，阶段 3.2 落地；2026-07-25 语义重定 4.4）：空间级 inbox/ 目录的完整
//! 生命周期（2026-08-12 用户定：inbox 空间级共用，盘符挂载恒定不随切库变化）——
//! 空间打开 / 自动导入开启时 start（盘符挂载 + 文件监听 + 存量清点），开关关闭 / 退出时 stop。
//! Inbox = 工作中素材/模板快速入口，不是工程收纳器：**自动导入只放行白名单可视素材扩展名**
//! （AUTO_IMPORT_EXTS，不反向枚举工程格式——枚举不完）；白名单以外留在磁盘不自动导入，
//! 由"工程文件"区块做磁盘实况展示（snapshot，不建档不写库），用户右键手动导入（移走入库）。
//! 自动导入目标 = 当前打开的库（dispatch_loop 取库逻辑）：就绪批次串行复用 3.1 导入管线：
//! Keep（丢入的文件夹保留层级）+ Move（过境语义：
//! 导入后即移走、目录常态为空）；hash 冲突走"导入即建档"既有规则——挂起收集，
//! 经 app.emit("inbox-import") 推给前端复用 11_导入进度条 / 10_重复添加提示。
//! 磁盘实况热刷新：watcher 任何路径事件 → app.emit("inbox-fs-changed")（前端去抖重扫）。
//! 实况快照分两层防卡（2026-07-25 用户批准）：快层 snapshot_fast 供 list_inbox_snapshot 同步返回
//! （文件大小取元数据、单文件占用探测；目录大小 / 占用先给 0 / false 占位）；慢层 snapshot_details
//! （递归大小、占用、shell 格式图标）由后台 detail worker 延迟 200ms 合并连发后计算，
//! 经 app.emit("inbox-disk-details") 推前端整体替换。**自动导入取消 / 失败不再自动重试**
//! （2026-08-13 用户定案，二轮）：跑过导入管线的 failed / 冲突项记入**搁置清单**
//! （空间根 `inbox_blocked.json`，本模块单一写点；按 顶层条目名 + 大小 + 修改时间 精确标识
//! "那个文件对象"——同名新文件 mtime 不同不拦，改名经 blocked_rename 平移记录保持拦截），
//! dispatch_loop 放行前过滤——重启存量清点 / 改名 fs 事件都不得再触发自动导入，
//! 由用户在"工程文件"区块右键手动入库。
//! 全部失败（目录创建失败、盘符挂载失败、监听启动失败）记日志降级，不影响空间打开。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::adapters::{inbox_drive, inbox_watch};
use crate::commands::library::AppState;
use crate::core::json_io;
use crate::services::import::{ImportOptions, ImportOutcome, SequenceMode, Source, Structure};
use crate::services::import_task::run_import;
use crate::services::thumbnail;

/// 空间级收件箱目录名（单点在 core::space，创建 / 自愈与盘符挂载同锚点）
pub use crate::core::space::INBOX_DIR;
/// 前端监听事件名（负载 = ImportEvent，与导入 Channel 同一形状）
pub const EVENT_NAME: &str = "inbox-import";
/// 磁盘实况热刷新事件名（负载 = ()，watcher 任何路径事件触发，前端去抖重扫）
pub const FS_CHANGED_EVENT: &str = "inbox-fs-changed";
/// 磁盘实况慢层事件名（负载 = Vec<InboxEntry>）：detail worker 算好递归大小 / 占用 /
/// 格式图标后整体推送，前端收到后全量替换快层快照
pub const DETAILS_EVENT: &str = "inbox-disk-details";

/// detail worker 延迟窗口：合并 fs 连发事件为一拍计算
const DETAIL_DELAY: Duration = Duration::from_millis(200);
/// detail worker 防重入标记：同一时刻至多一个在跑（连发事件由在跑的那拍合并覆盖）
static DETAIL_RUNNING: AtomicBool = AtomicBool::new(false);

/// 自动导入白名单（小写比较）：可视素材扩展名（4.2 可出缩略图口径 + 2026-07-25 用户补视频
/// mp4/mov/avi/mpeg4/m4v + 2026-07-26 音频 mp3/wav/wma（5.1 波形/元数据就位，消化 4.4 留待））；
/// 白名单以外一律不自动导入——不反向枚举工程格式（枚举不完，2026-07-25 用户定案）
pub const AUTO_IMPORT_EXTS: &[&str] = &[
  "png", "jpg", "jpeg", "bmp", "webp", "gif", "psd", "psb", "svg", "ai", "mp4", "mov", "avi", "mpeg4", "m4v",
  "mp3", "wav", "wma",
];

/// 单文件可否自动导入：扩展名在白名单（无扩展名不可）
pub fn file_auto_importable(path: &Path) -> bool {
  path
    .extension()
    .and_then(|e| e.to_str())
    .is_some_and(|e| AUTO_IMPORT_EXTS.contains(&e.to_lowercase().as_str()))
}

/// 条目可否自动导入：文件看扩展名；目录 = 递归文件非空且全部白名单
/// （混入任何非白名单文件整目录不自动导入，留在磁盘实况区等用户手动处理）
pub fn entry_auto_importable(path: &Path) -> bool {
  if path.is_file() {
    return file_auto_importable(path);
  }
  let files = files_of(path);
  !files.is_empty() && files.iter().all(|f| file_auto_importable(f))
}

/// 条目内全部文件（文件夹递归）；目录读取失败的条目按空计
fn files_of(path: &Path) -> Vec<PathBuf> {
  inbox_watch::files_of(path)
}

// ===== 自动导入搁置清单（2026-08-13 用户定案：取消 / 失败不再自动重试）=====
// 跑过导入管线的 failed / 冲突（含用户在重复提示里取消）源条目记录在此，dispatch_loop
// 放行前过滤——重启存量清点、改名 / 内容 fs 事件都不得再触发自动导入，只能手动入库。
// 标识 = 顶层条目名 + 大小 + 修改时间（精确到"那个文件对象"：同名新文件 mtime 不同不拦）；
// 改名经 blocked_rename 平移记录保持拦截，删除经 blocked_remove 清记录。
// 持久化在空间根 inbox_blocked.json（本模块单一写点）；记录随条目移走 / 删除变陈旧，
// 因 mtime + 大小不匹配新文件而无害，无需主动清理。

/// 搁置清单文件名（空间根，非库级——inbox 空间级共用同锚点）
const BLOCKED_FILE: &str = "inbox_blocked.json";

/// 搁置清单文件（json_io 统一出入口要求 version 门禁字段）
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BlockedFile {
  version: u32,
  blocked: Vec<BlockedEntry>,
}

/// 搁置记录：顶层条目名 + 大小 + 修改时间（Unix 毫秒）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BlockedEntry {
  name: String,
  size: u64,
  mtime_ms: u64,
}

fn blocked_path(root: &Path) -> PathBuf {
  root.join(BLOCKED_FILE)
}

/// 读搁置清单（缺文件 / 解析失败按空清单降级——不拦任何条目，宁可多导入一次不可丢用户文件）
fn blocked_load(root: &Path) -> Vec<BlockedEntry> {
  json_io::read_json::<BlockedFile>(&blocked_path(root)).map(|f| f.blocked).unwrap_or_default()
}

/// 写搁置清单（失败仅记日志——下次启动清单缺失回到"不拦"，不会卡死导入）
fn blocked_save(root: &Path, list: &[BlockedEntry]) {
  let file = BlockedFile { version: crate::core::models::DATA_VERSION, blocked: list.to_vec() };
  if let Err(e) = json_io::write_json(&blocked_path(root), &file) {
    log::error!("收件箱搁置清单写入失败：{e}");
  }
}

/// 取条目的搁置标识（文件 = 大小 + mtime；目录 = 自身 mtime、大小记 0——
/// 增删直接子项会改变目录 mtime，内容变了视作新条目放行，与"取消的是那一次"语义一致）
fn entry_record(path: &Path) -> Option<BlockedEntry> {
  let meta = std::fs::metadata(path).ok()?;
  Some(BlockedEntry {
    name: path.file_name()?.to_string_lossy().into_owned(),
    size: if meta.is_dir() { 0 } else { meta.len() },
    mtime_ms: modified_millis(&meta),
  })
}

/// 条目是否在搁置清单内（名 + 大小 + mtime 三项全中才拦）
fn is_blocked(blocked: &[BlockedEntry], path: &Path) -> bool {
  let Some(rec) = entry_record(path) else { return false };
  blocked.iter().any(|b| b == &rec)
}

/// 导入结果落搁置清单：conflicts（用户取消 = 全部跳过；即使后续 resolve 移走源，
/// 记录陈旧无害）与 failed 的源路径归并到顶层条目，同名替换后整体写回
fn block_outcome(root: &Path, outcome: &ImportOutcome) {
  let inbox_root = root.join(INBOX_DIR);
  let mut tops: Vec<PathBuf> = Vec::new();
  for c in &outcome.conflicts {
    if let Some(t) = top_entry(&inbox_root, &c.source_path) {
      tops.push(t);
    }
  }
  for f in &outcome.failed {
    if let Some(t) = top_entry(&inbox_root, Path::new(&f.path)) {
      tops.push(t);
    }
  }
  if tops.is_empty() {
    return;
  }
  let mut list = blocked_load(root);
  let mut added = 0u32;
  for t in tops {
    let Some(rec) = entry_record(&t) else { continue }; // 已移走（resolve copy/link）跳过
    list.retain(|b| b.name != rec.name); // 同名旧记录替换（内容可能已覆写）
    list.push(rec);
    added += 1;
  }
  if added > 0 {
    log::info!("收件箱自动导入搁置 {added} 项（取消 / 失败不再自动重试，可手动入库）");
    blocked_save(root, &list);
  }
}

/// 重命名平移搁置记录（条目名变、大小与 mtime 不变，保持拦截——改名不等于重新丢入）；
/// 命令层 rename_inbox_entry 成功后调用
pub fn blocked_rename(root: &Path, old_name: &str, new_name: &str) {
  let mut list = blocked_load(root);
  let mut hit = false;
  for b in &mut list {
    if b.name == old_name {
      b.name = new_name.to_string();
      hit = true;
    }
  }
  if hit {
    blocked_save(root, &list);
  }
}

/// 删除清搁置记录（命令层 delete_inbox_entry 成功后调用；手动导入不移记录——
/// 条目移走后记录自然陈旧无害）
pub fn blocked_remove(root: &Path, name: &str) {
  let mut list = blocked_load(root);
  let before = list.len();
  list.retain(|b| b.name != name);
  if list.len() != before {
    blocked_save(root, &list);
  }
}

/// 顶层条目：p 相对 inbox 目录的第一级（文件本身 / 被丢入的文件夹）；目录外路径 None
fn top_entry(inbox_root: &Path, p: &Path) -> Option<PathBuf> {
  let first = p.strip_prefix(inbox_root).ok()?.components().next()?;
  Some(inbox_root.join(first.as_os_str()))
}

/// 收件箱磁盘实况条目（"工程文件"区块展示用；事实来源 = 磁盘，不建档不写库）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InboxEntry {
  /// 顶层条目名（含扩展名）
  pub name: String,
  pub is_dir: bool,
  /// 条目总大小（字节，文件夹递归；快层快照中目录先给 0，details 事件补齐）
  pub size: u64,
  /// 修改时间（Unix 毫秒，前端显示用）
  pub modified_at: u64,
  /// 是否可自动导入（白名单判定；false 的留在磁盘等手动导入）
  pub auto_importable: bool,
  /// 是否被占用（与自动入库同口径的可移走探测；占用条目禁止手动导入；
  /// 快层快照中目录先给 false 过渡态，以 details 事件合并后为准）
  pub occupied: bool,
  /// 系统格式图标缓存绝对路径（库级 cache/thumbs/{ext}.webp，与导入管线共用同一份缓存；
  /// 目录恒 None，快层只命中已缓存、details 层现场生成；无当前库或 None 时前端回退扩展名角标）
  pub icon_path: Option<String>,
}

/// 快层快照（list_inbox_snapshot 同步返回，必须快）：
/// 文件大小直接取元数据、占用做单文件探测（都便宜）；目录的递归大小 / 占用探测贵，
/// 先给 0 / false 占位，由后台 details 补齐（前端收到 inbox-disk-details 后整体替换）。
/// 目录 auto_importable 仍按"递归扩展名全白名单"判定（只读目录项，不打开文件内容、不递归算大小）。
/// iconPath 只命中已缓存的格式图标（只判存在，绝不触发 shell——shell 调用只允许在 detail worker）；
/// 缓存在库级 cache/thumbs，cache_root = 当前库 root（无当前库时图标恒 None，前端回退扩展名角标）。
pub fn snapshot_fast(root: &Path, cache_root: Option<&Path>) -> Vec<InboxEntry> {
  let dir = root.join(INBOX_DIR);
  let mut out = Vec::new();
  let Ok(entries) = std::fs::read_dir(&dir) else { return out };
  for entry in entries.filter_map(|e| e.ok()) {
    let path = entry.path();
    let Ok(meta) = entry.metadata() else { continue };
    let is_dir = meta.is_dir();
    out.push(InboxEntry {
      name: entry.file_name().to_string_lossy().into_owned(),
      is_dir,
      size: if is_dir { 0 } else { meta.len() },
      modified_at: modified_millis(&meta),
      auto_importable: entry_auto_importable(&path),
      occupied: !is_dir && !inbox_watch::entry_movable(&path),
      icon_path: if is_dir { None } else { cached_icon_path(cache_root, &path) },
    });
  }
  sort_entries(&mut out);
  out
}

/// 慢层完整快照（detail worker 后台算）：递归大小 + 全部文件占用探测 + 格式图标
/// 现场生成（复用 4.2 按格式缓存，shell 取系统图标只写库级 cache/thumbs 不写 info；
/// cache_root = 当前库 root，无当前库时不生成、图标恒 None）。
pub fn snapshot_details(root: &Path, cache_root: Option<&Path>) -> Vec<InboxEntry> {
  let dir = root.join(INBOX_DIR);
  let mut out = Vec::new();
  let Ok(entries) = std::fs::read_dir(&dir) else { return out };
  for entry in entries.filter_map(|e| e.ok()) {
    let path = entry.path();
    let Ok(meta) = entry.metadata() else { continue };
    let is_dir = path.is_dir();
    out.push(InboxEntry {
      name: entry.file_name().to_string_lossy().into_owned(),
      is_dir,
      size: inbox_watch::entry_size(&path).unwrap_or(0),
      modified_at: modified_millis(&meta),
      auto_importable: entry_auto_importable(&path),
      occupied: !inbox_watch::entry_movable(&path),
      icon_path: if is_dir { None } else { ensure_icon_path(cache_root, &path) },
    });
  }
  sort_entries(&mut out);
  out
}

/// 修改时间转 Unix 毫秒（读取失败给 0）
fn modified_millis(meta: &std::fs::Metadata) -> u64 {
  meta
    .modified()
    .ok()
    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
    .map(|d| d.as_millis() as u64)
    .unwrap_or(0)
}

/// 按名称小写排序（快层 / 慢层同口径，前端整体替换时顺序稳定）
fn sort_entries(out: &mut [InboxEntry]) {
  out.sort_by_key(|e| e.name.to_lowercase());
}

/// 文件扩展名（小写）
fn ext_lower(path: &Path) -> String {
  path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase()
}

/// 快层用：格式图标已缓存才给路径（只判存在，不触发 shell）；无当前库（cache_root None）恒 None
fn cached_icon_path(cache_root: Option<&Path>, path: &Path) -> Option<String> {
  let dest = thumbnail::format_icon_path(cache_root?, &ext_lower(path));
  dest.is_file().then(|| dest.to_string_lossy().into_owned())
}

/// 慢层用：生成 / 命中格式图标缓存（shell 调用只允许出现在 detail worker）；
/// 缓存写在库级 cache/thumbs 下（与导入管线共用同一份缓存，cache_root = 当前库 root；
/// 无当前库不生成恒 None），不在 inbox 监听范围内，天然不会递归触发 fs-changed
fn ensure_icon_path(cache_root: Option<&Path>, path: &Path) -> Option<String> {
  let ext = ext_lower(path);
  let root = cache_root?;
  thumbnail::ensure_format_icon(root, path, &ext)
    .then(|| thumbnail::format_icon_path(root, &ext).to_string_lossy().into_owned())
}

/// 后台 details 调度（慢层）：fs 变化 / 收件箱启动时调用。已在跑则直接返回——
/// 在跑的那拍延迟窗口后才读磁盘，连发变化自然被合并覆盖；延迟 200ms 后算完整快照
/// 经 app.emit 推前端整体替换。不触发 fs-changed（见 ensure_icon_path 注）。
fn schedule_details(app: &AppHandle, root: &Path) {
  if DETAIL_RUNNING.swap(true, Ordering::SeqCst) {
    return;
  }
  let app = app.clone();
  let root = root.to_path_buf();
  std::thread::spawn(move || {
    std::thread::sleep(DETAIL_DELAY);
    // 图标缓存锚点 = 当前库 root（计算时短锁取；无当前库则本拍图标恒 None，不生成缓存）
    let cache_root =
      app.state::<AppState>().library.lock().ok().and_then(|g| g.as_ref().map(|l| l.root.clone()));
    let details = snapshot_details(&root, cache_root.as_deref());
    // 先放行标记再推事件：计算期间的变化靠下一次 fs-changed 再起一拍
    DETAIL_RUNNING.store(false, Ordering::SeqCst);
    if let Err(e) = app.emit(DETAILS_EVENT, details) {
      log::debug!("收件箱 details 事件推送失败（前端可能已关闭）：{e}");
    }
  });
}

/// 收件箱运行实例（存 AppState.inbox，随空间打开 / 自动导入开关 / 退出启停）
pub struct InboxHandle {
  stop: Arc<AtomicBool>,
  watch: Option<inbox_watch::WatchHandle>,
  dispatcher: Option<JoinHandle<()>>,
  /// 当前挂载的盘符（未挂载 None，托盘开关读写）
  drive: Option<char>,
}

/// 启动收件箱（空间打开 / 自动导入开启后调用，root = 空间根）：先停旧实例；任一部件失败记日志降级继续
pub fn start(app: &AppHandle, root: &Path) {
  stop(app);
  let dir = root.join(INBOX_DIR);
  if let Err(e) = std::fs::create_dir_all(&dir) {
    log::error!("收件箱目录创建失败：{e}");
    return;
  }
  let drive = match inbox_drive::mount(&dir) {
    Ok(letter) => Some(letter),
    Err(e) => {
      log::error!("收件箱盘符挂载失败（监听不受影响）：{e}");
      None
    }
  };

  // 派发线程：就绪批次串行跑导入管线（与 UI 导入共用库锁，分段短锁不卡界面）
  let (tx, rx) = mpsc::channel::<Vec<PathBuf>>();
  let stop = Arc::new(AtomicBool::new(false));
  let dispatcher = {
    let app = app.clone();
    let root = root.to_path_buf(); // 搁置清单锚点（空间根）
    let stop = stop.clone();
    std::thread::spawn(move || dispatch_loop(app, root, rx, stop))
  };
  // 磁盘实况热刷新：任何路径事件原样转发给前端（去抖在 listen 侧做），并起慢层 details 补算
  let app_fs = app.clone();
  let root_details = root.to_path_buf();
  let watch = match inbox_watch::spawn(
    dir.clone(),
    move |paths| {
      if tx.send(paths).is_err() {
        log::debug!("收件箱派发通道已关闭，就绪批次丢弃");
      }
    },
    move |_paths| {
      if let Err(e) = app_fs.emit(FS_CHANGED_EVENT, ()) {
        log::debug!("收件箱实况事件推送失败（前端可能已关闭）：{e}");
      }
      schedule_details(&app_fs, &root_details);
    },
  ) {
    Ok(w) => Some(w),
    Err(e) => {
      log::error!("收件箱监听启动失败：{e}");
      None
    }
  };

  let state = app.state::<AppState>();
  match state.inbox.lock() {
    Ok(mut slot) => *slot = Some(InboxHandle { stop, watch, dispatcher: Some(dispatcher), drive }),
    Err(_) => log::error!("收件箱状态登记失败（锁冲突）"),
  }
  sync_tray(app);
  schedule_details(app, root); // 启动即补一拍 details（存量条目的递归大小 / 占用 / 图标）
  log::info!("收件箱已启动：{}（盘符 {:?}）", dir.display(), drive);
}

/// 停止收件箱（自动导入关闭 / 应用退出）：停监听 → 等派发线程排空 → 卸载盘符（幂等）
pub fn stop(app: &AppHandle) {
  let handle = app.state::<AppState>().inbox.lock().ok().and_then(|mut slot| slot.take());
  if let Some(mut h) = handle {
    h.stop.store(true, Ordering::Relaxed);
    if let Some(mut w) = h.watch.take() {
      w.stop(); // 监听线程退出后回调里的 tx 随之释放，派发线程 recv 断开自然退出
    }
    if let Some(j) = h.dispatcher.take() {
      let _ = j.join();
    }
    if let Some(letter) = h.drive {
      if let Err(e) = inbox_drive::unmount(letter) {
        log::error!("收件箱盘符卸载失败：{e}");
      }
    }
  }
  sync_tray(app);
}

/// 托盘开关：已挂载 → 卸载；未挂载且有打开的空间 → 重挂（用户手动卸过的场景）
pub fn toggle_drive(app: &AppHandle) {
  let state = app.state::<AppState>();
  // 先取空间 root（短锁，收件箱空间级共用），再动 inbox 锁，避免嵌套
  let root = state.space.lock().ok().and_then(|g| g.as_ref().map(|s| s.root.clone()));
  if let Ok(mut slot) = state.inbox.lock() {
    if let Some(h) = slot.as_mut() {
      match h.drive {
        Some(letter) => {
          if let Err(e) = inbox_drive::unmount(letter) {
            log::error!("收件箱盘符卸载失败：{e}");
          } else {
            h.drive = None;
          }
        }
        None => match root {
          Some(root) => match inbox_drive::mount(&root.join(INBOX_DIR)) {
            Ok(letter) => h.drive = Some(letter),
            Err(e) => log::error!("收件箱盘符挂载失败：{e}"),
          },
          None => log::debug!("未打开储存空间，收件箱盘符开关忽略"),
        },
      }
    }
  }
  sync_tray(app);
}

/// 派发循环：逐批跑导入；stop 置位或通道关闭即退出。root = 空间根（搁置清单锚点）；
/// 导入目标 = 当前打开的库（库 root / assets 池逐批短锁取）。
/// 白名单过滤（2026-07-25 语义重定）：就绪条目逐个判定——文件仅白名单扩展名、
/// 目录仅递归全部文件白名单才进导入；其余跳过留在磁盘，由"工程文件"实况区块展示等手动导入
fn dispatch_loop(app: AppHandle, root: PathBuf, rx: mpsc::Receiver<Vec<PathBuf>>, stop: Arc<AtomicBool>) {
  while !stop.load(Ordering::Relaxed) {
    let Ok(paths) = rx.recv() else { break };
    if stop.load(Ordering::Relaxed) {
      break;
    }
    let paths: Vec<PathBuf> = paths.into_iter().filter(|p| entry_auto_importable(p)).collect();
    if paths.is_empty() {
      continue; // 整批都是白名单以外（工程文件等），留在磁盘不自动导入
    }
    // 搁置过滤（2026-08-13 用户定案）：取消冲突 / 失败过的条目不再自动重试——
    // 重启存量清点、改名 / 内容 fs 事件都拦在这里，只能"工程文件"区块右键手动入库
    let blocked = blocked_load(&root);
    let paths: Vec<PathBuf> = if blocked.is_empty() {
      paths
    } else {
      let (skip, keep): (Vec<PathBuf>, Vec<PathBuf>) = paths.into_iter().partition(|p| is_blocked(&blocked, p));
      if !skip.is_empty() {
        log::info!("收件箱自动导入跳过已搁置 {} 项", skip.len());
      }
      keep
    };
    if paths.is_empty() {
      continue;
    }
    let state = app.state::<AppState>();
    let opts = ImportOptions {
      structure: Structure::Keep,
      source: Source::Move,
      target_folder: None,
      paths,
      // 收件箱渠道标记（"已导入"区块口径与摘标语义用）；撞 hash 与常规导入同口径走重复提示
      imported_from: Some("inbox".to_string()),
      // 收件箱无弹窗交互，序列帧维持逐文件导入（4.1 留待，见 05_项目进度）
      sequence_mode: SequenceMode::Scatter,
    };
    let app_ev = app.clone();
    // 库 root（cache 等库级路径）与 assets 池目录随当前库取（短锁；库已关闭则本批跳过，保持与 with_lib 报错同义的"不导入"）
    let lib_paths = state.library.lock().ok().and_then(|g| g.as_ref().map(|l| (l.root.clone(), l.assets_dir.clone())));
    let Some((lib_root, assets_dir)) = lib_paths else {
      log::warn!("收件箱自动导入跳过：当前无打开的素材库");
      continue;
    };
    // 收件箱导入无 UI 取消入口（批次通常很小），cancel 恒 false
    let outcome = run_import(
      &state.library,
      lib_root,
      assets_dir,
      opts,
      move |ev| {
        if let Err(e) = app_ev.emit(EVENT_NAME, &ev) {
          log::debug!("收件箱事件推送失败（前端可能已关闭）：{e}");
        }
      },
      &AtomicBool::new(false),
    );
    // 取消冲突 / 失败条目落搁置清单：不再自动重试（2026-08-13 用户定案），手动入库除外
    block_outcome(&root, &outcome);
  }
}

/// 同步托盘菜单项：勾选态 + 盘符标注（收件箱未启动 / 未挂载显示"未挂载"）
fn sync_tray(app: &AppHandle) {
  let drive = app.state::<AppState>().inbox.lock().ok().and_then(|slot| slot.as_ref().and_then(|h| h.drive));
  let Some(items) = app.try_state::<crate::TrayItems>() else { return };
  let (label, checked) = match drive {
    Some(letter) => (format!("收件箱盘符（{letter}:）"), true),
    None => ("收件箱盘符（未挂载）".to_string(), false),
  };
  let _ = items.inbox.set_text(label);
  let _ = items.inbox.set_checked(checked);
}


#[cfg(test)]
mod tests {
  use super::*;
  use crate::services::import::{ConflictItem, FailedItem};

  fn harness() -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join(INBOX_DIR);
    std::fs::create_dir_all(&dir).unwrap();
    (tmp, dir)
  }

  fn conflict_at(path: &Path) -> ConflictItem {
    ConflictItem {
      source_path: path.to_path_buf(),
      source_name: path.file_name().unwrap().to_string_lossy().into_owned(),
      source_size: 0,
      source_w: None,
      source_h: None,
      source_thumb_path: None,
      target_folder: "root".to_string(),
      target_folder_name: "根文件夹".to_string(),
      existing_asset_id: "AN-X".to_string(),
      existing_name: "已有".to_string(),
      existing_w: None,
      existing_h: None,
      existing_size: 0,
      existing_ext: "png".to_string(),
      existing_thumb_path: None,
      existing_folder_name: "根文件夹".to_string(),
      sequence: None,
      imported_from: None,
    }
  }

  /// 冲突 / 失败源归并顶层条目落搁置清单；已移走的源（resolve copy/link 后）跳过；
  /// 清单外条目不拦
  #[test]
  fn block_outcome_records_conflict_and_failed_top_entries() {
    let (tmp, dir) = harness();
    let root = tmp.path();
    let f1 = dir.join("图.png");
    let f2 = dir.join("档.psd");
    let other = dir.join("别的.png");
    std::fs::write(&f1, "123").unwrap();
    std::fs::write(&f2, "12").unwrap();
    std::fs::write(&other, "x").unwrap();
    let outcome = ImportOutcome {
      imported: 0,
      conflicts: vec![conflict_at(&f1), conflict_at(&dir.join("已移走.png"))],
      failed: vec![FailedItem { path: f2.to_string_lossy().into_owned(), reason: "占用".to_string() }],
    };
    block_outcome(root, &outcome);
    let list = blocked_load(root);
    assert_eq!(list.len(), 2, "已移走的源不落清单：{list:?}");
    assert!(is_blocked(&list, &f1), "冲突源应搁置");
    assert!(is_blocked(&list, &f2), "失败源应搁置");
    assert!(!is_blocked(&list, &other), "清单外条目不拦");
  }

  /// 文件夹批的冲突源归并到顶层目录（目录记 size 0 + 自身 mtime）
  #[test]
  fn block_outcome_maps_nested_source_to_top_dir() {
    let (tmp, dir) = harness();
    let root = tmp.path();
    let sub = dir.join("夹");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(sub.join("a.png"), "1").unwrap();
    let outcome = ImportOutcome { imported: 0, conflicts: vec![conflict_at(&sub.join("a.png"))], failed: vec![] };
    block_outcome(root, &outcome);
    let list = blocked_load(root);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, "夹");
    assert!(is_blocked(&list, &sub), "顶层目录应整体搁置");
  }

  /// 拦截口径 = 名 + 大小 + mtime 三项全中：同名新文件（mtime / 大小不同）不拦
  #[test]
  fn is_blocked_requires_same_size_and_mtime() {
    let (_tmp, dir) = harness();
    let f = dir.join("图.png");
    std::fs::write(&f, "123").unwrap();
    let rec = entry_record(&f).unwrap();
    assert!(is_blocked(std::slice::from_ref(&rec), &f), "原记录应命中");
    let diff_mtime = BlockedEntry { mtime_ms: rec.mtime_ms + 1, ..rec.clone() };
    assert!(!is_blocked(&[diff_mtime], &f), "mtime 不同不拦（同名新文件）");
    let diff_size = BlockedEntry { size: rec.size + 1, ..rec };
    assert!(!is_blocked(&[diff_size], &f), "大小不同不拦");
  }

  /// 重命名平移记录（新名仍拦、旧名记录不再存在）；删除清记录
  #[test]
  fn blocked_rename_carries_and_remove_clears() {
    let (tmp, dir) = harness();
    let root = tmp.path();
    let f = dir.join("图.png");
    std::fs::write(&f, "123").unwrap();
    let outcome = ImportOutcome {
      imported: 0,
      conflicts: vec![],
      failed: vec![FailedItem { path: f.to_string_lossy().into_owned(), reason: "占用".to_string() }],
    };
    block_outcome(root, &outcome);

    let new = dir.join("新名.png");
    std::fs::rename(&f, &new).unwrap();
    blocked_rename(root, "图.png", "新名.png");
    let list = blocked_load(root);
    assert!(is_blocked(&list, &new), "改名后新名仍应搁置");
    assert!(list.iter().all(|b| b.name != "图.png"), "旧名记录应平移不残留");

    blocked_remove(root, "新名.png");
    assert!(blocked_load(root).is_empty(), "删除后记录应清除");
    assert!(root.join(BLOCKED_FILE).is_file(), "清单应持久化在空间根");
  }

  /// 清单缺文件 / 内容损坏按空清单降级（不拦任何条目）
  #[test]
  fn blocked_load_degrades_to_empty_on_missing_or_bad_json() {
    let (tmp, _dir) = harness();
    let root = tmp.path();
    assert!(blocked_load(root).is_empty(), "缺文件应为空清单");
    std::fs::write(root.join(BLOCKED_FILE), "不是 json").unwrap();
    assert!(blocked_load(root).is_empty(), "坏 JSON 应为空清单");
  }
}
