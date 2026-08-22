//! 动图 / 视频帧流服务（阶段四·自研解码器：动图卡 + 视频卡播放源 + 预览窗口动图 / 视频播放）。
//! 会话 = 解码线程（循环播 origin 源文件、按源帧 delay 节奏）+ 帧槽（恒持最新帧）；
//! WS server（127.0.0.1 动态端口）按会话推流（协议 v2，2026-08-22 5.3）：
//! - 视频帧（二进制 tag 0）：`[0][w u32le][h u32le][idx u32le][pts_ms f64le][seq u64le][rgba]`
//!   （idx = 源帧索引 0 起、循环回绕归 0——预览播放条帧号 / 进度用；pts_ms = 帧时刻，音画同步基准；
//!   seq 单调递增推送序号）
//! - 音频块（二进制 tag 1，视频有音轨时）：`[1][pts_ms f64le][frames u32le][pcm f32le × frames × 2ch]`
//!   （48kHz 立体声 f32，decoder::AUDIO_RATE / AUDIO_CHUNK；走 broadcast 通道——音频不可丢，
//!   watch latest-only 语义会丢块出爆音；视频帧仍走 watch，卡顿时丢帧是正确语义）
//! - 文本控制消息：`"flush"`（seek 重推前发——前端清音频队列与待渲帧）/ `"ended"`（loop=false 到底）
//!
//! 路由：gif → adapters/decoder（ffmpeg-next）；webp → services/anim（image-webp 纯 Rust——
//! lgpl 构建无 libwebp decoder，踩坑 039）；视频 → adapters/decoder decode_video_once（ffmpeg-next，
//! 音轨同趟解：resampling 48k 立体声 f32）。缩放 = 真短边等比不足不放大（thumbnail::shrink_to_edge
//! 同口径）；目标短边 = 调用方显示尺寸（卡片 = 卡片实测短边 clamp 1024；预览 = 视口×zoom clamp 4096），
//! 会话级 AtomicU32 热更新（set_size 不重启解码、不回第一帧）。
//! 节奏 = 累计 deadline（next_due += delay/rate，sleep 差值；落后不追赶防爆发）——消 Windows sleep
//! 15.6ms 粒度漂移（踩坑 040，音画同步前提）；init 时 timeBeginPeriod(1) 提升定时粒度。
//!
//! 播放控制（预览窗口对接，2026-08-20；卡片恒默认态 = 播放 / 1x / 无 seek 不受影响）：
//! playing 暂停 = publish 后阻塞轮询（5ms，stop/恢复/seek 唤醒）；rate = delay/rate；
//! seek 分两类——动图（GIF/WEBP 无索引只能顺序解码）：帧索引 seek_to，目标在当前之后 = 快进
//! （不推帧不睡眠纯解码），已过 = 中止本趟重开从头快进；视频（5.3）：秒 seek_secs——
//! **持久解码器就地 seek**（ictx.seek + 清缓冲，不重开文件；相邻前跳 ≤2 帧续解免 seek = 逐帧步进即时；
//! 循环重开 = 就地 seek(0)；丢帧阶段逐帧可查新 seek 打断）。
//! 视频 loop=false 自然到底：发 ended + 转暂停态停最后帧（seek / 重新播放唤醒）。
//!
//! 会话上限 MAX_SESSIONS = 泄漏兜底（正常由前端 autoplayGate 动图池 50 + 视频池 16 + 预览单实例）；
//! 会话线程 detach（stop 标记退出，不 join）；库切换 stop_all（会话绑定 assets 池路径）。
//! cfg(test) 不起 WS server（tauri GUI 依赖链拖垮测试 exe，踩坑 014），会话逻辑可单测。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::core::json_io::read_json;
use crate::core::models::AssetMeta;
use crate::core::paths;
use crate::services::{anim, media, thumbnail};

/// 会话上限（前端动图池 50 + 视频池 16 + 预览单实例之上留余量，防泄漏打爆 CPU）
const MAX_SESSIONS: usize = 80;

/// 目标短边 clamp 范围：下限防 0/异常尺寸；上限对齐全尺寸预览档（FULL_PREVIEW_LONG_EDGE），
/// 卡片前端自行 clamp 1024（AnimCanvasLayer），预览按视口×zoom 传（4096 封顶）
const MIN_EDGE: u32 = 16;
const MAX_EDGE: u32 = 4096;

/// seek_to / seek_secs 空标记（无待定位请求）
const NO_SEEK: u64 = u64::MAX;

