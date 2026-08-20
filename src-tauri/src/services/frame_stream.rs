//! 动图帧流服务（阶段四一轮·自研解码器铺量：动图卡播放源）。
//! 会话 = 解码线程（循环播 origin 源文件、按源帧 delay 节奏）+ 帧槽（恒持最新帧）；
//! WS server（127.0.0.1 动态端口）按会话推二进制帧 `[w u32le][h u32le][seq u64le][rgba]`。
//!
//! 路由：gif → adapters/decoder（ffmpeg-next）；webp → services/anim（image-webp 纯 Rust——
//! lgpl 构建无 libwebp decoder，踩坑 039）。缩放 = 真短边等比不足不放大（thumbnail::shrink_to_edge
//! 同口径）；目标短边 = 卡片显示尺寸（2026-08-20 用户定：缩小省资源、放大跟清晰），会话级 AtomicU32
//! 热更新（set_size 不重启解码、不回第一帧）。
//! 节奏：生产者按源帧 delay 睡眠（Windows 15.6ms 粒度坑 = 踩坑 040——动图 delay 普遍 ≥40ms 影响小；
//! 25.6/30fps 实测观感已流畅，后续要精确节奏可换"消费端 rAF"模型）。
//!
//! 会话上限 MAX_SESSIONS = 泄漏兜底（正常由前端 autoplayGate anim 池 50 调度）；
//! 会话线程 detach（stop 标记退出，不 join）；库切换 stop_all（会话绑定 assets 池路径）。
//! cfg(test) 不起 WS server（tauri GUI 依赖链拖垮测试 exe，踩坑 014），会话逻辑可单测。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use serde::Serialize;

use crate::core::json_io::read_json;
use crate::core::models::AssetMeta;
use crate::core::paths;
use crate::services::{anim, thumbnail};

/// 会话上限（前端 anim 池 50 之上留余量，防泄漏打爆 CPU）
const MAX_SESSIONS: usize = 64;

/// 目标短边 clamp 范围：下限防 0/异常尺寸，上限对齐卡片最大显示尺寸余量
/// （卡片缩放到最大档列宽也不过数百 px；预览对接走更高档时另行放开）
const MIN_EDGE: u32 = 16;
const MAX_EDGE: u32 = 1024;

fn clamp_edge(edge: u32) -> u32 {
  edge.clamp(MIN_EDGE, MAX_EDGE)
}

/// 帧二进制头长度（w u32 + h u32 + seq u64）
pub const FRAME_HEADER: usize = 16;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamStart {
  pub session: u64,
  pub port: u16,
}

#[derive(Clone, Default)]
struct Latest {
  w: u32,
  h: u32,
  seq: u64,
  rgba: Arc<Vec<u8>>,
}

struct Session {
  stop: AtomicBool,
  /// 目标短边（出帧尺寸热更新：解码循环每帧读取，卡片 ResizeObserver 驱动）
  target: AtomicU32,
  slot: Mutex<Latest>,
  tx: tokio::sync::watch::Sender<u64>,
}

impl Session {
  fn new(short_edge: u32) -> Arc<Self> {
    let (tx, _rx) = tokio::sync::watch::channel(0u64);
    Arc::new(Session {
      stop: AtomicBool::new(false),
      target: AtomicU32::new(clamp_edge(short_edge)),
      slot: Mutex::new(Latest::default()),
      tx,
    })
  }
}

pub struct FrameStream {
  sessions: Mutex<HashMap<u64, Arc<Session>>>,
  next: AtomicU64,
  port: AtomicU64, // u16 存这里（0 = 未启动）
}

static GLOBAL: OnceLock<FrameStream> = OnceLock::new();

pub fn global() -> &'static FrameStream {
  GLOBAL.get_or_init(|| FrameStream {
    sessions: Mutex::new(HashMap::new()),
    next: AtomicU64::new(1),
    port: AtomicU64::new(0),
  })
}

/// 启动 WS server（lib.rs setup 调用；端口动态分配，start 返回值带 port）
#[cfg(not(test))]
pub fn init(app: tauri::AppHandle) {
  let gs = global();
  tauri::async_runtime::spawn(async move {
    let listener = match tokio::net::TcpListener::bind("127.0.0.1:0").await {
      Ok(l) => l,
      Err(e) => {
        log::warn!("帧流 WS server 启动失败（动图卡回退静态图）：{e}");
        return;
      }
    };
    let port = listener.local_addr().map(|a| a.port()).unwrap_or(0);
    gs.port.store(port as u64, Ordering::Relaxed);
    log::info!("帧流 WS server 已启动：127.0.0.1:{port}");
    let _ = app; // 保留句柄语义（后续如需 emit 状态）
    while let Ok((stream, _)) = listener.accept().await {
      tokio::spawn(async move {
        if let Err(e) = serve_ws(stream).await {
          log::debug!("帧流 WS 连接结束：{e:?}");
        }
      });
    }
  });
}

