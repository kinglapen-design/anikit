//! 动图帧流服务（阶段四·自研解码器：动图卡播放源 + 预览窗口动图播放）。
//! 会话 = 解码线程（循环播 origin 源文件、按源帧 delay 节奏）+ 帧槽（恒持最新帧）；
//! WS server（127.0.0.1 动态端口）按会话推流（协议 v2，二进制帧 tag 0）：
//! `[0][w u32le][h u32le][idx u32le][pts_ms f64le][seq u64le][rgba]`
//! （idx = 源帧索引 0 起、循环回绕归 0——预览播放条帧号 / 进度用；pts_ms = 帧时刻（趟内累计 delay）；
//! seq 单调递增推送序号）。
//!
//! 路由：gif → adapters/decoder（ffmpeg-next）；webp → services/anim（image-webp 纯 Rust——
//! lgpl 构建无 libwebp decoder，踩坑 039）。缩放 = 真短边等比不足不放大（thumbnail::shrink_to_edge
//! 同口径）；目标短边 = 调用方显示尺寸（卡片 = 卡片实测短边 clamp 1024；预览 = 视口×zoom clamp 4096），
//! 会话级 AtomicU32 热更新（set_size 不重启解码、不回第一帧）。
//! 节奏 = 累计 deadline（next_due += delay/rate，sleep 差值；落后不追赶防爆发）——消 Windows sleep
//! 15.6ms 粒度漂移（踩坑 040）；init 时 timeBeginPeriod(1) 提升定时粒度。
//!
//! 播放控制（预览窗口对接，2026-08-20；卡片恒默认态 = 播放 / 1x / 无 seek 不受影响）：
//! playing 暂停 = publish 后阻塞轮询（5ms，stop/恢复/seek 唤醒）；rate = delay/rate；
//! seek = 目标帧索引写入 seek_to——GIF/WEBP 无索引只能顺序解码：目标在当前之后 = 快进（不推帧不睡眠
//! 纯解码），目标已过 = 中止本趟重开从头快进；到位后按播放态继续或停在该帧（逐帧步进 = 暂停 + seek±1）。
//!
//! 会话上限 MAX_SESSIONS = 泄漏兜底（正常由前端 autoplayGate anim 池 50 调度 + 预览单实例）；
//! 会话线程 detach（stop 标记退出，不 join）；库切换 stop_all（会话绑定 assets 池路径）。
//! cfg(test) 不起 WS server（tauri GUI 依赖链拖垮测试 exe，踩坑 014），会话逻辑可单测。
//!
//! （5.3 视频会话 / 音频通路已随视频播放回滚 <video> 直读拆除，2026-08-22——帧流纯动图；
//! 文本控制消息 flush/ended/error 与 PCM 音频块同为视频专用协议，一并退役；方案存档见 git 历史，
//! decoder 侧视频/音频引擎随视频播放定线 <video> 于 2026-09-07 删除）

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::services::{anim, thumbnail};

/// 会话上限（前端 anim 池 50 + 预览单实例之上留余量，防泄漏打爆 CPU）
const MAX_SESSIONS: usize = 80;

/// 目标短边 clamp 范围：下限防 0/异常尺寸；上限对齐全尺寸预览档（FULL_PREVIEW_LONG_EDGE），
/// 卡片前端自行 clamp 1024（AnimCanvasLayer），预览按视口×zoom 传（4096 封顶）
const MIN_EDGE: u32 = 16;
const MAX_EDGE: u32 = 4096;

/// seek_to 空标记（无待定位请求）
const NO_SEEK: u64 = u64::MAX;

fn clamp_edge(edge: u32) -> u32 {
  edge.clamp(MIN_EDGE, MAX_EDGE)
}

/// 帧二进制头长度（tag u8 + w u32 + h u32 + idx u32 + pts_ms f64 + seq u64，协议 v2）
#[cfg(not(test))] // 仅 WS server 路径使用（测试构建不起 server，踩坑 014）
pub const FRAME_HEADER: usize = 29;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamStart {
  pub session: u64,
  pub port: u16,
  /// 素材自身帧率（动图恒 null）
  pub fps: Option<f64>,
  /// 时长秒（动图 metadata duration 透传，可能 null）
  pub duration: Option<f64>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub segment: Option<crate::services::semantic::types::SegmentRange>,
}

