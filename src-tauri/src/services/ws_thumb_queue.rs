//! 工作空间缩略图生成队列（2026-09-26 性能优化 A，PRD 02）：进程级常驻 worker 池取代
//! 原「每次 invoke 裸 spawn 一条线程 + 严格串行 for」（commands::workspace 直调
//! services::workspace::request_thumbs）——
//!
//! - **固定并发度**：N = 可用并行度一半（钳 1..=4）。解码 / 重采样 / webp 编码是 CPU 密集
//!   重活，留一半核给 UI 与系统；音视频逐文件 spawn ffmpeg 也随池限流（原串行 → 至多 N 并发）。
//!   结构仿 preview_queue / anim_preview_queue 自管 worker（Condvar 常驻线程）而非 rayon
//!   par_iter——任务来自持续到达的请求事件流、需常驻队列 + 跨请求去重 + 队首插队，
//!   不是一把批处理形态。
//! - **跨请求去重**：键 =（workspace 根, 缓存键茎 rel+mtime+size，同 workspace::job_key）；
//!   队列中 / 进行中已有的跳过，fs-changed 去抖重扫与视口优先补发带来的重复请求归零。
//! - **失败记忆**：worker 解码失败记 workspace::note_thumb_failed（进程级），入队前
//!   resolve_thumb_job 命中即跳过；失败条目由 failed_icon_fallback 落回按扩展名系统图标
//!   随事件带给前端（thumbKind=formatIcon）。
//! - **优先级**：priority=true 的批（前端视口内缺失项）整批插到队首（保批内顺序），
//!   普通批（目录全量补发）排尾部。
//!
//! 完成批次仍走 workspace-thumbs-ready 事件回填：凑满 BATCH 或队列完全排空时回吐一帧
//! （大目录首开不再每 invoke 一事件 / 每线程一事件，前端 patchThumbs 按批响应）。
//!
//! 事件（app 为 None 时不发，仅执行落盘——单测用）；cfg(test) 不持 AppHandle
//! （同 preview_queue：tauri GUI 依赖链会拖垮测试 exe，见踩坑 014）。

use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, OnceLock};

use tauri::AppHandle;
#[cfg(not(test))]
use tauri::Emitter;

use crate::services::views::ThumbKind;
use crate::services::workspace::{self, ThumbReady};

/// 事件回吐批大小：凑满即推（前端 patchThumbs 按批换新数组，逐条推会把大目录首开打成逐条重排）
const BATCH: usize = 16;

/// 单条生成任务（入队前已过 resolve_thumb_job：workspace 内文件、缓存不新鲜、非失败记忆）
#[derive(Debug)]
struct ThumbJob {
  /// workspace 根（缓存落点 + 去重键分量）
  root: PathBuf,
  /// 源文件绝对路径
  abs: PathBuf,
  /// 相对 workspace 根路径（/ 分隔，缓存键来源）
  rel: String,
  mtime_ms: i64,
  size: u64,
}

impl ThumbJob {
  /// 去重键：（根, 缓存键茎）——与失败记忆同身份口径，跨空间不撞
  fn dedup_key(&self) -> (PathBuf, String) {
    (self.root.clone(), workspace::job_key(&self.rel, self.mtime_ms, self.size))
  }
}

/// 队列状态（单 Mutex 全包：任务执行在锁外，锁内只有入队 / 取任务 / 记账，竞争可忽略）
#[derive(Default)]
struct QState {
  pending: VecDeque<ThumbJob>,
  /// 排队中 ∪ 进行中的去重键（完成时移除；任务出队不清——执行期间仍要挡住重复请求）
  keys: HashSet<(PathBuf, String)>,
  /// 正在执行中的 worker 数（排空判定：busy==0 且 pending 空 = 本批收尾，回吐余量）
  busy: usize,
  /// 待回吐结果缓冲
  ready: Vec<ThumbReady>,
}

/// worker 共享状态
struct Shared {
  /// 事件发射句柄（None = 不发事件只执行）；cfg(test) 不持有（见模块头注释）
  #[cfg(not(test))]
  app: Option<AppHandle>,
  state: Mutex<QState>,
  condvar: Condvar,
}

