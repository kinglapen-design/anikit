//! 媒体后台队列（2026-07-26 动图解包；2026-07-28 扩展视频 scrub 帧预抽——导入期生成，
//! 悬停毫秒级命中，Eagle 手感的来源即"帧图提前就绪"）。
//! 仿 preview_queue 结构（单 worker 线程 + Condvar），但**无代际作废语义**——解包是建档的
//! 一部分（丢了资产就永远没有帧图），不是"当前停留素材"的预览解析，任务排队直到完成；
//! 库切换时清空 pending（任务绑定库 root，切库后无意义，进行中的任务落盘幂等无害）。
//!
//! 挂钩：import_task 单元注册完成时——资产为动图（anim::is_anim）submit 解包；为视频
//! （media::VIDEO_EXTS）submit_scrub 预抽（手动导入 / 收件箱同一管线）。库打开时不自动补
//! （本轮新导入才有；scrub 帧另由悬停命令 ensure 补抽存量与预抽失败，全部场景闭合）。
//!
//! 事件（app 为 None 时不发，仅执行落盘——单测用）：
//! - `anim-unpacked`：`{ assetId }`（解包完成，前端预览中收到后重拉 list_sequence_frames 切帧驱动）
//! - `anim-unpack-failed`：`{ assetId }`（解码失败；资产仍可用，维持原生 `<img>` 播放兜底）
//! - scrub 预抽不发事件：前端无 scrub 事件消费者（悬停 invoke 时命中缓存或 ensure 补抽）
//!
//! cfg(test) 不持 AppHandle（同 preview_queue：tauri GUI 依赖链会拖垮测试 exe，见踩坑 014）——
//! emit 为空实现、不执行 apply_meta（metadata 同步归非测试构建）。

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, OnceLock};

#[cfg(not(test))]
use serde::Serialize;
use tauri::AppHandle;
#[cfg(not(test))]
use tauri::{Emitter, Manager};

use crate::services::anim_unpack::{self, UnpackOutcome};

/// 解包完成事件名
pub const EVENT_UNPACKED: &str = "anim-unpacked";
/// 解包失败事件名
pub const EVENT_UNPACK_FAILED: &str = "anim-unpack-failed";

/// 动图解包任务
#[derive(Debug)]
pub struct UnpackTask {
  /// 库根目录
  pub root: PathBuf,
  pub asset_id: String,
}

/// 后台任务（2026-07-28 扩展）：动图解包 / 视频 scrub 帧预抽
#[derive(Debug)]
pub enum QueueTask {
  Unpack(UnpackTask),
  /// 视频 scrub 预览帧预抽（cache/scrub/，services::scrub_frames）
  Scrub { root: PathBuf, asset_id: String },
}

impl QueueTask {
  /// 去重键（同类型同资产才互斥：动图解包与视频抽帧不会同资产，但防御性区分）
  fn dedup_key(&self) -> (&'static str, &str) {
    match self {
      QueueTask::Unpack(t) => ("unpack", &t.asset_id),
      QueueTask::Scrub { asset_id, .. } => ("scrub", asset_id),
    }
  }
}

/// 事件负载（两个事件同形状）
#[cfg(not(test))]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UnpackPayload {
  asset_id: String,
}

/// worker 共享状态
struct Shared {
  /// 事件发射 + metadata 短锁同步句柄（None = 不发事件只落盘）；cfg(test) 不持有（见模块头注释）
  #[cfg(not(test))]
  app: Option<AppHandle>,
  pending: Mutex<VecDeque<QueueTask>>,
  condvar: Condvar,
}

#[cfg(not(test))]
impl Shared {
  fn emit(&self, event: &str, asset_id: &str) {
    if let Some(app) = &self.app {
      let payload = UnpackPayload { asset_id: asset_id.to_string() };
      if let Err(e) = app.emit(event, payload) {
        log::debug!("{event} 事件推送失败（前端可能已关闭）：{e}");
      }
    }
  }

  /// 解包完成后的 metadata 同步（短锁）：thumbFrame 兜底 + modifiedAt + 内存索引
  fn apply_meta(&self, asset_id: &str, first_frame: &str) {
    let Some(app) = &self.app else { return };
    let state = app.state::<crate::commands::library::AppState>();
    match state.library.lock() {
      Ok(mut guard) => {
        if let Some(lib) = guard.as_mut() {
          if let Err(e) = anim_unpack::apply_meta(lib, asset_id, first_frame) {
            log::warn!("动图解包 metadata 同步失败（{asset_id}）：{e}");
          }
        }
      }
      Err(_) => log::warn!("动图解包 metadata 同步失败（{asset_id}）：应用状态锁冲突"),
    };
  }
}