fn clamp_edge(edge: u32) -> u32 {
  edge.clamp(MIN_EDGE, MAX_EDGE)
}

/// 帧二进制头长度（tag u8 + w u32 + h u32 + idx u32 + pts_ms f64 + seq u64，协议 v2）
pub const FRAME_HEADER: usize = 29;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamStart {
  pub session: u64,
  pub port: u16,
  /// 有音轨（视频；动图恒 false）——前端决定是否出音量钮 / 建音频调度
  pub has_audio: bool,
  /// 素材自身帧率（视频；动图为 null）
  pub fps: Option<f64>,
  /// 时长秒（视频；动图 metadata duration 透传，可能 null）
  pub duration: Option<f64>,
}

#[derive(Clone, Default)]
struct Latest {
  w: u32,
  h: u32,
  idx: u32, // 源帧索引（0 起，循环回绕归 0）
  pts_ms: f64, // 帧时刻（协议 v2 音画同步字段）
  seq: u64,
  rgba: Arc<Vec<u8>>,
}

/// 音频 / 控制消息（broadcast 通道；pcm Arc 共享防拷贝）
///（字段仅 WS server 路径读取——cfg(not(test))；测试构建只构造不读，豁免 dead_code）
#[cfg_attr(test, allow(dead_code))]
#[derive(Clone)]
enum AudioMsg {
  Chunk { pts_ms: f64, pcm: Arc<Vec<f32>> },
  /// seek 重推前：前端清音频队列与待渲帧
  Flush,
  /// loop=false 自然到底：前端停播放态（末帧已由视频通道推过）
  Ended,
  /// 会话线程死亡（解码错误等）：前端降级静态封面 / 报错——防"无声无息冻结"
  Error,
}

struct Session {
  /// 会话 id（死亡路径自清会话表用）
  id: u64,
  stop: AtomicBool,
  /// 目标短边（出帧尺寸热更新：解码循环每帧读取，卡片 ResizeObserver 驱动）
  target: AtomicU32,
  /// 播放态（预览；卡片恒 true）：false = publish 后阻塞轮询
  playing: AtomicBool,
  /// 倍速（f32 bits，预览；卡片恒 1.0）：delay 睡眠 = delay_ms / rate
  rate: AtomicU32,
  /// 待定位目标帧索引（动图；NO_SEEK = 无）：快进 / 回退重开见模块头注释
  seek_to: AtomicU64,
  /// 待定位目标秒（视频；f64 bits，NO_SEEK = 无）：任何方向中止本趟重开 + ffmpeg 真 seek
  seek_secs: AtomicU64,
  /// 循环开关（视频播放条；动图恒 true）：false = 自然到底发 ended 转暂停
  loop_enabled: AtomicBool,
  /// 有音轨（视频会话启动时定；动图 false）
  has_audio: bool,
  /// 素材自身帧率（视频 idx 推导 round(pts×fps) 与 0 delay 兜底；动图 0 不用）
  fps: f64,
  /// 视频会话首趟起播点（秒；卡片自动播放 = 封面帧 t）
  start_secs: f64,
  slot: Mutex<Latest>,
  tx: tokio::sync::watch::Sender<u64>,
  /// 音频 / 控制广播（容量 256 块 ≈ 5.5s 音频；Lag = 极端卡顿跳最新）
  atx: tokio::sync::broadcast::Sender<AudioMsg>,
}

impl Session {
  fn new(id: u64, short_edge: u32, has_audio: bool, fps: f64, start_secs: f64, loop_enabled: bool) -> Arc<Self> {
    let (tx, _rx) = tokio::sync::watch::channel(0u64);
    let (atx, _arx) = tokio::sync::broadcast::channel(256);
    Arc::new(Session {
      id,
      stop: AtomicBool::new(false),
      target: AtomicU32::new(clamp_edge(short_edge)),
      playing: AtomicBool::new(true),
      rate: AtomicU32::new(1.0f32.to_bits()),
      seek_to: AtomicU64::new(NO_SEEK),
      seek_secs: AtomicU64::new(NO_SEEK),
      loop_enabled: AtomicBool::new(loop_enabled),
      has_audio,
      fps,
      start_secs,
      slot: Mutex::new(Latest::default()),
      tx,
      atx,
    })
  }

  /// seek 唤醒信号是否 pending（视频查 seek_secs，动图查 seek_to）——pace 阻塞 / 睡眠的唤醒条件
  fn seek_pending(&self) -> bool {
    self.seek_to.load(Ordering::Relaxed) != NO_SEEK || self.seek_secs.load(Ordering::Relaxed) != NO_SEEK
  }