#[cfg(not(test))]
impl Shared {
  fn emit_ready(&self, items: Vec<ThumbReady>) {
    if items.is_empty() {
      return;
    }
    if let Some(app) = &self.app {
      if let Err(e) = app.emit(workspace::THUMBS_EVENT, items) {
        log::debug!("{} 事件推送失败（前端可能已关闭）：{e}", workspace::THUMBS_EVENT);
      }
    }
  }
}

/// cfg(test) 空实现：测试构建不发事件（见模块头注释）；引用事件名常量避免 dead_code
#[cfg(test)]
impl Shared {
  fn emit_ready(&self, _: Vec<ThumbReady>) {
    let _ = workspace::THUMBS_EVENT;
  }
}

/// 工作空间缩略图队列。可独立实例化（单测），生产用 global() 单例
pub struct WsThumbQueue {
  shared: Arc<Shared>,
}

/// worker 数 = 可用并行度一半（钳 1..=4）：解码重采样是 CPU 密集，留一半核给 UI / 系统
fn worker_count() -> usize {
  (std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4) / 2).clamp(1, 4)
}

/// 入队：priority 整批插到队首（rev 逐个 push_front 保批内顺序），普通批排尾部
fn push_jobs(pending: &mut VecDeque<ThumbJob>, jobs: Vec<ThumbJob>, priority: bool) {
  if priority {
    for j in jobs.into_iter().rev() {
      pending.push_front(j);
    }
  } else {
    pending.extend(jobs);
  }
}

impl WsThumbQueue {
  /// 启动队列：worker 常驻，空闲 park 在 Condvar 上。app 为 None 不发事件只执行（单测用）。
  /// cfg(test) 下句柄直接丢弃（mem::forget，避免实例化 AppHandle 析构，见模块头注释）
  pub fn start(app: Option<AppHandle>) -> Self {
    #[cfg(test)]
    std::mem::forget(app);
    let shared = Arc::new(Shared {
      #[cfg(not(test))]
      app,
      state: Mutex::new(QState::default()),
      condvar: Condvar::new(),
    });
    for i in 0..worker_count() {
      let worker = Arc::clone(&shared);
      std::thread::Builder::new()
        .name(format!("ws-thumb-{i}"))
        .spawn(move || worker_loop(&worker))
        .expect("工作空间缩略图 worker 线程启动失败");
    }
    Self { shared }
  }

  /// 提交一批路径：逐条解析（workspace 内文件 / 缓存不新鲜 / 非失败记忆）+ 去重后入队，
  /// 返回实际新入队条数（全部重复 / 已新鲜 / 已失败 = 0）
  pub fn submit(&self, root: PathBuf, abs_paths: Vec<PathBuf>, priority: bool) -> usize {
    let n = {
      let mut st = self.shared.state.lock().unwrap();
      let mut jobs = Vec::new();
      for abs in abs_paths {
        let Some((rel, mtime_ms, size)) = workspace::resolve_thumb_job(&root, &abs) else { continue };
        let job = ThumbJob { root: root.clone(), abs, rel, mtime_ms, size };
        if !st.keys.insert(job.dedup_key()) {
          continue; // 队列中 / 进行中已有同键——跨请求去重
        }
        jobs.push(job);
      }
      let n = jobs.len();
      push_jobs(&mut st.pending, jobs, priority);
      if n > 0 {
        log::debug!("工作空间缩略图入队 {n} 条（priority={priority}，排队 {} 条）", st.pending.len());
      }
      n
    };
    if n > 0 {
      // 出锁后再唤醒（避免唤醒后抢锁空转）；多 worker 共享一队列：批量到达全唤醒抢活
      self.shared.condvar.notify_all();
    }
    n
  }
}

/// 全局单例（生产）
static GLOBAL: OnceLock<WsThumbQueue> = OnceLock::new();

/// 初始化全局队列（lib.rs setup 调用）：绑定 AppHandle，事件可达前端
pub fn init(app: AppHandle) {
  let _ = GLOBAL.set(WsThumbQueue::start(Some(app)));
}

/// 全局队列。未 init 时（单测）惰性启动无事件实例——只执行落盘不发事件
pub fn global() -> &'static WsThumbQueue {
  GLOBAL.get_or_init(|| WsThumbQueue::start(None))
}

