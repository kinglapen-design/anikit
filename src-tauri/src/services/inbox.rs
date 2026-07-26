//! 收件箱服务（PRD 05-01，阶段 3.2 落地；2026-07-25 语义重定 4.4）：库内 inbox/ 目录的完整
//! 生命周期——库打开 / 切换时 start（盘符挂载 + 文件监听 + 存量清点），库切换 / 退出时 stop。
//! Inbox = 工作中素材/模板快速入口，不是工程收纳器：**自动导入只放行白名单可视素材扩展名**
//! （AUTO_IMPORT_EXTS，不反向枚举工程格式——枚举不完）；白名单以外留在磁盘不自动导入，
//! 由"工程文件"区块做磁盘实况展示（snapshot，不建档不写库），用户右键手动导入（移走入库）。
//! 就绪批次串行复用 3.1 导入管线：Keep（丢入的文件夹保留层级）+ Move（过境语义：
//! 导入后即移走、目录常态为空）；hash 冲突走"导入即建档"既有规则——挂起收集，
//! 经 app.emit("inbox-import") 推给前端复用 11_导入进度条 / 10_重复添加提示。
//! 磁盘实况热刷新：watcher 任何路径事件 → app.emit("inbox-fs-changed")（前端去抖重扫）。
//! 实况快照分两层防卡（2026-07-25 用户批准）：快层 snapshot_fast 供 list_inbox_snapshot 同步返回
//! （文件大小取元数据、单文件占用探测；目录大小 / 占用先给 0 / false 占位）；慢层 snapshot_details
//! （递归大小、占用、shell 格式图标）由后台 detail worker 延迟 200ms 合并连发后计算，
//! 经 app.emit("inbox-disk-details") 推前端整体替换。自动导入 failed 残留（move 阶段探测后又被锁住）
//! 经 requeue_failed 重投监听等待队列，放手后自动重试。
//! 全部失败（目录创建失败、盘符挂载失败、监听启动失败）记日志降级，不影响库打开。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::adapters::{inbox_drive, inbox_watch};
use crate::commands::library::AppState;
use crate::services::import::{FailedItem, ImportOptions, SequenceMode, Source, Structure};
use crate::services::import_task::run_import;
use crate::services::thumbnail;

/// 库内收件箱目录名
pub const INBOX_DIR: &str = "inbox";
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
/// mp4/mov/avi/mpeg4/m4v；音频待 5.1 波形/元数据就位再扩）；
/// 白名单以外一律不自动导入——不反向枚举工程格式（枚举不完，2026-07-25 用户定案）
pub const AUTO_IMPORT_EXTS: &[&str] = &[
  "png", "jpg", "jpeg", "bmp", "webp", "gif", "psd", "psb", "svg", "ai", "mp4", "mov", "avi", "mpeg4", "m4v",
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
  /// 系统格式图标缓存绝对路径（cache/thumbs/{ext}.webp；目录恒 None，
  /// 快层只命中已缓存、details 层现场生成；None 时前端回退扩展名角标）
  pub icon_path: Option<String>,
}

/// 快层快照（list_inbox_snapshot 同步返回，必须快）：
/// 文件大小直接取元数据、占用做单文件探测（都便宜）；目录的递归大小 / 占用探测贵，
/// 先给 0 / false 占位，由后台 details 补齐（前端收到 inbox-disk-details 后整体替换）。
/// 目录 auto_importable 仍按"递归扩展名全白名单"判定（只读目录项，不打开文件内容、不递归算大小）。
/// iconPath 只命中已缓存的格式图标（只判存在，绝不触发 shell——shell 调用只允许在 detail worker）。
pub fn snapshot_fast(root: &Path) -> Vec<InboxEntry> {
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
      icon_path: if is_dir { None } else { cached_icon_path(root, &path) },
    });
  }
  sort_entries(&mut out);
  out
}

/// 慢层完整快照（detail worker 后台算）：递归大小 + 全部文件占用探测 + 格式图标
/// 现场生成（复用 4.2 按格式缓存，shell 取系统图标只写 cache/thumbs 不写 info）。
pub fn snapshot_details(root: &Path) -> Vec<InboxEntry> {
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
      icon_path: if is_dir { None } else { ensure_icon_path(root, &path) },
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

/// 快层用：格式图标已缓存才给路径（只判存在，不触发 shell）
fn cached_icon_path(root: &Path, path: &Path) -> Option<String> {
  let dest = thumbnail::format_icon_path(root, &ext_lower(path));
  dest.is_file().then(|| dest.to_string_lossy().into_owned())
}

/// 慢层用：生成 / 命中格式图标缓存（shell 调用只允许出现在 detail worker）；
/// 缓存写在库 cache/thumbs 下，不在 inbox 监听范围内，天然不会递归触发 fs-changed
fn ensure_icon_path(root: &Path, path: &Path) -> Option<String> {
  let ext = ext_lower(path);
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
    let details = snapshot_details(&root);
    // 先放行标记再推事件：计算期间的变化靠下一次 fs-changed 再起一拍
    DETAIL_RUNNING.store(false, Ordering::SeqCst);
    if let Err(e) = app.emit(DETAILS_EVENT, details) {
      log::debug!("收件箱 details 事件推送失败（前端可能已关闭）：{e}");
    }
  });
}

