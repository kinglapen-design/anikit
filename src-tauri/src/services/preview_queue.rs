//! 预览解析队列（4.3 预览窗口）：单 worker 线程串行执行 PSD/PSB 全尺寸解码与序列帧 / 动图
//! 帧条生成（cache/frames，动图 2026-07-26 定案后帧原图 = 解包持久化的 info/frames/，帧条仍会话级）。
//!
//! 解决的问题（2026-07-25）：预览窗口快速切换素材时，解析命令在 tokio 线程池堆积且不取消，
//! 当前停留素材要等前面全部解析完；序列帧要等帧条全部生成完才能播放。
//!
//! 语义：**预览窗口只服务"当前停留"的素材**——submit 新任务时清空 pending 队列并代际 +1，
//! 旧任务一律作废；进行中的 FrameStrips 逐帧比对代际，切换即中断（FullPreview
//! 单文件解码不可中断，完成落盘无害——产物按源文件散列幂等，切回来直接命中）。
//!
//! 事件（app 为 None 时不发，仅执行落盘——单测用）：
//! - `preview-full-ready`：`{ assetId, path }`（FullPreview 完成且代际仍有效时）
//! - `preview-strips-ready`：`{ assetId, items: [{ name, stripPath }] }`（每凑满 8 帧或收尾一批）
//!
//! 注意（2026-07-25 踩坑）：**测试构建（cfg(test)）不持 AppHandle**——其方法 / 析构会把
//! tauri GUI 依赖链（rfd 的 comctl32!TaskDialogIndirect 等）拉进测试 exe，而测试 exe 无
//! comctl32 v6 清单，加载即 STATUS_ENTRYPOINT_NOT_FOUND (0xc0000139)、全部测试无法启动。
//! 故 cfg(test) 下 emit 为空实现、start 丢弃句柄（mem::forget，避免实例化其析构）。

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};

use serde::Serialize;
use tauri::AppHandle;
#[cfg(not(test))]
use tauri::Emitter;

use crate::services::thumbnail;

/// 全尺寸预览就绪事件名
pub const EVENT_FULL_READY: &str = "preview-full-ready";
/// 帧条批次就绪事件名
pub const EVENT_STRIPS_READY: &str = "preview-strips-ready";

/// 走全局队列（global()）的命令层测试互斥锁：队列语义是"新任务作废旧任务"，
/// 并行测试会互相顶掉任务（commands/preview.rs 的测试用）
#[cfg(test)]
pub(crate) static TEST_LOCK: Mutex<()> = Mutex::new(());

/// 帧条事件单批帧数：每凑满 8 帧推一批，前端播放条渐进显示（不等全部生成完）
const STRIP_BATCH: usize = 8;

/// 预览解析任务
#[derive(Debug)]
pub enum PreviewTask {
  /// psd/psb 全尺寸解码预览（cache/previews）
  FullPreview {
    /// 库根目录
    root: PathBuf,
    asset_id: String,
    /// 源文件绝对路径
    origin: PathBuf,
  },
  /// 序列帧帧条批量生成（cache/frames）：帧名 + 帧原图路径
  FrameStrips {
    /// 库根目录
    root: PathBuf,
    asset_id: String,
    /// (帧名, 帧原图绝对路径)，按播放顺序
    frames: Vec<(String, PathBuf)>,
  },
}

/// preview-full-ready 事件负载
#[cfg(not(test))]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FullReadyPayload {
  asset_id: String,
  path: String,
}

/// preview-strips-ready 事件负载
#[cfg(not(test))]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct StripsReadyPayload {
  asset_id: String,
  items: Vec<StripItem>,
}

/// 单条帧条结果
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct StripItem {
  name: String,
  strip_path: String,
}

/// worker 共享状态（Arc 让 worker 线程与 PreviewQueue 实例共用）
struct Shared {
  /// 事件发射句柄（None = 不发事件只执行）；cfg(test) 不持有（见模块头注释）
  #[cfg(not(test))]
  app: Option<AppHandle>,
  pending: Mutex<VecDeque<(u64, PreviewTask)>>,
  condvar: Condvar,
  /// 代际号：每次 submit +1，pending 与进行中的旧代际任务全部作废
  generation: AtomicU64,
}

impl Shared {
  /// 当前代际是否仍有效（任务取到时 / 每帧执行前 / 事件发射前比对）
  fn is_current(&self, gen: u64) -> bool {
    gen == self.generation.load(Ordering::SeqCst)
  }
}

#[cfg(not(test))]
impl Shared {
  fn emit_full_ready(&self, asset_id: String, path: String) {
    if let Some(app) = &self.app {
      if let Err(e) = app.emit(EVENT_FULL_READY, FullReadyPayload { asset_id, path }) {
        log::debug!("{EVENT_FULL_READY} 事件推送失败（前端可能已关闭）：{e}");
      }
    }
  }

