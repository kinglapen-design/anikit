//! 动图小动画生成队列（2026-09-02 恢复，2.3 时期挂在已删除的 unpack_queue）：单 worker 线程
//! 串行转码（ffmpeg sidecar 逐资产一个进程，串行即限流）。仿 preview_queue 结构但**无代际
//! 作废语义**——生成是建档的一部分（丢了资产就永远没有缓存），不是"当前停留素材"的预览解析，
//! 任务排队直到完成；库切换时清空 pending（任务绑定 assets 池目录，切库后无意义，进行中的
//! 任务落盘幂等无害）。
//!
//! 挂钩：import_task 单元注册完成且资产为动图（anim::is_anim）时 submit（手动导入 / 收件箱
//! 同一管线）；CEP 伴生 webp 入库（import_pair / 仅缩略图绑定）同提交；get_asset_file_path
//! 懒生成补提交（存量旧动图首次播放回退原文件、生成完成经事件热切换）。
//!
//! 事件（app 为 None 时不发，仅执行落盘——单测用）：
//! - `preview-anim-ready`：`{ assetId, path }`（生成/命中完成；前端卡片播放层热切换小动画）
//!
//! cfg(test) 不持 AppHandle（同 preview_queue：tauri GUI 依赖链会拖垮测试 exe，见踩坑 014）。

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, OnceLock};

#[cfg(not(test))]
use serde::Serialize;
use tauri::AppHandle;
#[cfg(not(test))]
use tauri::Emitter;

use crate::services::anim_preview;

/// 小动画就绪事件名
pub const EVENT_ANIM_READY: &str = "preview-anim-ready";

/// 小动画生成任务
#[derive(Debug)]
pub struct AnimPreviewTask {
  /// 空间级 assets 池目录（info 路径由 ID 推导）
  pub assets_dir: PathBuf,
  pub asset_id: String,
}

/// 事件负载
#[cfg(not(test))]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AnimReadyPayload {
  asset_id: String,
  path: String,
}

/// worker 共享状态
struct Shared {
  /// 事件发射句柄（None = 不发事件只执行）；cfg(test) 不持有（见模块头注释）
  #[cfg(not(test))]
  app: Option<AppHandle>,
  pending: Mutex<VecDeque<AnimPreviewTask>>,
  condvar: Condvar,
}

#[cfg(not(test))]
impl Shared {
  fn emit(&self, asset_id: &str, path: &PathBuf) {
    if let Some(app) = &self.app {
      let payload = AnimReadyPayload { asset_id: asset_id.to_string(), path: path.to_string_lossy().into_owned() };
      if let Err(e) = app.emit(EVENT_ANIM_READY, payload) {
        log::debug!("{EVENT_ANIM_READY} 事件推送失败（前端可能已关闭）：{e}");
      }
    }
  }
}

/// 小动画生成队列。可独立实例化（单测），生产用 global() 单例
pub struct AnimPreviewQueue {
  shared: Arc<Shared>,
}

impl AnimPreviewQueue {
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
      .name("anim-preview".to_string())
      .spawn(move || worker_loop(&worker))
      .expect("小动画生成队列 worker 线程启动失败");
    Self { shared }
  }

  /// 提交生成任务：**排队直到完成**（无代际作废——生成是建档的一部分，不能随切换丢弃）。
  /// 同资产 pending 去重（重复提交幂等无害——ensure 已新鲜跳过，但白跑一次判定）
  pub fn submit(&self, task: AnimPreviewTask) {
    {
      let mut q = self.shared.pending.lock().unwrap();
      if q.iter().any(|t| t.asset_id == task.asset_id) {
        return;
      }
      q.push_back(task);
    }
    self.shared.condvar.notify_one();
  }

  /// 清空 pending（库切换调用；进行中的任务不可中断，落盘幂等无害）
  pub fn clear(&self) {
    self.shared.pending.lock().unwrap().clear();
  }
}

/// 全局单例（生产）
static GLOBAL: OnceLock<AnimPreviewQueue> = OnceLock::new();