/// 收件箱运行实例（存 AppState.inbox，随库打开 / 切换 / 退出重建）
pub struct InboxHandle {
  stop: Arc<AtomicBool>,
  watch: Option<inbox_watch::WatchHandle>,
  dispatcher: Option<JoinHandle<()>>,
  /// 当前挂载的盘符（未挂载 None，托盘开关读写）
  drive: Option<char>,
}

impl InboxHandle {
  /// 重投路径进监听等待队列（自动导入 failed 残留重投；监听已停则静默丢弃）
  fn requeue(&self, paths: Vec<PathBuf>) {
    if let Some(w) = &self.watch {
      w.requeue(paths);
    }
  }
}

/// 启动收件箱（库打开 / 切换后调用）：先停旧实例；任一部件失败记日志降级继续
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
    let root = root.to_path_buf();
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
    move || {
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

/// 停止收件箱（库切换 / 应用退出）：停监听 → 等派发线程排空 → 卸载盘符（幂等）
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

/// 托盘开关：已挂载 → 卸载；未挂载且有打开的库 → 重挂（用户手动卸过的场景）
pub fn toggle_drive(app: &AppHandle) {
  let state = app.state::<AppState>();
  // 先取库 root（短锁），再动 inbox 锁，避免嵌套
  let root = state.library.lock().ok().and_then(|g| g.as_ref().map(|l| l.root.clone()));
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
          None => log::debug!("未打开素材库，收件箱盘符开关忽略"),
        },
      }
    }
  }
  sync_tray(app);
}

/// 派发循环：逐批跑导入；stop 置位或通道关闭即退出。
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
    let state = app.state::<AppState>();
    let opts = ImportOptions {
      structure: Structure::Keep,
      source: Source::Move,
      target_folder: None,
      paths,
      // 收件箱渠道标记：撞 hash 且已有资产同渠道时自动合并（PRD 01-2.3 注）
      imported_from: Some("inbox".to_string()),
      // 收件箱无弹窗交互，序列帧维持逐文件导入（4.1 留待，见 05_项目进度）
      sequence_mode: SequenceMode::Scatter,
    };
    let app_ev = app.clone();
    // 收件箱导入无 UI 取消入口（批次通常很小），cancel 恒 false
    let outcome = run_import(
      &state.library,
      root.clone(),
      opts,
      move |ev| {
        if let Err(e) = app_ev.emit(EVENT_NAME, &ev) {
          log::debug!("收件箱事件推送失败（前端可能已关闭）：{e}");
        }
      },
      &AtomicBool::new(false),
    );
    // failed 残留重投：move 阶段失败（探测后软件又锁住）的源文件留在 inbox 不会再自动重试，
    // 重投监听等待队列，放手后走常规就绪判定自动入库；conflicts 不重投（用户决策，源保留属预期）
    requeue_failed(&app, &root, &outcome.failed);
  }
}

/// 自动导入 failed 残留重投：取仍存在且位于 inbox 根内的 failed 源路径，
/// 从 AppState 取监听实例重投等待队列。手动 import_inbox_entry 的失败不调本函数
/// （用户显式动作，错误已回前端），但函数本身与触发来源无关，后续要统一可直接复用
pub fn requeue_failed(app: &AppHandle, root: &Path, failed: &[FailedItem]) {
  let inbox_root = root.join(INBOX_DIR);
  let paths: Vec<PathBuf> = failed
    .iter()
    .map(|f| PathBuf::from(&f.path))
    .filter(|p| p.starts_with(&inbox_root) && p.exists())
    .collect();
  if paths.is_empty() {
    return;
  }
  log::info!("收件箱 failed 残留重投 {} 项", paths.len());
  let state = app.state::<AppState>();
  match state.inbox.lock() {
    Ok(slot) => {
      if let Some(h) = slot.as_ref() {
        h.requeue(paths);
      }
    }
    Err(_) => log::error!("收件箱 failed 重投失败（锁冲突）"),
  };
}

/// 同步托盘菜单项：勾选态 + 盘符标注（无库 / 未挂载显示"未挂载"）
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