  fn emit_strips_ready(&self, asset_id: &str, items: Vec<StripItem>) {
    if let Some(app) = &self.app {
      let payload = StripsReadyPayload { asset_id: asset_id.to_string(), items };
      if let Err(e) = app.emit(EVENT_STRIPS_READY, payload) {
        log::debug!("{EVENT_STRIPS_READY} 事件推送失败（前端可能已关闭）：{e}");
      }
    }
  }
}

/// cfg(test) 空实现：测试构建不发事件（见模块头注释）；引用事件名常量避免 dead_code
#[cfg(test)]
impl Shared {
  fn emit_full_ready(&self, _: String, _: String) {
    let _ = EVENT_FULL_READY;
  }
  fn emit_strips_ready(&self, _: &str, _: Vec<StripItem>) {
    let _ = EVENT_STRIPS_READY;
  }
}

/// 预览解析队列。可独立实例化（单测），生产用 global() 单例
pub struct PreviewQueue {
  shared: Arc<Shared>,
}

impl PreviewQueue {
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
      generation: AtomicU64::new(0),
    });
    let worker = Arc::clone(&shared);
    std::thread::Builder::new()
      .name("preview-queue".to_string())
      .spawn(move || worker_loop(&worker))
      .expect("预览解析队列 worker 线程启动失败");
    Self { shared }
  }

  /// 提交任务：**清空 pending 队列并代际 +1**——旧任务（未开始的与进行中的）一律作废，
  /// 队列里只保留当前停留素材的解析任务
  pub fn submit(&self, task: PreviewTask) {
    let gen = self.shared.generation.fetch_add(1, Ordering::SeqCst) + 1;
    let mut q = self.shared.pending.lock().unwrap();
    q.clear();
    q.push_back((gen, task));
    drop(q);
    self.shared.condvar.notify_one();
  }
}

/// 全局单例（生产）
static GLOBAL: OnceLock<PreviewQueue> = OnceLock::new();

/// 初始化全局队列（lib.rs setup 调用）：绑定 AppHandle，事件可达前端
pub fn init(app: AppHandle) {
  let _ = GLOBAL.set(PreviewQueue::start(Some(app)));
}

/// 全局队列。未 init 时（单测）惰性启动无事件实例——只执行落盘不发事件
pub fn global() -> &'static PreviewQueue {
  GLOBAL.get_or_init(|| PreviewQueue::start(None))
}

/// worker 主循环：取任务 → 比对代际（不符直接丢弃）→ 执行；空闲 park
fn worker_loop(shared: &Shared) {
  loop {
    let (gen, task) = {
      let mut q = shared.pending.lock().unwrap();
      loop {
        if let Some(item) = q.pop_front() {
          break item;
        }
        q = shared.condvar.wait(q).unwrap();
      }
    };
    if !shared.is_current(gen) {
      continue;
    }
    execute(shared, gen, task);
  }
}