/// 初始化全局队列（lib.rs setup 调用）：绑定 AppHandle，事件可达前端
pub fn init(app: AppHandle) {
  let _ = GLOBAL.set(AnimPreviewQueue::start(Some(app)));
}

/// 全局队列。未 init 时（单测）惰性启动无事件实例——只执行落盘不发事件
pub fn global() -> &'static AnimPreviewQueue {
  GLOBAL.get_or_init(|| AnimPreviewQueue::start(None))
}

/// 库切换清空 pending（commands/library activate 调用；未初始化时无操作）
pub fn clear_pending() {
  if let Some(q) = GLOBAL.get() {
    q.clear();
  }
}

/// worker 主循环：取任务 → 执行；空闲 park（无代际比对，任务排到就必执行）
fn worker_loop(shared: &Shared) {
  loop {
    let task = {
      let mut q = shared.pending.lock().unwrap();
      loop {
        if let Some(task) = q.pop_front() {
          break task;
        }
        q = shared.condvar.wait(q).unwrap();
      }
    };
    if let Some(path) = anim_preview::ensure_anim_preview(&task.assets_dir, &task.asset_id) {
      #[cfg(not(test))]
      shared.emit(&task.asset_id, &path);
      #[cfg(test)]
      let _ = (EVENT_ANIM_READY, path); // 引用常量防 dead_code（emit 归非测试构建）
    } else {
      log::debug!("小动画缓存未生成（非动图 / sidecar 缺失 / 转码失败，前端回退原文件播放）：{}", task.asset_id);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::BufReader;
  use std::path::Path;
  use std::time::{Duration, Instant};

  use crate::core::library;
  use crate::services::import::{self, PendingFile, Source};

  /// 轮询等待条件成立（10s 超时判定失败），不裸 sleep 长等
  fn wait_until(mut cond: impl FnMut() -> bool, what: &str) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !cond() {
      assert!(Instant::now() < deadline, "等待超时：{what}");
      std::thread::sleep(Duration::from_millis(5));
    }
  }

  fn make_gif(path: &Path) {
    let file = std::fs::File::create(path).unwrap();
    let mut enc = image::codecs::gif::GifEncoder::new(file);
    enc.set_repeat(image::codecs::gif::Repeat::Infinite).unwrap();
    let frames: Vec<image::Frame> = [[220, 40, 40, 255], [40, 200, 40, 255]]
      .iter()
      .map(|c| {
        image::Frame::from_parts(
          image::RgbaImage::from_pixel(64, 48, image::Rgba(*c)),
          0,
          0,
          image::Delay::from_numer_denom_ms(100, 1),
        )
      })
      .collect();
    enc.encode_frames(frames).unwrap();
  }

  #[test]
  fn queue_generates_and_dedups() {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    let src = dir.path().join("动图.gif");
    make_gif(&src);
    let pf = PendingFile { path: src, chain: Vec::new() };
    let meta = import::build_asset_with_id(&lib.assets_dir, &pf, "sha256:t", "AN-Q", Source::Copy, None).unwrap();

    let q = AnimPreviewQueue::start(None);
    let task = || AnimPreviewTask { assets_dir: lib.assets_dir.clone(), asset_id: "AN-Q".to_string() };
    q.submit(task());
    q.submit(task()); // 同资产 pending 去重：第二次直接丢弃
    assert_eq!(q.shared.pending.lock().unwrap().len(), 1);

    let dest = crate::core::paths::anim_preview_file(&lib.assets_dir, "AN-Q", &meta.origin_name);
    wait_until(|| dest.is_file(), "队列生成小动画");
    let dec = image_webp::WebPDecoder::new(BufReader::new(std::fs::File::open(&dest).unwrap())).unwrap();
    assert!(dec.is_animated(), "产物应为 animated webp");

    // 已新鲜后再提交：ensure 快路径直接命中（不重复转码，mtime 不变）
    let mtime = std::fs::metadata(&dest).unwrap().modified().unwrap();
    q.submit(task());
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(std::fs::metadata(&dest).unwrap().modified().unwrap(), mtime);
  }
}