#[cfg(not(test))]
async fn serve_ws(
  stream: tokio::net::TcpStream,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  use futures_util::{SinkExt, StreamExt};
  let mut session_id = 0u64;
  let ws = tokio_tungstenite::accept_hdr_async(
    stream,
    |req: &tokio_tungstenite::tungstenite::handshake::server::Request,
     resp: tokio_tungstenite::tungstenite::handshake::server::Response| {
      session_id = req.uri().path().trim_start_matches('/').parse().unwrap_or(0);
      Ok(resp)
    },
  )
  .await?;
  let Some(sess) = global().sessions.lock().unwrap().get(&session_id).cloned() else {
    return Ok(()); // 会话不存在（已 stop）：直接断开
  };
  let mut rx = sess.tx.subscribe();
  let (mut write, mut read) = ws.split();
  // 先推当前帧（已有内容时，新连接立刻有画面）
  {
    let f = sess.slot.lock().unwrap().clone();
    if f.seq > 0 {
      write.send(tokio_tungstenite::tungstenite::Message::Binary(frame_bytes(&f).into())).await?;
    }
  }
  let mut last = sess.slot.lock().unwrap().seq;
  loop {
    tokio::select! {
      changed = rx.changed() => {
        if changed.is_err() { break; }
        let f = sess.slot.lock().unwrap().clone();
        if f.seq != last {
          last = f.seq;
          write.send(tokio_tungstenite::tungstenite::Message::Binary(frame_bytes(&f).into())).await?;
        }
      }
      msg = read.next() => {
        match msg {
          Some(Ok(tokio_tungstenite::tungstenite::Message::Close(_))) | None => break,
          Some(Err(_)) => break,
          _ => {}
        }
      }
    }
  }
  Ok(())
}

/// [w u32le][h u32le][seq u64le][rgba...]
fn frame_bytes(f: &Latest) -> Vec<u8> {
  let mut out = Vec::with_capacity(FRAME_HEADER + f.rgba.len());
  out.extend_from_slice(&f.w.to_le_bytes());
  out.extend_from_slice(&f.h.to_le_bytes());
  out.extend_from_slice(&f.seq.to_le_bytes());
  out.extend_from_slice(&f.rgba);
  out
}

/// 开启动图帧流会话：读 metadata 判动图 → 路由解码桥 → 起会话线程。
/// short_edge = 目标短边（卡片显示尺寸，clamp [MIN_EDGE, MAX_EDGE]；不足不放大）。
/// 非动图 / metadata 缺失 / 超上限返回 Err（前端回退静态缩略图）。
pub fn start(assets_dir: &Path, asset_id: &str, short_edge: u32) -> Result<StreamStart, String> {
  let meta: AssetMeta = read_json(&paths::info_dir(assets_dir, asset_id).join("metadata.json"))
    .map_err(|e| format!("metadata 读取失败: {e}"))?;
  if !anim::is_anim(&meta) {
    return Err("非动图资产".to_string());
  }
  let gs = global();
  {
    let sessions = gs.sessions.lock().unwrap();
    if sessions.len() >= MAX_SESSIONS {
      return Err(format!("帧流会话已达上限 {MAX_SESSIONS}"));
    }
  }
  let origin = paths::origin_file(assets_dir, asset_id, &meta.origin_name);
  if !origin.is_file() {
    return Err("origin 文件不存在".to_string());
  }
  let sess = Session::new(short_edge);
  let id = gs.next.fetch_add(1, Ordering::Relaxed);
  let runner = Arc::clone(&sess);
  let ext = meta.ext.to_lowercase();
  std::thread::Builder::new()
    .name(format!("anim-stream-{id}"))
    .spawn(move || run_session(runner, origin, ext))
    .map_err(|e| format!("会话线程启动失败: {e}"))?;
  gs.sessions.lock().unwrap().insert(id, sess);
  Ok(StreamStart { session: id, port: gs.port.load(Ordering::Relaxed) as u16 })
}

/// 热更新目标短边（卡片尺寸跟随）：只改 atomic，解码循环下一帧按新尺寸出帧——
/// 不重启解码、不回第一帧（对齐 2026-08-20 用户"随实际大小跟随"要求）。会话不存在静默忽略。
pub fn set_size(session: u64, short_edge: u32) {
  if let Some(sess) = global().sessions.lock().unwrap().get(&session) {
    sess.target.store(clamp_edge(short_edge), Ordering::Relaxed);
  }
}

/// 停止会话（幂等）：标记停止 + 移出会话表；线程下一轮循环退出
pub fn stop(session: u64) {
  if let Some(sess) = global().sessions.lock().unwrap().remove(&session) {
    sess.stop.store(true, Ordering::Relaxed);
  }
}