/// 切空间清空 pending（workspace::start_watch 调用；进行中的任务不可中断，落盘幂等无害）。
/// 排队任务的键同步从去重集摘除（不清会永远占键——它们再不会有完成时刻）
pub fn clear_pending() {
  let Some(q) = GLOBAL.get() else { return };
  let mut st = q.shared.state.lock().unwrap();
  let drained: Vec<ThumbJob> = st.pending.drain(..).collect();
  for job in drained {
    st.keys.remove(&job.dedup_key());
  }
}

/// worker 主循环：取任务（busy 记账与取任务同锁原子）→ 锁外执行 → 完成记账与回吐；空闲 park
fn worker_loop(shared: &Shared) {
  loop {
    let job = {
      let mut st = shared.state.lock().unwrap();
      loop {
        if let Some(job) = st.pending.pop_front() {
          st.busy += 1;
          break job;
        }
        st = shared.condvar.wait(st).unwrap();
      }
    };
    execute(shared, job);
  }
}

/// 执行单条：执行前重查新鲜度（排队期间可能已被另一趟生成）→ 生成；成功清失败记忆、
/// 失败记失败记忆并落回格式图标条目；完成记账后按批回吐（凑满 BATCH 或本批排空）
fn execute(shared: &Shared, job: ThumbJob) {
  let dedup = job.dedup_key();
  let result = if workspace::fresh_thumb_path(&job.root, &job.rel, job.mtime_ms, job.size).is_some() {
    None // 排队期间已新鲜：静默跳过（不记失败、不上事件）
  } else {
    match workspace::generate_thumb(&job.root, &job.abs, &job.rel, job.mtime_ms, job.size) {
      Some((dest, w, h)) => {
        workspace::clear_thumb_failed(&job.root, &job.rel, job.mtime_ms, job.size);
        Some(ThumbReady {
          abs_path: job.abs.to_string_lossy().into_owned(),
          thumb_path: dest.to_string_lossy().into_owned(),
          w,
          h,
          thumb_kind: ThumbKind::Asset,
        })
      }
      None => {
        log::warn!("工作空间缩略图生成失败（记入失败记忆，本会话不再重试）：{}", job.abs.display());
        workspace::note_thumb_failed(&job.root, &job.rel, job.mtime_ms, job.size);
        workspace::failed_icon_fallback(&job.root, &job.abs)
      }
    }
  };
  let mut st = shared.state.lock().unwrap();
  st.busy -= 1;
  st.keys.remove(&dedup);
  if let Some(item) = result {
    st.ready.push(item);
  }
  // 回吐时机：凑满 BATCH 或队列完全排空（本 worker 是最后一个在跑的且队列空 = 收尾一帧）
  if st.ready.len() >= BATCH || (st.busy == 0 && st.pending.is_empty()) {
    let items = std::mem::take(&mut st.ready);
    drop(st);
    shared.emit_ready(items);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::path::Path;
  use std::time::{Duration, Instant};

  fn make_ws() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("workspace");
    std::fs::create_dir_all(&root).unwrap();
    (dir, root)
  }

  fn make_png(path: &Path, w: u32, h: u32) {
    image::RgbaImage::from_pixel(w, h, image::Rgba([40, 40, 200, 255])).save(path).unwrap();
  }

  /// 轮询等待条件成立（10s 超时判定失败），不裸 sleep 长等
  fn wait_until(mut cond: impl FnMut() -> bool, what: &str) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !cond() {
      assert!(Instant::now() < deadline, "等待超时：{what}");
      std::thread::sleep(Duration::from_millis(5));
    }
  }

  fn job(root: &Path, rel: &str) -> ThumbJob {
    ThumbJob { root: root.to_path_buf(), abs: root.join(rel), rel: rel.to_string(), mtime_ms: 1, size: 1 }
  }

  /// worker 数公式：可用并行度一半，钳 1..=4
  #[test]
  fn worker_count_bounded() {
    assert!((1..=4).contains(&worker_count()));
  }

  /// 入队顺序：priority 整批插队首且保批内顺序，普通批排尾
  #[test]
  fn push_jobs_priority_jumps_front_preserving_order() {
    let root = Path::new("r");
    let mut pending = VecDeque::from([job(root, "old.png")]);
    push_jobs(&mut pending, vec![job(root, "p1.png"), job(root, "p2.png")], true);
    push_jobs(&mut pending, vec![job(root, "n1.png")], false);
    let order: Vec<&str> = pending.iter().map(|j| j.rel.as_str()).collect();
    assert_eq!(order, ["p1.png", "p2.png", "old.png", "n1.png"]);
  }

  /// 跨请求去重：在途 / 排队中的同文件重复提交归零；完成（缓存新鲜）后提交同样归零
  #[test]
  fn submit_dedups_queued_and_fresh() {
    let (_dir, root) = make_ws();
    let f = root.join("甲.png");
    make_png(&f, 1600, 1200);

    let q = WsThumbQueue::start(None);
    assert_eq!(q.submit(root.clone(), vec![f.clone()], false), 1, "首次入队 1 条");
    assert_eq!(q.submit(root.clone(), vec![f.clone()], false), 0, "在途 / 排队重复提交去重");
    wait_until(|| q.shared.state.lock().unwrap().keys.is_empty(), "任务完成（去重键移除）");
    let meta = std::fs::metadata(&f).unwrap();
    let mtime_ms = meta.modified().unwrap().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64;
    assert!(workspace::fresh_thumb_path(&root, "甲.png", mtime_ms, meta.len()).is_some(), "缓存已生成");
    assert_eq!(q.submit(root.clone(), vec![f], false), 0, "缓存已新鲜不再入队");
  }

  /// 失败记忆：解码失败记入后不再重试（重复提交 0）；缓存目录无产物
  #[test]
  fn failure_recorded_and_never_retried() {
    let (_dir, root) = make_ws();
    let f = root.join("坏.png");
    // IDAT 数据字节原地翻转：文件头完好（entry 判可解码会请求），数据段 CRC 失配 decode 必败
    make_png(&f, 64, 48);
    let mut bytes = std::fs::read(&f).unwrap();
    let mut pos = 8;
    while pos + 8 <= bytes.len() {
      let len = u32::from_be_bytes(bytes[pos..pos + 4].try_into().unwrap()) as usize;
      if &bytes[pos + 4..pos + 8] == b"IDAT" && len > 0 {
        bytes[pos + 8] ^= 0xFF;
        break;
      }
      pos += 12 + len;
    }
    std::fs::write(&f, &bytes).unwrap();
    let meta = std::fs::metadata(&f).unwrap();
    let mtime_ms = meta.modified().unwrap().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64;
    let size = meta.len();

    let q = WsThumbQueue::start(None);
    assert_eq!(q.submit(root.clone(), vec![f.clone()], false), 1);
    wait_until(|| workspace::is_thumb_failed(&root, "坏.png", mtime_ms, size), "失败记忆记入");
    assert_eq!(q.submit(root.clone(), vec![f], false), 0, "失败记忆命中不再入队");
  }

  /// 并发与批量回吐：4 个文件全部完成落盘（worker 池并行执行，最终全部产物新鲜）
  #[test]
  fn pool_generates_all() {
    let (_dir, root) = make_ws();
    let files: Vec<PathBuf> = (0..4)
      .map(|i| {
        let f = root.join(format!("图{i}.png"));
        make_png(&f, 320 + i * 10, 240);
        f
      })
      .collect();

    let q = WsThumbQueue::start(None);
    assert_eq!(q.submit(root.clone(), files.clone(), false), 4);
    wait_until(
      || {
        let st = q.shared.state.lock().unwrap();
        st.busy == 0 && st.pending.is_empty() && st.keys.is_empty()
      },
      "本批全部完成",
    );
    for (i, f) in files.iter().enumerate() {
      let meta = std::fs::metadata(f).unwrap();
      let mtime_ms = meta.modified().unwrap().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64;
      assert!(
        workspace::fresh_thumb_path(&root, &format!("图{i}.png"), mtime_ms, meta.len()).is_some(),
        "图{i}.png 缓存应已生成"
      );
    }
  }
}