#[derive(Clone, Default)]
struct Latest {
  w: u32,
  h: u32,
  idx: u32, // 源帧索引（0 起，循环回绕归 0）
  /// 帧时刻（协议 v2 字段，趟内累计 delay）——仅 WS server 序列化读取（cfg(not(test))），
  /// 测试构建只构造不读，豁免 dead_code
  #[cfg_attr(test, allow(dead_code))]
  pts_ms: f64,
  seq: u64,
  rgba: Arc<Vec<u8>>,
}

struct Session {
  stop: AtomicBool,
  /// 目标短边（出帧尺寸热更新：解码循环每帧读取，卡片 ResizeObserver 驱动）
  target: AtomicU32,
  /// 播放态（预览；卡片恒 true）：false = publish 后阻塞轮询
  playing: AtomicBool,
  /// 倍速（f32 bits，预览；卡片恒 1.0）：delay 睡眠 = delay_ms / rate
  rate: AtomicU32,
  /// 待定位目标帧索引（NO_SEEK = 无）：快进 / 回退重开见模块头注释
  seek_to: AtomicU64,
  range: Option<(u32, u32)>,
  slot: Mutex<Latest>,
  tx: tokio::sync::watch::Sender<u64>,
}

impl Session {
  fn ranged(short_edge: u32, range: Option<(u32, u32)>) -> Arc<Self> {
    let (tx, _rx) = tokio::sync::watch::channel(0u64);
    Arc::new(Session {
      stop: AtomicBool::new(false),
      target: AtomicU32::new(clamp_edge(short_edge)),
      playing: AtomicBool::new(true),
      rate: AtomicU32::new(1.0f32.to_bits()),
      seek_to: AtomicU64::new(range.map(|(start, _)| u64::from(start)).unwrap_or(NO_SEEK)),
      range,
      slot: Mutex::new(Latest::default()),
      tx,
    })
  }

  /// seek 唤醒信号是否 pending——pace 阻塞 / 睡眠的唤醒条件
  fn seek_pending(&self) -> bool {
    self.seek_to.load(Ordering::Relaxed) != NO_SEEK
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
  // Windows 定时粒度 15.6ms → 1ms（踩坑 040：deadline pacing 的 sleep 精度由此提升；
  // 进程级设置，随进程退出自动释放）
  #[cfg(windows)]
  unsafe {
    windows::Win32::Media::timeBeginPeriod(1);
  }
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
    serve(listener).await;
  });
}

/// accept 循环（仅 WS server 路径使用——测试构建不起 server，踩坑 014）
#[cfg(not(test))]
async fn serve(listener: tokio::net::TcpListener) {
  while let Ok((stream, _)) = listener.accept().await {
    tokio::spawn(async move {
      if let Err(e) = serve_ws(stream).await {
        log::debug!("帧流 WS 连接结束：{e:?}");
      }
    });
  }
}

/// 单连接推流：先推当前帧（已有内容时新连接立刻有画面），随后跟 watch 通道推新帧
///（latest-only 语义——卡顿丢帧是正确语义）；对端关闭 / 会话消亡即结束
#[cfg(not(test))]
#[allow(clippy::result_large_err)] // accept_hdr_async 握手回调 Err 变体 = tungstenite ErrorResponse，签名既定无法装箱
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

/// 帧（协议 v2）：[tag=0][w u32le][h u32le][idx u32le][pts_ms f64le][seq u64le][rgba...]
#[cfg(not(test))] // 仅 WS server 路径使用
fn frame_bytes(f: &Latest) -> Vec<u8> {
  let mut out = Vec::with_capacity(FRAME_HEADER + f.rgba.len());
  out.push(0u8);
  out.extend_from_slice(&f.w.to_le_bytes());
  out.extend_from_slice(&f.h.to_le_bytes());
  out.extend_from_slice(&f.idx.to_le_bytes());
  out.extend_from_slice(&f.pts_ms.to_le_bytes());
  out.extend_from_slice(&f.seq.to_le_bytes());
  out.extend_from_slice(&f.rgba);
  out
}