/// cfg(test) 空实现：不发事件、不同步 metadata（见模块头注释）；引用事件名常量避免 dead_code
#[cfg(test)]
impl Shared {
  fn emit(&self, event: &str, _: &str) {
    let _ = (EVENT_UNPACKED, EVENT_UNPACK_FAILED, event);
  }
  fn apply_meta(&self, _: &str, _: &str) {}
}

/// 动图解包队列。可独立实例化（单测），生产用 global() 单例
pub struct UnpackQueue {
  shared: Arc<Shared>,
}

impl UnpackQueue {
  /// 启动队列：worker 线程常驻，空闲时 park 在 Condvar 上。app 为 None 不发事件只执行（单测用）。
  /// cfg(test) 下句柄直接丢弃（mem::forget，避免实例化 AppHandle 析构，见模块头注释）
  pub fn start(app: Option<AppHandle>) -> Self {
    #[cfg(test)]
    std::mem::forget(app);
    let shared = Arc::new(Shared {
      #[cfg(not(test))]
      app,
      pending: Mutex::new(VecDeque::new()),
      condvar: Condvar::new(),
    });
    let worker = Arc::clone(&shared);
    std::thread::Builder::new()
      .name("anim-unpack".to_string())
      .spawn(move || worker_loop(&worker))
      .expect("动图解包队列 worker 线程启动失败");
    Self { shared }
  }

  /// 提交解包任务：**排队直到完成**（无代际作废——解包是建档的一部分，不能随切换丢弃）。
  /// 同资产 pending 去重（重复提交幂等无害——unpack_frames 完成态跳过，但白跑一趟全量解码）
  pub fn submit(&self, task: UnpackTask) {
    self.push(QueueTask::Unpack(task));
  }

  /// 提交视频 scrub 帧预抽任务（2026-07-28 导入期预抽定案）：排队直到完成；
  /// 已缓存 / 生成中撞车由 services::scrub_frames 幂等收口
  pub fn submit_scrub(&self, root: PathBuf, asset_id: String) {
    self.push(QueueTask::Scrub { root, asset_id });
  }

  fn push(&self, task: QueueTask) {
    {
      let mut q = self.shared.pending.lock().unwrap();
      if !push_dedup(&mut q, task) {
        return;
      }
    }
    self.shared.condvar.notify_one();
  }

  /// 清空 pending（库切换调用；进行中的任务不可中断，落盘幂等无害）
  pub fn clear(&self) {
    self.shared.pending.lock().unwrap().clear();
  }
}

/// 全局单例（生产）
static GLOBAL: OnceLock<UnpackQueue> = OnceLock::new();

/// 初始化全局队列（lib.rs setup 调用）：绑定 AppHandle，事件可达前端
pub fn init(app: AppHandle) {
  let _ = GLOBAL.set(UnpackQueue::start(Some(app)));
}

/// 全局队列。未 init 时（单测）惰性启动无事件实例——只执行落盘不发事件
pub fn global() -> &'static UnpackQueue {
  GLOBAL.get_or_init(|| UnpackQueue::start(None))
}

/// 库切换清空 pending（commands/library activate 调用；未初始化时无操作）
pub fn clear_pending() {
  if let Some(q) = GLOBAL.get() {
    q.clear();
  }
}

/// pending 尾插，同类型同资产去重；返回 false = 已在队列未插入
fn push_dedup(q: &mut VecDeque<QueueTask>, task: QueueTask) -> bool {
  let key = task.dedup_key();
  if q.iter().any(|t| t.dedup_key() == key) {
    return false;
  }
  q.push_back(task);
  true
}

/// worker 主循环：取任务 → 执行；空闲 park（无代际比对，任务排到就必执行）
fn worker_loop(shared: &Shared) {  loop {
    let task = {
      let mut q = shared.pending.lock().unwrap();
      loop {
        if let Some(task) = q.pop_front() {
          break task;
        }
        q = shared.condvar.wait(q).unwrap();
      }
    };
    execute(shared, task);
  }
}