/// 停止全部会话（库切换调用：会话绑定 assets 池路径，切库后无意义）
pub fn stop_all() {
  let drained: Vec<Arc<Session>> = global().sessions.lock().unwrap().drain().map(|(_, s)| s).collect();
  for sess in drained {
    sess.stop.store(true, Ordering::Relaxed);
  }
}

fn run_session(sess: Arc<Session>, origin: PathBuf, ext: String) {
  loop {
    if sess.stop.load(Ordering::Relaxed) {
      return;
    }
    let alive = match ext.as_str() {
      "webp" => run_webp_once(&sess, &origin),
      _ => run_ffmpeg_once(&sess, &origin), // gif 及其余动图走 ffmpeg
    };
    if !alive {
      return; // 解码失败 / 被停止：不循环（防死循环刷错误日志）
    }
  }
}

/// gif 桥（ffmpeg）：单趟解码循环播放；Ok(true)= 自然到底继续循环，false = 停止/失败
/// 目标短边由 decode_once 每帧读 sess.target（热更新）
fn run_ffmpeg_once(sess: &Arc<Session>, origin: &Path) -> bool {
  let r = crate::adapters::decoder::decode_once(origin, &sess.target, &mut |f| {
    if sess.stop.load(Ordering::Relaxed) {
      return false;
    }
    publish(sess, f.w, f.h, f.rgba);
    if f.delay_ms > 0 {
      std::thread::sleep(std::time::Duration::from_millis(f.delay_ms as u64));
    }
    true
  });
  matches!(r, Ok(())) && !sess.stop.load(Ordering::Relaxed)
}

/// webp 桥（image-webp 纯 Rust）：单趟流式逐帧 + 缩放（每帧读 sess.target 热更新），循环播放
fn run_webp_once(sess: &Arc<Session>, origin: &Path) -> bool {
  let r = anim::decode_frames(origin, |_i, img, delay| {
    if sess.stop.load(Ordering::Relaxed) {
      return false;
    }
    let (w, h, rgba) = scale_to_thumb(img, sess.target.load(Ordering::Relaxed));
    publish(sess, w, h, rgba);
    if delay > 0 {
      std::thread::sleep(std::time::Duration::from_millis(delay as u64));
    }
    true
  });
  r.is_some() && !sess.stop.load(Ordering::Relaxed)
}

/// 真短边等比不足不放大（复用 thumbnail::shrink_to_edge，与缩略图 / gif 桥口径严格一致），输出紧凑 RGBA
fn scale_to_thumb(img: image::RgbaImage, short_edge: u32) -> (u32, u32, Vec<u8>) {
  let out = thumbnail::shrink_to_edge(image::DynamicImage::ImageRgba8(img), short_edge).to_rgba8();
  let (w, h) = out.dimensions();
  (w, h, out.into_raw())
}

