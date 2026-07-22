//! 收件箱服务（PRD 05-01，阶段 3.2）：库内 inbox/ 目录的完整生命周期——
//! 库打开 / 切换时 start（盘符挂载 + 文件监听 + 存量清点），库切换 / 退出时 stop。
//! 就绪批次串行复用 3.1 导入管线：Keep（丢入的文件夹保留层级）+ Move（过境语义：
//! 导入后即移走、目录常态为空）；hash 冲突走"导入即建档"既有规则——挂起收集，
//! 经 app.emit("inbox-import") 推给前端复用 11_导入进度条 / 10_重复添加提示。
//! 全部失败（目录创建失败、盘符挂载失败、监听启动失败）记日志降级，不影响库打开。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;

use tauri::{AppHandle, Emitter, Manager};

use crate::adapters::{inbox_drive, inbox_watch};
use crate::commands::library::AppState;
use crate::services::import::{ImportOptions, Source, Structure};
use crate::services::import_task::run_import;

/// 库内收件箱目录名
pub const INBOX_DIR: &str = "inbox";
/// 前端监听事件名（负载 = ImportEvent，与导入 Channel 同一形状）
pub const EVENT_NAME: &str = "inbox-import";

/// 收件箱运行实例（存 AppState.inbox，随库打开 / 切换 / 退出重建）
pub struct InboxHandle {
  stop: Arc<AtomicBool>,
  watch: Option<inbox_watch::WatchHandle>,
  dispatcher: Option<JoinHandle<()>>,
  /// 当前挂载的盘符（未挂载 None，托盘开关读写）
  drive: Option<char>,
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
  let watch = match inbox_watch::spawn(dir.clone(), move |paths| {
    if tx.send(paths).is_err() {
      log::debug!("收件箱派发通道已关闭，就绪批次丢弃");
    }
  }) {
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

/// 派发循环：逐批跑导入；stop 置位或通道关闭即退出
fn dispatch_loop(app: AppHandle, root: PathBuf, rx: mpsc::Receiver<Vec<PathBuf>>, stop: Arc<AtomicBool>) {
  while !stop.load(Ordering::Relaxed) {
    let Ok(paths) = rx.recv() else { break };
    if stop.load(Ordering::Relaxed) {
      break;
    }
    let state = app.state::<AppState>();
    let opts = ImportOptions {
      structure: Structure::Keep,
      source: Source::Move,
      target_folder: None,
      paths,
      // 收件箱渠道标记：撞 hash 且已有资产同渠道时自动合并（PRD 01-2.3 注）
      imported_from: Some("inbox".to_string()),
    };
    let app_ev = app.clone();
    // 收件箱导入无 UI 取消入口（批次通常很小），cancel 恒 false
    run_import(
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
  }
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