/// 开启动图帧流会话：读 metadata 判动图 → 路由解码桥 → 起会话线程。
/// short_edge = 目标短边（卡片显示尺寸，clamp [MIN_EDGE, MAX_EDGE]；不足不放大）。
/// 非动图 / metadata 缺失 / 超上限返回 Err（前端回退静态缩略图）。
pub fn start(assets_dir: &Path, asset_id: &str, short_edge: u32) -> Result<StreamStart, String> {
  let playback = crate::services::semantic::playback::resolve(assets_dir, asset_id)?;
  let segment = playback.segment.clone();
  let bounds = segment.as_ref().map(|range| range.frame_bounds()).transpose()?;
  let meta = playback.media_meta().clone();
  let origin = playback.origin;
  if !anim::is_animated_file(&meta, &origin) {
    return Err("非动图资产".to_string());
  }
  let gs = global();
  {
    let sessions = gs.sessions.lock().unwrap();
    if sessions.len() >= MAX_SESSIONS {
      return Err(format!("帧流会话已达上限 {MAX_SESSIONS}"));
    }
  }
  if !origin.is_file() {
    return Err("origin 文件不存在".to_string());
  }
  let id = gs.next.fetch_add(1, Ordering::Relaxed);
  let sess = Session::ranged(short_edge, bounds);
  let runner = Arc::clone(&sess);
  let ext = meta.ext.to_lowercase();
  std::thread::Builder::new()
    .name(format!("anim-stream-{id}"))
    .spawn(move || run_session(runner, origin, ext))
    .map_err(|e| format!("会话线程启动失败: {e}"))?;
  gs.sessions.lock().unwrap().insert(id, sess);
  Ok(StreamStart {
    session: id,
    port: gs.port.load(Ordering::Relaxed) as u16,
    fps: None,
    duration: segment.as_ref().map(|range| range.duration_secs()).or(meta.duration),
    segment,
  })
}

/// 热更新目标短边（卡片尺寸跟随）：只改 atomic，解码循环下一帧按新尺寸出帧——
/// 不重启解码、不回第一帧（对齐 2026-08-20 用户"随实际大小跟随"要求）。会话不存在静默忽略。
pub fn set_size(session: u64, short_edge: u32) {
  if let Some(sess) = global().sessions.lock().unwrap().get(&session) {
    sess.target.store(clamp_edge(short_edge), Ordering::Relaxed);
  }
}

/// 播放/暂停（预览）：暂停 = publish 后阻塞轮询（stop/seek 可唤醒）。会话不存在静默忽略。
pub fn set_playing(session: u64, playing: bool) {
  if let Some(sess) = global().sessions.lock().unwrap().get(&session) {
    sess.playing.store(playing, Ordering::Relaxed);
  }
}

/// 倍速（预览）：delay 睡眠 = delay_ms / rate；非法值（≤0 / NaN / 无穷）忽略。
pub fn set_rate(session: u64, rate: f32) {
  if !rate.is_finite() || rate <= 0.0 {
    return;
  }
  if let Some(sess) = global().sessions.lock().unwrap().get(&session) {
    sess.rate.store(rate.to_bits(), Ordering::Relaxed);
  }
}