  /// 会话线程死亡收尾（解码错误等）：通知前端降级 + 自清会话表（防"无声无息冻结" + 泄漏）
  fn die(&self) {
    let _ = self.atx.send(AudioMsg::Error);
    self.stop.store(true, Ordering::Relaxed);
    global().sessions.lock().unwrap().remove(&self.id);
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
  // Windows 定时粒度 15.6ms → 1ms（踩坑 040：30fps 视频帧节奏被粒度顶到 25.6fps；
  // deadline pacing 的 sleep 精度由此提升；进程级设置，随进程退出自动释放）
  #[cfg(windows)]
  unsafe {
    windows::Win32::Media::timeBeginPeriod(1);
  }
  let gs = global();
  tauri::async_runtime::spawn(async move {
    let listener = match tokio::net::TcpListener::bind("127.0.0.1:0").await {
      Ok(l) => l,
      Err(e) => {
        log::warn!("帧流 WS server 启动失败（动图/视频卡回退静态图）：{e}");
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

/// accept 循环（init 与测试 harness 共用；无 tauri 依赖——serve_ws 全家不触 AppHandle，
/// 测试构建可编译可用，踩坑 014 的门控只需罩住 init 的 AppHandle 签名）
async fn serve(listener: tokio::net::TcpListener) {
  while let Ok((stream, _)) = listener.accept().await {
    tokio::spawn(async move {
      if let Err(e) = serve_ws(stream).await {
        log::debug!("帧流 WS 连接结束：{e:?}");
      }
    });
  }
}

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
  let mut arx = sess.atx.subscribe();
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
      // 音频 / 控制优先于视频帧（biased）：视频帧 MB 级写耗时会让音频块排队延迟
      //（用户实机"音画错位"诱因之一）；音频 16KB 快写先走，视频帧 watch 语义本就丢得起
      biased;
      amsg = arx.recv() => {
        match amsg {
          Ok(AudioMsg::Chunk { pts_ms, pcm }) => {
            write.send(tokio_tungstenite::tungstenite::Message::Binary(audio_bytes(pts_ms, &pcm).into())).await?;
          }
          Ok(AudioMsg::Flush) => {
            write.send(tokio_tungstenite::tungstenite::Message::Text("flush".into())).await?;
          }
          Ok(AudioMsg::Ended) => {
            write.send(tokio_tungstenite::tungstenite::Message::Text("ended".into())).await?;
          }
          Ok(AudioMsg::Error) => {
            write.send(tokio_tungstenite::tungstenite::Message::Text("error".into())).await?;
          }
          Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
            // 极端卡顿丢消息：控制消息（flush/ended）丢失会让前端状态卡死（如实机冻结案：
            // seek 的 flush 被丢 → 前端永久丢帧）——合成 flush 强制前端重同步（音频丢块本就该丢）
            let _ = write.send(tokio_tungstenite::tungstenite::Message::Text("flush".into())).await;
            continue;
          }
          Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
        }
      }
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

/// 视频帧（协议 v2）：[tag=0][w u32le][h u32le][idx u32le][pts_ms f64le][seq u64le][rgba...]
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

/// 音频块（协议 v2）：[tag=1][pts_ms f64le][frames u32le][pcm f32le × frames × 2ch]
/// （48kHz 立体声，frames = 每声道样本数）
fn audio_bytes(pts_ms: f64, pcm: &[f32]) -> Vec<u8> {
  let frames = (pcm.len() / 2) as u32;
  let mut out = Vec::with_capacity(13 + pcm.len() * 4);
  out.push(1u8);
  out.extend_from_slice(&pts_ms.to_le_bytes());
  out.extend_from_slice(&frames.to_le_bytes());
  for s in pcm {
    out.extend_from_slice(&s.to_le_bytes());
  }
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
  let id = gs.next.fetch_add(1, Ordering::Relaxed);
  let sess = Session::new(id, short_edge, false, 0.0, 0.0, true);
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
    has_audio: false,
    fps: None,
    duration: meta.duration,
  })
}

/// 开启视频帧流会话（5.3）：读 metadata 判视频 → 起视频桥（ffmpeg-next AV 同趟解码）。
/// fps 必需（idx 推导 / 0 delay 兜底；缺失 = Err，命令层先做自愈探测）；
/// start_secs = 首趟起播点（卡片自动播放 = 封面帧 t；预览 = 0）；
/// loop_enabled = 循环（卡片恒 true；预览跟随播放条循环钮，set_loop 热改）。
/// has_audio = probe 音轨存在性（有则同趟解码推送；音量 / 静音纯前端 GainNode）。
pub fn start_video(
  assets_dir: &Path,
  asset_id: &str,
  short_edge: u32,
  start_secs: f64,
  loop_enabled: bool,
) -> Result<StreamStart, String> {
  let meta: AssetMeta = read_json(&paths::info_dir(assets_dir, asset_id).join("metadata.json"))
    .map_err(|e| format!("metadata 读取失败: {e}"))?;
  let origin = paths::origin_file(assets_dir, asset_id, &meta.origin_name);
  if !origin.is_file() {
    return Err("origin 文件不存在".to_string());
  }
  if !media::is_video(&origin) {
    return Err("非视频资产".to_string());
  }
  let fps = meta.fps.ok_or_else(|| "视频帧率未知".to_string())?;
  let gs = global();
  {
    let sessions = gs.sessions.lock().unwrap();
    if sessions.len() >= MAX_SESSIONS {
      return Err(format!("帧流会话已达上限 {MAX_SESSIONS}"));
    }
  }
  // 音轨存在性探测（ffprobe sidecar；失败 / 无音轨 = false，画面照常）
  let has_audio = crate::adapters::ffmpeg::probe_json(&origin)
    .and_then(|j| j.get("streams")?.as_array().cloned())
    .map(|streams| {
      streams
        .iter()
        .any(|s| s.get("codec_type").and_then(|t| t.as_str()) == Some("audio"))
    })
    .unwrap_or(false);
  let id = gs.next.fetch_add(1, Ordering::Relaxed);
  let sess = Session::new(id, short_edge, has_audio, fps, start_secs.max(0.0), loop_enabled);
  let runner = Arc::clone(&sess);
  std::thread::Builder::new()
    .name(format!("video-stream-{id}"))
    .spawn(move || run_video_session(runner, origin))
    .map_err(|e| format!("会话线程启动失败: {e}"))?;
  gs.sessions.lock().unwrap().insert(id, sess);
  Ok(StreamStart {
    session: id,
    port: gs.port.load(Ordering::Relaxed) as u16,
    has_audio,
    fps: Some(fps),
    duration: meta.duration,
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
    sess.seek_to.store(frame_index as u64, Ordering::Relaxed);
  }
}

/// 定位到目标秒（视频预览播放条拖动 / 逐帧步进 / 卡片 scrub / 范围循环回起点）：
/// 任何方向都中止本趟重开 + ffmpeg 真 seek（关键帧落点 + 前向丢帧）。会话不存在静默忽略。
pub fn seek_video(session: u64, secs: f64) {
  if !secs.is_finite() || secs < 0.0 {
    return;
  }
  if let Some(sess) = global().sessions.lock().unwrap().get(&session) {
    sess.seek_secs.store(secs.to_bits(), Ordering::Relaxed);
  }
}

/// 循环开关（视频播放条循环钮）：false = 自然到底发 ended 转暂停。会话不存在静默忽略。
pub fn set_loop(session: u64, loop_enabled: bool) {
  if let Some(sess) = global().sessions.lock().unwrap().get(&session) {
    sess.loop_enabled.store(loop_enabled, Ordering::Relaxed);
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

/// 视频会话（5.3；同日性能返修：持久解码器 + 就地 seek——用户实机报 seek/逐帧 ~1s、循环接缝卡顿）：
/// VideoDecoder 会话全程一份——seek = ictx.seek + 清缓冲（**不重开文件 / 解码器**，旧 reopen-per-seek
/// 每次 = format::input + find_stream_info + 解码器初始化 + 长 GOP 前向丢帧，~1s 延迟根源）；
/// **相邻前跳 ≤2 帧 = 续解免 seek**（逐帧步进即时：只抬丢帧阈值继续解）；**循环重开 = 就地 seek(0)**；
/// 丢帧阶段逐帧可查新 seek（Pull::Interrupted 即回顶处理，scrub 拖动不被旧目标拖住）。
fn run_video_session(sess: Arc<Session>, origin: PathBuf) {
  use crate::adapters::decoder::{Pull, VideoDecoder};
  let mut dec = match VideoDecoder::open(&origin, sess.has_audio) {
    Ok(d) => d,
    Err(e) => {
      log::warn!("视频解码会话打开失败（{}）：{e}", origin.display());
      sess.die();
      return;
    }
  };
  if sess.start_secs > 0.0 && dec.seek(sess.start_secs).is_err() {
    sess.die();
    return;
  }
  let fps = sess.fps;
  let frame_ms = if fps > 0.0 { 1000.0 / fps } else { 40.0 };
  // 高帧率发布节流（2026-08-22 实机返修）：源 >60fps 时隔帧发布（120fps 录屏推流是 2 倍无效负载——
  // 前端渲染 60 封顶；WS 大帧 × 120 会把 webview 主线程压垮成"假冻结"）
  let throttle = if fps > 60.0 { (fps / 60.0).round() as u32 } else { 1 };
  let mut seen: u32 = 0;
  let mut due = Instant::now();
  loop {
    if sess.stop.load(Ordering::Relaxed) {
      return;
    }
    // seek 处理（就地）：先发 flush（前端清音频队列与待渲帧）；相邻前跳续解，其余真 seek
    let pending = sess.seek_secs.swap(NO_SEEK, Ordering::Relaxed);
    if pending != NO_SEEK {
      let t = f64::from_bits(pending);
      let _ = sess.atx.send(AudioMsg::Flush);
      let pos_s = dec.pos_pts_ms() / 1000.0;
      if !dec.at_eof() && t > pos_s && t - pos_s <= 2.0 * frame_ms / 1000.0 {
        dec.continue_to(t); // 相邻前跳（逐帧步进）：续解免关键帧往返
      } else if dec.seek(t).is_err() {
        return;
      }
      due = Instant::now();
    }
    let interrupted = || sess.seek_pending() || sess.stop.load(Ordering::Relaxed);
    match dec.read_frame(&sess.target, &interrupted) {
      Ok(Pull::Frame(f)) => {
        if sess.stop.load(Ordering::Relaxed) {
          return;
        }
        if sess.seek_pending() {
          continue; // 新 seek 到达：不推中间帧，回顶立即处理
        }
        seen += 1;
        let idx = (f.pts_ms / 1000.0 * fps).round() as u32;
        let delay = if f.delay_ms > 0 { f.delay_ms } else { frame_ms.max(1.0) as u32 };
        let paused = !sess.playing.load(Ordering::Relaxed);
        // 高帧率节流：非发布帧照走 pace（节奏按源帧率，输出 ≤60fps）；暂停 / 逐帧定位帧恒发布
        if throttle > 1 && !paused && seen % throttle != 1 {
          pace(&sess, delay, &mut due);
          continue;
        }
        publish(&sess, f.w, f.h, idx, f.pts_ms, f.rgba);
        pace(&sess, delay, &mut due);
      }
      Ok(Pull::Audio(c)) => {
        if sess.stop.load(Ordering::Relaxed) {
          return;
        }
        if sess.seek_pending() {
          continue;
        }
        let _ = sess.atx.send(AudioMsg::Chunk { pts_ms: c.pts_ms, pcm: Arc::new(c.pcm) });
      }
      Ok(Pull::Interrupted) => continue, // 丢帧被新 seek 打断：回顶立即处理
      Ok(Pull::Eof) => {
        if sess.stop.load(Ordering::Relaxed) {
          return;
        }
        if sess.seek_pending() {
          continue;
        }
        if sess.loop_enabled.load(Ordering::Relaxed) {
          // 循环重开 = 就地 seek(0)（不重开文件，消除循环接缝卡顿）
          if dec.seek(0.0).is_err() {
            sess.die();
            return;
          }
          due = Instant::now();
          continue;
        }
        // loop=false 到底：发 ended + 转暂停停末帧（播放键语义 = 前端 seek(0)+play，由 seek 唤醒重开）
        let _ = sess.atx.send(AudioMsg::Ended);
        sess.playing.store(false, Ordering::Relaxed);
        while !sess.stop.load(Ordering::Relaxed) && sess.seek_secs.load(Ordering::Relaxed) == NO_SEEK {
          std::thread::sleep(Duration::from_millis(5));
        }
      }
      Err(e) => {
        log::warn!("视频解码会话失败（{}）：{e}", origin.display());
        sess.die();
        return;
      }
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

/// publish 后节奏（三桥共用）：pause 阻塞轮询（stop / 恢复 / seek 唤醒）+ 累计 deadline 睡眠
/// （next_due += delay/rate，sleep 差值；落后不追赶——重置基准防欠账爆发补帧，音画同步以音频连续为准；
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

  // —— 视频会话（5.3）：sample.mp4 = mpeg4 无音轨 10fps 2s ——

  /// 真后端帧流 harness（5.3 实机级复现 / CDP 探针 cdp-video-real-probe 联调，手动跑）：
  /// ANIKIT_HARNESS_VIDEO=<视频> ANIKIT_HARNESS_STATE=<状态文件> ANIKIT_HARNESS_CMD=<命令文件>
  /// [ANIKIT_HARNESS_EDGE=短边，缺省 640]
  /// cargo test --lib -- --ignored --exact services::frame_stream::tests::video_stream_harness --nocapture
  ///
  /// 起真 WS server（serve 全家测试构建可编译——不触 AppHandle，踩坑 014 门控只罩 init）+
  /// 视频入库临时库 + 真会话；状态文件写 {session, port, hasAudio, fps, duration}；
  /// 命令文件 JSON 行轮询执行（{"seek":秒}/{"playing":bool}/{"rate":x}/{"loop":bool}/{"size":px}/{"stop":true}）；
  /// 状态文件删除 = 退出信号；超时 10 分钟自灭防孤儿。
  #[test]
  #[ignore]
  fn video_stream_harness() {
    let video = std::env::var("ANIKIT_HARNESS_VIDEO").expect("ANIKIT_HARNESS_VIDEO 未设");
    let state_file = std::env::var("ANIKIT_HARNESS_STATE").expect("ANIKIT_HARNESS_STATE 未设");
    let cmd_file = std::env::var("ANIKIT_HARNESS_CMD").expect("ANIKIT_HARNESS_CMD 未设");
    let edge: u32 = std::env::var("ANIKIT_HARNESS_EDGE").ok().and_then(|s| s.parse().ok()).unwrap_or(640);
    let _g = TEST_LOCK.lock().unwrap();
    let h = make_lib();
    let pf = PendingFile { path: PathBuf::from(&video), chain: Vec::new() };
    import::build_asset_with_id(&h.assets_dir, &pf, "sha256:h", "AN-H1", Source::Copy, None)
      .expect("harness 视频入库失败");
    // WS server 后台线程（测试进程无 tauri 运行时——自建 current_thread runtime 常驻）
    let (port_tx, port_rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
      .name("harness-ws".to_string())
      .spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(async move {
          let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
          port_tx.send(listener.local_addr().unwrap().port()).unwrap();
          serve(listener).await;
        });
      })
      .unwrap();
    let port = port_rx.recv().unwrap();
    let st = start_video(&h.assets_dir, "AN-H1", edge, 0.0, true).unwrap();
    std::fs::write(
      &state_file,
      serde_json::json!({
        "session": st.session, "port": port, "hasAudio": st.has_audio,
        "fps": st.fps, "duration": st.duration,
      })
      .to_string(),
    )
    .unwrap();
    eprintln!("[harness] 会话 {} 端口 {port} 已就绪（{video}）", st.session);
    let deadline = Instant::now() + Duration::from_secs(600);
    loop {
      if !Path::new(&state_file).is_file() {
        break; // 探针删文件 = 退出信号
      }
      assert!(Instant::now() < deadline, "harness 10 分钟超时自灭");
      // 命令文件：读完全部行后清空（探针侧追加写）
      if let Ok(body) = std::fs::read_to_string(&cmd_file) {
        if !body.trim().is_empty() {
          std::fs::write(&cmd_file, "").unwrap();
          for line in body.lines() {
            let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else { continue };
            if let Some(s) = v.get("seek").and_then(|x| x.as_f64()) {
              seek_video(st.session, s);
            } else if let Some(p) = v.get("playing").and_then(|x| x.as_bool()) {
              set_playing(st.session, p);
            } else if let Some(r) = v.get("rate").and_then(|x| x.as_f64()) {
              set_rate(st.session, r as f32);
            } else if let Some(l) = v.get("loop").and_then(|x| x.as_bool()) {
              set_loop(st.session, l);
            } else if let Some(s) = v.get("size").and_then(|x| x.as_u64()) {
              set_size(st.session, s as u32);
            } else if v.get("stop").and_then(|x| x.as_bool()) == Some(true) {
              stop(st.session);
            }
            eprintln!("[harness] 执行命令: {line}");
          }
        }
      }
      std::thread::sleep(Duration::from_millis(50));
    }
    stop(st.session);
    eprintln!("[harness] 退出");
  }

  /// 视频资产入库；fps 探测依赖 ffprobe sidecar（缺失返回 false = 跳过本用例）
  fn add_video(h: &Harness, asset_id: &str) -> bool {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/test-assets/sample.mp4");
    let pf = PendingFile { path: src, chain: Vec::new() };
    import::build_asset_with_id(&h.assets_dir, &pf, "sha256:t", asset_id, Source::Copy, None).unwrap();
    let meta: AssetMeta =
      read_json(&paths::info_dir(&h.assets_dir, asset_id).join("metadata.json")).unwrap();
    meta.fps.is_some()
  }

  /// 真实录屏压测（2026-08-22 用户实机"预览数帧后冻结"复现）：4K 120fps h264 + AAC 38MB
  /// （assets/video 内用户提供样本）——持续出帧速率 / seek 响应 / seek 后续播。
  /// 复现关键差异 vs sample.mp4：有音轨（AV 交错泵）、120fps（8ms 节奏）、4K 大帧缩放。
  #[test]
  fn video_session_real_recording_progression() {
    let _g = TEST_LOCK.lock().unwrap();
    let h = make_lib();
    let src = Path::new(env!("CARGO_MANIFEST_DIR"))
      .join("../assets/video/Desktop 2026.08.22 - 17.32.24.02_1.mp4");
    if !src.is_file() {
      eprintln!("跳过：assets/video 录屏样本缺失");
      return;
    }
    let pf = PendingFile { path: src, chain: Vec::new() };
    import::build_asset_with_id(&h.assets_dir, &pf, "sha256:t", "AN-R1", Source::Copy, None).unwrap();
    let meta: AssetMeta =
      read_json(&paths::info_dir(&h.assets_dir, "AN-R1").join("metadata.json")).unwrap();
    let Some(fps) = meta.fps else {
      eprintln!("跳过：ffprobe sidecar 缺失（fps 探测失败）");
      return;
    };
    eprintln!("[压测] fps={fps} duration={:?}", meta.duration);
    let st = start_video(&h.assets_dir, "AN-R1", 320, 0.0, true).unwrap();
    assert!(st.has_audio, "录屏应有音轨（AAC）");
    wait_frame(st.session, "首帧");
    // 3 秒持续出帧采样（每 200ms 一记）
    let mut samples = Vec::new();
    for _ in 0..15 {
      std::thread::sleep(Duration::from_millis(200));
      let f = cur_frame(st.session).unwrap();
      samples.push((f.idx, f.pts_ms));
    }
    eprintln!("[压测] 3s 采样: {:?}", samples);
    let advanced = samples.last().unwrap().0 - samples.first().unwrap().0;
    assert!(advanced >= 30, "3s 应推进 ≥30 帧（120fps 实际 {}）", advanced);
    // seek(10s) 响应延迟 + 落点
    let t0 = Instant::now();
    seek_video(st.session, 10.0);
    let deadline = Instant::now() + Duration::from_secs(5);
    let landed;
    loop {
      let f = cur_frame(st.session).unwrap();
      if f.pts_ms >= 9500.0 {
        landed = (t0.elapsed(), f.pts_ms);
        break;
      }
      assert!(Instant::now() < deadline, "seek 10s 落点超时（当前 {}ms）", f.pts_ms);
      std::thread::sleep(Duration::from_millis(10));
    }
    eprintln!("[压测] seek(10s) 到位: {:?} → {}ms", landed.0, landed.1);
    assert!(landed.0 < Duration::from_secs(2), "seek 到位应 <2s（实际 {:?}）", landed.0);
    // seek 后续播（不冻结）：再采 1s
    let (i0, _) = {
      let f = cur_frame(st.session).unwrap();
      (f.idx, f.pts_ms)
    };
    std::thread::sleep(Duration::from_secs(1));
    let i1 = cur_frame(st.session).unwrap().idx;
    eprintln!("[压测] seek 后续播 1s: {i0} → {i1}");
    assert!(i1 > i0, "seek 后应继续播放（{i0} → {i1}）——冻结复现");
    stop(st.session);
  }

  /// 视频会话：出帧 / idx·pts 递增 / has_audio=false（无音轨）/ fps 透传；非视频资产拒绝
  #[test]
  fn video_session_streams() {
    let _g = TEST_LOCK.lock().unwrap();
    let h = make_lib();
    if !add_video(&h, "AN-V1") {
      eprintln!("跳过：ffprobe sidecar 缺失（fps 探测失败）");
      return;
    }
    let st = start_video(&h.assets_dir, "AN-V1", 64, 0.0, true).unwrap();
    assert_eq!(st.fps, Some(10.0));
    assert!(!st.has_audio, "sample.mp4 无音轨");
    assert_eq!(st.duration, Some(2.0));
    wait_frame(st.session, "视频会话出帧");
    let (i0, p0) = {
      let f = cur_frame(st.session).unwrap();
      (f.idx, f.pts_ms)
    };
    std::thread::sleep(Duration::from_millis(400));
    let (i1, p1) = {
      let f = cur_frame(st.session).unwrap();
      (f.idx, f.pts_ms)
    };
    assert!(i1 > i0, "idx 应推进（{i0} → {i1}）");
    assert!(p1 > p0, "pts 应推进（{p0} → {p1}）");
    stop(st.session);
    // 非视频资产 / 不存在资产：Err 不起会话
    add_gif(&h, "AN-VG");
    assert!(start_video(&h.assets_dir, "AN-VG", 64, 0.0, true).is_err());
    assert!(start_video(&h.assets_dir, "AN-不存在", 64, 0.0, true).is_err());
  }

  /// 视频 seek：任意方向真定位——前进 1.5s / 回退 0.5s 落点精确（10fps 帧距 100ms，容差 ±1 帧）
  #[test]
  fn video_seek_positions() {
    let _g = TEST_LOCK.lock().unwrap();
    let h = make_lib();
    if !add_video(&h, "AN-V2") {
      eprintln!("跳过：ffprobe sidecar 缺失（fps 探测失败）");
      return;
    }
    let st = start_video(&h.assets_dir, "AN-V2", 64, 0.0, true).unwrap();
    wait_frame(st.session, "出帧");
    set_playing(st.session, false); // 暂停态 seek = 逐帧定位语义（到位推一帧停住）
    std::thread::sleep(Duration::from_millis(150)); // 等解码线程进入暂停阻塞
    // 前进 1.5s
    seek_video(st.session, 1.5);
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
      let f = cur_frame(st.session).unwrap();
      if (f.pts_ms - 1500.0).abs() <= 100.0 {
        break;
      }
      assert!(Instant::now() < deadline, "seek 1.5s 落点超时（当前 {}ms）", f.pts_ms);
      std::thread::sleep(Duration::from_millis(10));
    }
    // 回退 0.5s（真 seek 重开，非顺序快进）
    seek_video(st.session, 0.5);
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
      let f = cur_frame(st.session).unwrap();
      if (f.pts_ms - 500.0).abs() <= 100.0 {
        break;
      }
      assert!(Instant::now() < deadline, "回退 0.5s 落点超时（当前 {}ms）", f.pts_ms);
      std::thread::sleep(Duration::from_millis(10));
    }
    // 暂停保持（定位后不续播）
    let frozen = cur_frame(st.session).unwrap().seq;
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(cur_frame(st.session).unwrap().seq, frozen, "暂停态 seek 到位后应停住");
    stop(st.session);
  }

  /// loop=false 自然到底：发 ended 语义 = 会话转暂停停末帧；seek(0) 唤醒重开
  #[test]
  fn video_no_loop_ends() {
    let _g = TEST_LOCK.lock().unwrap();
    let h = make_lib();
    if !add_video(&h, "AN-V3") {
      eprintln!("跳过：ffprobe sidecar 缺失（fps 探测失败）");
      return;
    }
    let st = start_video(&h.assets_dir, "AN-V3", 64, 1.8, false).unwrap(); // 从 1.8s 起 → 0.2s 到底
    wait_frame(st.session, "出帧");
    // 等到底转暂停（2s 片长 + 余量）
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
      let playing = global()
        .sessions
        .lock()
        .unwrap()
        .get(&st.session)
        .map(|s| s.playing.load(Ordering::Relaxed));
      if playing == Some(false) {
        break;
      }
      assert!(Instant::now() < deadline, "loop=false 到底应转暂停");
      std::thread::sleep(Duration::from_millis(20));
    }
    let last = cur_frame(st.session).unwrap();
    assert!(last.pts_ms >= 1800.0, "应停在末帧区域（{}ms）", last.pts_ms);
    // seek(0) 唤醒重开（播放键语义）；set_loop 热开循环 = 重开后循环播放（顺带覆盖 set_loop 调用）
    set_loop(st.session, true);
    seek_video(st.session, 0.0);
    set_playing(st.session, true);
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
      let f = cur_frame(st.session).unwrap();
      if f.pts_ms < 500.0 && f.seq > last.seq {
        break;
      }
      assert!(Instant::now() < deadline, "seek(0) 后应从头重推（当前 {}ms）", f.pts_ms);
      std::thread::sleep(Duration::from_millis(10));
    }
    stop(st.session);
  }
}