fn execute(shared: &Shared, task: QueueTask) {
  match task {
    QueueTask::Unpack(t) => match anim_unpack::unpack_frames(&t.root, &t.asset_id) {
      Ok(UnpackOutcome::Unpacked { first_frame, .. }) => {
        shared.apply_meta(&t.asset_id, &first_frame);
        shared.emit(EVENT_UNPACKED, &t.asset_id);
      }
      Ok(UnpackOutcome::Skipped) => log::debug!("动图解包跳过（已解包或非动图）：{}", t.asset_id),
      Err(reason) => {
        log::warn!("动图解包失败（{}）：{reason}", t.asset_id);
        shared.emit(EVENT_UNPACK_FAILED, &t.asset_id);
      }
    },
    // scrub 帧预抽：只落盘不发事件（前端无消费者；失败下次悬停 ensure 补抽，资产仍可用）
    QueueTask::Scrub { root, asset_id } => {
      if let Err(reason) = crate::services::scrub_frames::generate_by_id(&root, &asset_id) {
        log::warn!("scrub 帧预抽失败（{asset_id}）：{reason}");
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::path::Path;
  use std::time::{Duration, Instant};

  use crate::core::library::{self, ASSTES_DIR};
  use crate::services::import::{self, PendingFile, Source};

  /// 轮询等待条件成立（10s 超时判定失败），不裸 sleep 长等
  fn wait_until(mut cond: impl FnMut() -> bool, what: &str) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !cond() {
      assert!(Instant::now() < deadline, "等待超时：{what}");
      std::thread::sleep(Duration::from_millis(5));
    }
  }

  struct Harness {
    _dir: tempfile::TempDir,
    root: PathBuf,
  }

  /// 建库并把真实样本入库（gif/webp 各一，build_asset_with_id 会 probe 写 frameCount）
  fn make_lib() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("测试库");
    let lib = library::create(&root).unwrap();
    drop(lib);
    for (sample, asset_id) in [("anim3.gif", "AN-GIF"), ("anim3.webp", "AN-WEBP")] {
      let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/test-assets").join(sample);
      let pf = PendingFile { path: src, chain: Vec::new() };
      import::build_asset_with_id(&root, &pf, "sha256:t", asset_id, Source::Copy, None).unwrap();
    }
    Harness { _dir: dir, root }
  }

  fn frame(h: &Harness, asset_id: &str, name: &str, n: u32) -> PathBuf {
    h.root.join(ASSTES_DIR).join(format!("{asset_id}.info")).join("frames").join(format!("{name}-{n:05}.webp"))
  }

  /// 解包全部落盘判定：3 帧 + frames.json（frames.json 在帧图之后写，必须一起等防竞态）
  fn unpacked(h: &Harness, asset_id: &str, name: &str) -> bool {
    (1..=3).all(|n| frame(h, asset_id, name, n).is_file())
      && h.root.join(ASSTES_DIR).join(format!("{asset_id}.info")).join(anim_unpack::FRAMES_FILE).is_file()
  }

  /// 无代际作废语义：连提两任务都执行（对比 preview_queue：submit 会顶替旧任务）
  #[test]
  fn queue_runs_all_submitted_tasks_no_invalidation() {
    let h = make_lib();
    let queue = UnpackQueue::start(None);
    queue.submit(UnpackTask { root: h.root.clone(), asset_id: "AN-GIF".to_string() });
    queue.submit(UnpackTask { root: h.root.clone(), asset_id: "AN-WEBP".to_string() });

    wait_until(|| unpacked(&h, "AN-GIF", "anim3") && unpacked(&h, "AN-WEBP", "anim3"), "两个解包任务全部完成");
    // 各自的 frames.json 也都落盘
    for id in ["AN-GIF", "AN-WEBP"] {
      let info = h.root.join(ASSTES_DIR).join(format!("{id}.info"));
      assert_eq!(anim_unpack::read_delays(&info).unwrap(), [200, 200, 200], "{id} delays");
    }
  }

  /// clear 清空 pending，且队列未被破坏（clear 后新任务照常执行）
  #[test]
  fn clear_drops_pending_and_queue_still_works() {
    let h = make_lib();
    let queue = UnpackQueue::start(None);
    queue.submit(UnpackTask { root: h.root.clone(), asset_id: "AN-GIF".to_string() });
    queue.clear();
    assert!(queue.shared.pending.lock().unwrap().is_empty(), "clear 应清空 pending");
    // 已被 worker 取走的任务落盘幂等无害；clear 后提交的任务照常执行
    queue.submit(UnpackTask { root: h.root.clone(), asset_id: "AN-WEBP".to_string() });
    wait_until(|| unpacked(&h, "AN-WEBP", "anim3"), "clear 后新任务照常执行");
  }

  /// 同类型同资产 pending 去重：重复提交只排一份；Unpack 与 Scrub 不互斥（防御性区分类型）
  #[test]
  fn push_dedup_skips_same_kind_and_asset() {
    let mut q = VecDeque::new();
    let u = |id: &str| QueueTask::Unpack(UnpackTask { root: PathBuf::from("/r"), asset_id: id.to_string() });
    let s = |id: &str| QueueTask::Scrub { root: PathBuf::from("/r"), asset_id: id.to_string() };
    assert!(push_dedup(&mut q, u("A")));
    assert!(!push_dedup(&mut q, u("A")), "同类型同资产重复提交应被去重");
    assert!(push_dedup(&mut q, u("B")));
    assert!(push_dedup(&mut q, s("A")), "不同类型同资产不互斥");
    assert!(!push_dedup(&mut q, s("A")), "Scrub 同资产重复提交应被去重");
    assert_eq!(q.len(), 3);
  }
}