/// 定位到目标帧（动图预览播放条拖动 / 逐帧步进）：GIF/WEBP 无索引只能顺序解码——
/// 目标在当前之后 = 快进（不推帧不睡眠）；已过 = 中止本趟重开从头快进。会话不存在静默忽略。
pub fn seek(session: u64, frame_index: u32) {
  if let Some(sess) = global().sessions.lock().unwrap().get(&session) {
    let index = sess.range.map(|(start, end)| frame_index.clamp(start, end)).unwrap_or(frame_index);
    sess.seek_to.store(index as u64, Ordering::Relaxed);
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
    if let Some((start, _)) = sess.range {
      sess.seek_to.store(u64::from(start), Ordering::Relaxed);
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

/// 每帧推送前的统一闸门（动图两桥共用）：seek 快进（目标在 idx 之后 = 不推不睡纯解码）/ 到位清请求 / 已过回退
enum Gate {
  Deliver, // 正常推帧
  Skip,    // seek 快进中：本帧不推不停
  Abort,   // seek 目标已过：中止本趟（seek_to 保留），外层重开从头快进
}

fn gate(sess: &Arc<Session>, idx: u32) -> Gate {
  if sess.range.is_some_and(|(_, end)| idx > end) {
    return Gate::Abort;
  }
  let t = sess.seek_to.load(Ordering::Relaxed);
  if t == NO_SEEK {
    return Gate::Deliver;
  }
  if (idx as u64) < t {
    return Gate::Skip;
  }
  if (idx as u64) == t {
    sess.seek_to.store(NO_SEEK, Ordering::Relaxed); // 到位：本帧正常推
    return Gate::Deliver;
  }
  Gate::Abort
}

/// publish 后节奏（动图两桥共用）：pause 阻塞轮询（stop / 恢复 / seek 唤醒）+ 累计 deadline 睡眠
/// （next_due += delay/rate，sleep 差值；落后不追赶——重置基准防欠账爆发补帧；
/// seek 待处理时不睡——逐帧步进 / 播放条拖动响应不拖一个帧时长）
fn pace(sess: &Arc<Session>, delay_ms: u32, next_due: &mut Instant) {
  while !sess.playing.load(Ordering::Relaxed) && !sess.stop.load(Ordering::Relaxed) && !sess.seek_pending() {
    std::thread::sleep(Duration::from_millis(5));
  }
  if sess.stop.load(Ordering::Relaxed) || sess.seek_pending() {
    *next_due = Instant::now(); // 中断后基准归零（防恢复 / 重开时追旧账）
    return;
  }
  let rate = f64::from(f32::from_bits(sess.rate.load(Ordering::Relaxed)));
  *next_due += Duration::from_secs_f64((delay_ms as f64 / 1000.0 / rate).max(0.0));
  let now = Instant::now();
  if *next_due > now {
    std::thread::sleep(*next_due - now);
  } else {
    *next_due = now; // 落后不追赶（重置基准防爆发）
  }
}

/// gif 桥（ffmpeg）：单趟解码循环播放；Ok(true)= 自然到底 / seek 回退重开，false = 停止/失败
/// 目标短边由 decode_once 每帧读 sess.target（热更新）；idx = 本趟帧序号（源帧索引）
fn run_ffmpeg_once(sess: &Arc<Session>, origin: &Path) -> bool {
  let mut idx: u32 = 0;
  let mut due = Instant::now();
  let r = crate::adapters::decoder::decode_once(origin, &sess.target, &mut |f| {
    if sess.stop.load(Ordering::Relaxed) {
      return false;
    }
    let i = idx;
    idx += 1;
    match gate(sess, i) {
      Gate::Skip => return true,
      Gate::Abort => return false,
      Gate::Deliver => {}
    }
    publish(sess, f.w, f.h, i, f.pts_ms, f.rgba);
    pace(sess, f.delay_ms, &mut due);
    true
  });
  if sess.stop.load(Ordering::Relaxed) {
    return false;
  }
  if sess.seek_to.load(Ordering::Relaxed) != NO_SEEK {
    return true; // seek 回退中止 → 重开一趟从头快进
  }
  matches!(r, Ok(()))
}

/// webp 桥（image-webp 纯 Rust）：单趟流式逐帧 + 缩放（每帧读 sess.target 热更新），循环播放
/// （decode_frames 回调 false = 主动提前结束返回 Some，stop 与 seek 回退靠 atomic 区分）
fn run_webp_once(sess: &Arc<Session>, origin: &Path) -> bool {
  let mut due = Instant::now();
  let mut cum_pts = 0.0f64; // 趟内累计 delay（协议 v2 pts 字段，动图前端忽略）
  let r = anim::decode_frames(origin, |i, img, delay| {
    if sess.stop.load(Ordering::Relaxed) {
      return false;
    }
    match gate(sess, i) {
      Gate::Skip => {
        cum_pts += delay as f64;
        return true;
      }
      Gate::Abort => return false,
      Gate::Deliver => {}
    }
    let pts_ms = cum_pts;
    cum_pts += delay as f64;
    let (w, h, rgba) = scale_to_thumb(img, sess.target.load(Ordering::Relaxed));
    publish(sess, w, h, i, pts_ms, rgba);
    pace(sess, delay, &mut due);
    true
  });
  if sess.stop.load(Ordering::Relaxed) {
    return false;
  }
  if sess.seek_to.load(Ordering::Relaxed) != NO_SEEK {
    return true;
  }
  r.is_some()
}

/// 真短边等比不足不放大（复用 thumbnail::shrink_to_edge，与缩略图 / gif 桥口径严格一致），输出紧凑 RGBA
fn scale_to_thumb(img: image::RgbaImage, short_edge: u32) -> (u32, u32, Vec<u8>) {
  let out = thumbnail::shrink_to_edge(image::DynamicImage::ImageRgba8(img), short_edge).to_rgba8();
  let (w, h) = out.dimensions();
  (w, h, out.into_raw())
}

fn publish(sess: &Arc<Session>, w: u32, h: u32, idx: u32, pts_ms: f64, rgba: Vec<u8>) {
  let seq = {
    let mut g = sess.slot.lock().unwrap();
    let s = g.seq + 1;
    *g = Latest { w, h, idx, pts_ms, seq: s, rgba: Arc::new(rgba) };
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

  /// 暂停 / 恢复（预览播放控制）：暂停后帧序号冻结，恢复后续播
  #[test]
  fn pause_and_resume() {
    let _g = TEST_LOCK.lock().unwrap();
    let h = make_lib();
    add_gif(&h, "AN-PR");
    let st = start(&h.assets_dir, "AN-PR", 320).unwrap();
    wait_frame(st.session, "出帧");
    set_playing(st.session, false);
    std::thread::sleep(Duration::from_millis(120)); // 等解码线程进入阻塞（当前帧推完）
    let frozen = cur_frame(st.session).unwrap().seq;
    std::thread::sleep(Duration::from_millis(200)); // 50ms/帧：若未暂停必涨
    assert_eq!(cur_frame(st.session).unwrap().seq, frozen, "暂停后帧序号应冻结");
    set_playing(st.session, true);
    wait_seq(st.session, frozen + 1, "恢复后续播");
    stop(st.session);
  }

  /// seek（预览播放条 / 逐帧步进）：暂停态定位推一帧停住；回退目标重开快进
  #[test]
  fn seek_positions_frame() {
    let _g = TEST_LOCK.lock().unwrap();
    let h = make_lib();
    add_gif(&h, "AN-SK");
    let st = start(&h.assets_dir, "AN-SK", 320).unwrap();
    wait_frame(st.session, "出帧");
    set_playing(st.session, false);
    std::thread::sleep(Duration::from_millis(120));
    // 定位到第 2 帧：暂停态推一帧后冻结
    seek(st.session, 2);
    let s0 = cur_frame(st.session).unwrap().seq;
    wait_seq(st.session, s0 + 1, "seek 到位推帧");
    assert_eq!(cur_frame(st.session).unwrap().idx, 2, "seek 后应停在第 2 帧");
    let frozen = cur_frame(st.session).unwrap().seq;
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(cur_frame(st.session).unwrap().seq, frozen, "seek 后暂停态应保持");
    // 回退到第 0 帧（当前 idx=2，目标已过 → 重开快进）
    seek(st.session, 0);
    wait_seq(st.session, frozen + 1, "回退 seek 到位");
    assert_eq!(cur_frame(st.session).unwrap().idx, 0, "回退 seek 应停在第 0 帧");
    stop(st.session);
  }

  /// 倍速（预览 RateControl）：8x 下 400ms 推进帧数显著超 1x 上限（50ms/帧 1x 约 8 帧）
  #[test]
  fn rate_accelerates() {
    let _g = TEST_LOCK.lock().unwrap();
    let h = make_lib();
    add_gif(&h, "AN-RT");
    let st = start(&h.assets_dir, "AN-RT", 320).unwrap();
    wait_frame(st.session, "出帧");
    set_rate(st.session, 8.0);
    let s0 = cur_frame(st.session).unwrap().seq;
    std::thread::sleep(Duration::from_millis(400));
    let gained = cur_frame(st.session).unwrap().seq - s0;
    assert!(gained >= 12, "8x 倍速 400ms 应推进 ≥12 帧（实际 {gained}）");
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

  #[test]
  fn ranged_gate_starts_and_ends_on_original_frame_bounds() {
    let session = Session::ranged(320, Some((1, 2)));
    assert!(matches!(gate(&session, 0), Gate::Skip));
    assert!(matches!(gate(&session, 1), Gate::Deliver));
    assert!(matches!(gate(&session, 2), Gate::Deliver));
    assert!(matches!(gate(&session, 3), Gate::Abort));
  }
}