fn publish(sess: &Arc<Session>, w: u32, h: u32, rgba: Vec<u8>) {
  let seq = {
    let mut g = sess.slot.lock().unwrap();
    let s = g.seq + 1;
    *g = Latest { w, h, seq: s, rgba: Arc::new(rgba) };
    s
  };
  sess.tx.send_replace(seq);
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::time::{Duration, Instant};

  use crate::core::library;
  use crate::services::import::{self, PendingFile, Source};

  /// 会话类测试共享全局 sessions 表（stop_all / is_empty 断言会互踩）——串行执行
  static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

  struct Harness {
    _dir: tempfile::TempDir,
    assets_dir: PathBuf,
  }

  fn make_lib() -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let lib = library::create_test(dir.path());
    let assets_dir = lib.assets_dir.clone();
    drop(lib);
    Harness { _dir: dir, assets_dir }
  }

  /// 造 3 帧 gif 入库（红/绿/蓝各 50ms，128×96——set_size 用例需要缩放空间）
  fn add_gif(h: &Harness, asset_id: &str) {
    let colors = [[220, 40, 40, 255], [40, 200, 40, 255], [40, 40, 220, 255]];
    let src = h._dir.path().join("动.gif");
    let file = std::fs::File::create(&src).unwrap();
    let mut enc = image::codecs::gif::GifEncoder::new(file);
    enc.set_repeat(image::codecs::gif::Repeat::Infinite).unwrap();
    let frames: Vec<image::Frame> = colors
      .iter()
      .map(|c| {
        image::Frame::from_parts(
          image::RgbaImage::from_pixel(128, 96, image::Rgba(*c)),
          0,
          0,
          image::Delay::from_numer_denom_ms(50, 1),
        )
      })
      .collect();
    enc.encode_frames(frames).unwrap();
    let pf = PendingFile { path: src, chain: Vec::new() };
    import::build_asset_with_id(&h.assets_dir, &pf, "sha256:t", asset_id, Source::Copy, None).unwrap();
  }

  /// 真实 webp 样本入库（tools/test-assets/anim3.webp，3 帧 200ms）
  fn add_webp(h: &Harness, asset_id: &str) {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/test-assets/anim3.webp");
    let pf = PendingFile { path: src, chain: Vec::new() };
    import::build_asset_with_id(&h.assets_dir, &pf, "sha256:t", asset_id, Source::Copy, None).unwrap();
  }

  fn wait_frame(id: u64, what: &str) {
    wait_seq(id, 1, what);
  }

  /// 等会话帧序号 ≥ at_least（set_size 后等新尺寸帧：传当前 seq+1）
  fn wait_seq(id: u64, at_least: u64, what: &str) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
      let seq = global().sessions.lock().unwrap().get(&id).map(|s| s.slot.lock().unwrap().seq);
      if seq.unwrap_or(0) >= at_least {
        return;
      }
      assert!(Instant::now() < deadline, "等待超时：{what}");
      std::thread::sleep(Duration::from_millis(5));
    }
  }

  fn cur_frame(id: u64) -> Option<Latest> {
    global().sessions.lock().unwrap().get(&id).map(|s| s.slot.lock().unwrap().clone())
  }

  /// gif 会话：出帧（128×96 源目标 320 不足不放大出原尺寸）→ stop 后会话移出表
  #[test]
  fn gif_session_streams_and_stops() {
    let _g = TEST_LOCK.lock().unwrap();
    let h = make_lib();
    add_gif(&h, "AN-S1");
    let st = start(&h.assets_dir, "AN-S1", 320).unwrap();
    wait_frame(st.session, "gif 会话出帧");
    {
      let f = cur_frame(st.session).unwrap();
      assert_eq!((f.w, f.h), (128, 96));
      assert!(f.rgba.len() == (f.w * f.h * 4) as usize);
    }
    stop(st.session);
    let s1 = {
      let g = global().sessions.lock().unwrap();
      g.get(&st.session).map(|s| s.slot.lock().unwrap().seq)
    };
    assert!(s1.is_none(), "stop 后会话应移出表");
  }

  /// set_size 热更新：128×96 源 320 档出 128×96 → 改 48 → 下一帧起 64×48（真短边等比）
  #[test]
  fn set_size_resizes_live() {
    let _g = TEST_LOCK.lock().unwrap();
    let h = make_lib();
    add_gif(&h, "AN-SZ");
    let st = start(&h.assets_dir, "AN-SZ", 320).unwrap();
    wait_frame(st.session, "出帧");
    let seq = cur_frame(st.session).unwrap().seq;
    set_size(st.session, 48);
    wait_seq(st.session, seq + 1, "新尺寸帧");
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
      let f = cur_frame(st.session).unwrap();
      if (f.w, f.h) == (64, 48) {
        break;
      }
      assert!(Instant::now() < deadline, "等待超时：帧尺寸未变为 64×48（当前 {}x{}）", f.w, f.h);
      std::thread::sleep(Duration::from_millis(5));
    }
    stop(st.session);
  }

  /// webp 会话：image-webp 桥出帧 → stop
  #[test]
  fn webp_session_streams_and_stops() {
    let _g = TEST_LOCK.lock().unwrap();
    let h = make_lib();
    add_webp(&h, "AN-S2");
    let st = start(&h.assets_dir, "AN-S2", 320).unwrap();
    wait_frame(st.session, "webp 会话出帧");
    stop(st.session);
  }

  /// 非动图 / 不存在资产：start 返回 Err 不起会话
  #[test]
  fn start_rejects_non_anim() {
    let _g = TEST_LOCK.lock().unwrap();
    let h = make_lib();
    let src = h._dir.path().join("静.png");
    image::RgbaImage::from_pixel(64, 48, image::Rgba([1, 2, 3, 255])).save(&src).unwrap();
    let pf = PendingFile { path: src, chain: Vec::new() };
    import::build_asset_with_id(&h.assets_dir, &pf, "sha256:t", "AN-PNG", Source::Copy, None).unwrap();
    assert!(start(&h.assets_dir, "AN-PNG", 320).is_err());
    assert!(start(&h.assets_dir, "AN-不存在", 320).is_err());
    assert!(global().sessions.lock().unwrap().is_empty());
  }

  /// stop_all 清空（库切换语义）
  #[test]
  fn stop_all_drains_sessions() {
    let _g = TEST_LOCK.lock().unwrap();
    let h = make_lib();
    add_gif(&h, "AN-S3");
    let st = start(&h.assets_dir, "AN-S3", 320).unwrap();
    wait_frame(st.session, "出帧");
    stop_all();
    assert!(global().sessions.lock().unwrap().is_empty());
  }
}