fn execute(shared: &Shared, gen: u64, task: PreviewTask) {
  match task {
    PreviewTask::FullPreview { root, asset_id, origin } => {
      // 单文件解码不可中断（秒级）：完成落盘无害，产物按源文件散列幂等可复用；
      // 但代际已切换就不再发事件（前端已切走）
      if let Some(path) = thumbnail::generate_full_preview(&root, &origin, &asset_id) {
        if shared.is_current(gen) {
          shared.emit_full_ready(asset_id, path);
        }
      }
    }
    PreviewTask::FrameStrips { root, asset_id, frames } => {
      let total = frames.len();
      let mut items: Vec<StripItem> = Vec::with_capacity(STRIP_BATCH);
      for (idx, (name, frame_path)) in frames.into_iter().enumerate() {
        // 每帧执行前查代际：切换素材即中断，后续帧不再生成
        if !shared.is_current(gen) {
          return;
        }
        // 解码失败帧跳过（stripPath 维持 null，前端用原图兜底）
        if let Some(strip_path) = thumbnail::generate_frame_strip(&root, &frame_path, &asset_id, &name) {
          items.push(StripItem { name, strip_path });
        }
        // 每凑满 8 帧（或收尾）推一批；代际已切换则整批丢弃
        if items.len() >= STRIP_BATCH || (idx + 1 == total && !items.is_empty()) {
          if !shared.is_current(gen) {
            return;
          }
          shared.emit_strips_ready(&asset_id, std::mem::take(&mut items));
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::path::Path;
  use std::time::{Duration, Instant};

  use crate::core::library::CACHE_DIR;

  fn make_png(path: &Path, w: u32, h: u32) {
    image::RgbaImage::from_pixel(w, h, image::Rgba([30, 200, 30, 255])).save(path).unwrap();
  }

  fn strip_file(root: &Path, asset_id: &str, frame_name: &str) -> PathBuf {
    root.join(CACHE_DIR).join("frames").join(asset_id).join(format!("{frame_name}.webp"))
  }

  fn make_frames(root: &Path, dir_name: &str, count: u32, w: u32, h: u32) -> Vec<(String, PathBuf)> {
    let dir = root.join(dir_name);
    std::fs::create_dir_all(&dir).unwrap();
    (1..=count)
      .map(|n| {
        let name = format!("帧-{n:03}.png");
        let p = dir.join(&name);
        make_png(&p, w, h);
        (name, p)
      })
      .collect()
  }

  /// 轮询等待条件成立（10s 超时判定失败），不裸 sleep 长等
  fn wait_until(mut cond: impl FnMut() -> bool, what: &str) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !cond() {
      assert!(Instant::now() < deadline, "等待超时：{what}");
      std::thread::sleep(Duration::from_millis(5));
    }
  }

  /// start(None) 不发事件只执行：帧条逐帧落盘，产物规格正确
  #[test]
  fn frame_strips_generate_products() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let frames = make_frames(&root, "src", 3, 640, 480);

    let queue = PreviewQueue::start(None);
    queue.submit(PreviewTask::FrameStrips { root: root.clone(), asset_id: "A1".to_string(), frames: frames.clone() });

    wait_until(|| frames.iter().all(|(name, _)| strip_file(&root, "A1", name).is_file()), "帧条全部落盘");
    for (name, _) in &frames {
      let strip = strip_file(&root, "A1", name);
      let (w, h) = image::image_dimensions(&strip).unwrap();
      assert_eq!(w.min(h), 128, "帧条短边应为 128：{w}x{h}");
    }
  }

  /// submit 清空 pending：被顶替的旧任务代际作废，永远不执行
  /// （worker 被大图全尺寸预览占住 ≥1s，OLD/NEW 提交间隔微秒级，OLD 必在 pending 中被清掉；
  /// 即使 worker 尚未取走大图，大图也会被一并清掉——两种时序下 OLD 都不会执行，结论确定）
  #[test]
  fn submit_clears_pending_and_superseded_task_never_runs() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    // 大图占住 worker（解码 + 缩放 + webp 编码，秒级）
    let big = root.join("big.png");
    make_png(&big, 8000, 6000);
    let old_frames = make_frames(&root, "old", 3, 640, 480);
    let new_frames = make_frames(&root, "new", 2, 640, 480);

    let queue = PreviewQueue::start(None);
    queue.submit(PreviewTask::FullPreview { root: root.clone(), asset_id: "BIG".to_string(), origin: big });
    queue.submit(PreviewTask::FrameStrips { root: root.clone(), asset_id: "OLD".to_string(), frames: old_frames.clone() });
    queue.submit(PreviewTask::FrameStrips { root: root.clone(), asset_id: "NEW".to_string(), frames: new_frames.clone() });

    wait_until(|| new_frames.iter().all(|(name, _)| strip_file(&root, "NEW", name).is_file()), "NEW 帧条落盘");
    // OLD 被顶替作废：一帧都不应生成
    for (name, _) in &old_frames {
      assert!(!strip_file(&root, "OLD", name).exists(), "被作废任务的帧条不应生成：{name}");
    }
  }

  /// 进行中的 FrameStrips 逐帧查代际：切换素材即中断，后续帧不再生成
  #[test]
  fn running_frame_strips_interrupted_on_generation_switch() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    // 12 帧较大图（单帧数十毫秒），保证提交新任务时任务仍在进行中
    let frames_a = make_frames(&root, "a", 12, 2000, 1500);
    let frames_b = make_frames(&root, "b", 1, 640, 480);

    let queue = PreviewQueue::start(None);
    queue.submit(PreviewTask::FrameStrips { root: root.clone(), asset_id: "A".to_string(), frames: frames_a.clone() });

    // 等 A 首帧落盘（确认已开始执行），立即切换素材
    wait_until(|| strip_file(&root, "A", &frames_a[0].0).is_file(), "A 首帧落盘");
    queue.submit(PreviewTask::FrameStrips { root: root.clone(), asset_id: "B".to_string(), frames: frames_b.clone() });

    wait_until(|| strip_file(&root, "B", &frames_b[0].0).is_file(), "B 帧条落盘");
    // B 完成后 worker 已空闲：A 中断了，12 帧不应全部生成
    let done = frames_a.iter().filter(|(name, _)| strip_file(&root, "A", name).is_file()).count();
    assert!((1..12).contains(&done), "A 应中断于中途，实际已生成 {done}/12 帧");
  }
}
