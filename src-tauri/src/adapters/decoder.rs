//! ffmpeg-next 解码适配（阶段四·自研解码器）：GIF / 视频 → 逐帧 RGBA + 帧时长。
//! 动图 webp 不归此（BtbN lgpl 构建无 libwebp decoder，踩坑 039）——走 services/anim（image-webp）。
//! 链路口径与 spike/decode-proto 一致（已实测：GIF 50 路 3110fps / H264 50 路 7740fps @320p）。
//!
//! 5.3 视频扩展（decode_video_once）：
//! - 真 seek：`start_secs > 0` 时 `ictx.seek`（AV_TIME_BASE 微秒，关键帧落点）+ 前向丢帧至 pts 到位，
//!   替代帧流动图桥的"中止重开顺序快进"（分钟级视频顺序解码不可行）；
//! - pts：视频帧 pts_ms 来自 best_effort_timestamp（idx 由调用方按 fps 推导，pts 才是音画同步基准）；
//! - 音轨（opts.audio）：best Audio 流 → resampling::Context 重采样 48kHz 立体声 f32 packed，
//!   攒 CHUNK 帧/块回调（pts_ms = 块首样本时刻）；无音轨 / 解码失败静默跳过（画面不受影响）。

use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};

use ffmpeg_next as ffmpeg;

/// 音频输出采样率（Hz，立体声 f32 packed；前端 WebAudio AudioBuffer 按此率创建，播放自动重采样）
pub const AUDIO_RATE: u32 = 48000;
/// 音频块帧数（每声道）：1024 帧 ≈ 21.3ms——调度平滑与推送频率的平衡点
pub const AUDIO_CHUNK: usize = 1024;

/// 一帧解码产物（RGBA 紧凑字节，短边等比缩放后）
pub struct DecodedFrame {
  pub w: u32,
  pub h: u32,
  pub delay_ms: u32, // 本帧展示时长（动图逐帧 delay；视频 = 帧距）
  /// 本帧时刻（ms，趟内累计 delay——动图循环重开归零；帧流协议 v2 音画同步字段，动图前端忽略）
  pub pts_ms: f64,
  pub rgba: Vec<u8>,
}

/// 单趟解码：逐帧回调，返回 false 中止；解码/打开失败返回 Err。
/// 多线程并发安全（各线程独立 demuxer/decoder；spike bench 50 线程实证）。
/// short_edge = 目标短边（真短边口径，thumbnail::shrink_to_edge 同语义；不足不放大）——
/// 每帧读取，调用方可运行期改值热更新输出尺寸（帧流卡片尺寸跟随，2026-08-20 用户定）。
pub fn decode_once(
  path: &Path,
  short_edge: &AtomicU32,
  on_frame: &mut impl FnMut(DecodedFrame) -> bool,
) -> Result<(), String> {
  ffmpeg::init().map_err(|e| format!("ffmpeg init 失败: {e:?}"))?;
  let mut ictx = ffmpeg::format::input(path).map_err(|e| format!("打开失败: {e:?}"))?;
  let stream = ictx
    .streams()
    .best(ffmpeg::media::Type::Video)
    .ok_or_else(|| "无视频流".to_string())?;
  let si = stream.index();
  let tb_num = stream.time_base().numerator() as f64;
  let tb_den = stream.time_base().denominator() as f64;
  let ctx = ffmpeg::codec::context::Context::from_parameters(stream.parameters())
    .map_err(|e| format!("解码上下文失败: {e:?}"))?;
  let mut dec = ctx.decoder().video().map_err(|e| format!("解码器失败: {e:?}"))?;

  let mut scaler: Option<ffmpeg::software::scaling::Context> = None;
  let mut dims = (0u32, 0u32);
  let mut frame = ffmpeg::frame::Video::empty();
  let mut rgb = ffmpeg::frame::Video::empty();
  let mut cum_pts = 0.0f64; // 趟内累计 delay（pts_ms）

  for (s, packet) in ictx.packets() {
    if s.index() != si {
      continue;
    }
    dec.send_packet(&packet).map_err(|e| format!("送包失败: {e:?}"))?;
    if !drain(
      &mut dec,
      &mut scaler,
      &mut dims,
      &mut frame,
      &mut rgb,
      short_edge,
      (tb_num, tb_den),
      &mut cum_pts,
      on_frame,
    ) {
      return Ok(());
    }
  }
  dec.send_eof().map_err(|e| format!("EOF 失败: {e:?}"))?;
  drain(
    &mut dec,
    &mut scaler,
    &mut dims,
    &mut frame,
    &mut rgb,
    short_edge,
    (tb_num, tb_den),
    &mut cum_pts,
    on_frame,
  );
  Ok(())
}

/// 帧缩放到目标短边（真短边等比、不足不放大，thumbnail::shrink_to_edge 同口径）→ 紧凑 RGBA。
/// 目标尺寸变化时重建 scaler 与输出帧（旧 buffer 尺寸不符会让 run 报错丢帧）；单帧失败返回 None 跳过。
#[allow(clippy::too_many_arguments)]
fn scale_frame(
  scaler: &mut Option<ffmpeg::software::scaling::Context>,
  dims: &mut (u32, u32),
  frame: &ffmpeg::frame::Video,
  rgb: &mut ffmpeg::frame::Video,
  short_edge: &AtomicU32,
) -> Option<(u32, u32, Vec<u8>)> {
  let edge = short_edge.load(Ordering::Relaxed);
  let (sw, sh) = (frame.width(), frame.height());
  let nd = if sw.min(sh) <= edge {
    (sw, sh)
  } else if sw <= sh {
    (edge, (sh as u64 * edge as u64 / sw as u64) as u32)
  } else {
    ((sw as u64 * edge as u64 / sh as u64) as u32, edge)
  };
  let nd = (nd.0.max(2), nd.1.max(2));
  if *dims != nd || scaler.is_none() {
    *dims = nd;
    *rgb = ffmpeg::frame::Video::empty();
    *scaler = Some(
      ffmpeg::software::scaling::Context::get(
        frame.format(),
        sw,
        sh,
        ffmpeg::format::Pixel::RGBA,
        nd.0,
        nd.1,
        ffmpeg::software::scaling::Flags::BILINEAR,
      )
      .expect("创建 scaler 失败"),
    );
  }
  if scaler.as_mut().unwrap().run(frame, rgb).is_err() {
    return None;
  }
  // linesize 可能大于 w*4（对齐），逐行拷出紧凑 RGBA
  let (w, h) = *dims;
  let row = (w * 4) as usize;
  let stride = rgb.stride(0);
  let data = rgb.data(0);
  let mut rgba = Vec::with_capacity(row * h as usize);
  for y in 0..h as usize {
    rgba.extend_from_slice(&data[y * stride..y * stride + row]);
  }
  Some((w, h, rgba))
}

#[allow(clippy::too_many_arguments)]
fn drain(
  dec: &mut ffmpeg::decoder::Video,
  scaler: &mut Option<ffmpeg::software::scaling::Context>,
  dims: &mut (u32, u32),
  frame: &mut ffmpeg::frame::Video,
  rgb: &mut ffmpeg::frame::Video,
  short_edge: &AtomicU32,
  tb: (f64, f64),
  cum_pts: &mut f64,
  on_frame: &mut impl FnMut(DecodedFrame) -> bool,
) -> bool {
  while dec.receive_frame(frame).is_ok() {
    let Some((w, h, rgba)) = scale_frame(scaler, dims, frame, rgb, short_edge) else {
      continue; // 单帧缩放失败跳过，不整体失败
    };
    let delay_ms = (frame.packet().duration as f64 * tb.0 / tb.1 * 1000.0).max(0.0) as u32;
    let pts_ms = *cum_pts;
    *cum_pts += delay_ms as f64;
    if !on_frame(DecodedFrame { w, h, delay_ms, pts_ms, rgba }) {
      return false;
    }
  }
  true
}

/// 视频解码选项（decode_video_once；测试专用——生产帧流直接用 VideoDecoder 持久形态）
#[cfg(test)]
pub struct VideoOpts {
  /// 起播点（秒）：>0 时 ictx.seek 真定位（关键帧落点 + 前向丢帧），0 = 从头
  pub start_secs: f64,
  /// 是否解码音轨（48kHz 立体声 f32 块推送）；无音轨 / 初始化失败静默跳过
  pub audio: bool,
}

/// 音频块：pts_ms = 块首样本时刻（与视频帧 pts 同一时间轴，音画同步基准）
pub struct AudioChunk {
  pub pts_ms: f64,
  /// 48kHz 立体声 f32 packed 交错（长度恒 AUDIO_CHUNK*2，趟末尾块可短）
  pub pcm: Vec<f32>,
}

/// 单趟视频解码（5.3，薄封装供测试 / 一次性消费者）：= VideoDecoder::open + 逐事件回调
/// （回调返回 false 中止，两回调同语义）。帧流会话用 VideoDecoder 持久形态（就地 seek 不重开）。
/// 视频帧 pts_ms 来自 best_effort_timestamp（时间轴绝对值，seek 后不从 0 起）；
/// delay_ms = 帧距（个别帧 0 时调用方按 fps 兜底）。音频块 pts 与视频同轴。
/// seek 丢帧口径：帧区间 [pts, pts+delay) 整体在开始点之前才丢（跨点帧保留 = 开始点所在帧精确命中）。
#[cfg(test)]
pub fn decode_video_once(
  path: &Path,
  opts: &VideoOpts,
  short_edge: &AtomicU32,
  on_video: &mut impl FnMut(DecodedFrame) -> bool,
  on_audio: &mut impl FnMut(AudioChunk) -> bool,
) -> Result<(), String> {
  let mut dec = VideoDecoder::open(path, opts.audio)?;
  if opts.start_secs > 0.0 {
    dec.seek(opts.start_secs)?;
  }
  loop {
    match dec.read_frame(short_edge, &|| false)? {
      Pull::Frame(f) => {
        if !on_video(f) {
          return Ok(());
        }
      }
      Pull::Audio(c) => {
        if !on_audio(c) {
          return Ok(());
        }
      }
      Pull::Eof | Pull::Interrupted => return Ok(()),
    }
  }
}

/// 拉取结果（VideoDecoder::read_frame）
pub enum Pull {
  /// 视频帧（已过 seek 丢帧过滤 + 真短边缩放）
  Frame(DecodedFrame),
  /// 音频块（48kHz 立体声 f32）
  Audio(AudioChunk),
  /// 自然到底（两路解码器均排空 + 音频残块已推）
  Eof,
  /// 丢帧阶段被 interrupt 打断（bridge 立即处理新 seek；解码位置已推进，pos_pts_ms 反映真实位置）
  Interrupted,
}

/// 持久视频解码器（5.3 性能返修，用户实机报"seek/逐帧 ~1s、scrub 卡顿"）：
/// 会话全程一份 ictx + 解码器——**seek = ictx.seek + 清解码器缓冲，不重开文件**
///（旧实现 reopen-per-seek：format::input + find_stream_info + 解码器初始化每次上百 ms 起，
/// 长 GOP 再叠加前向丢帧解码，~1s 延迟根源）；**相邻前跳 = continue_to 续解**（逐帧步进免
/// 关键帧往返：只设丢帧阈值继续解，下一帧即目标帧）；**循环重开 = seek(0) 同路径**（消除循环接缝卡顿）。
/// 拉取模型（read_frame 一次一事件）：seek 打断粒度 = 单帧（丢帧循环内逐帧查 interrupt）。
pub struct VideoDecoder {
  ictx: ffmpeg::format::context::Input,
  vi: usize,
  vtb: (f64, f64),
  vdec: ffmpeg::decoder::Video,
  audio: Option<(usize, (f64, f64), ffmpeg::decoder::Audio)>,
  resampler: Option<ffmpeg::software::resampling::Context>,
  scaler: Option<ffmpeg::software::scaling::Context>,
  dims: (u32, u32),
  frame: ffmpeg::frame::Video,
  rgb: ffmpeg::frame::Video,
  aframe: ffmpeg::frame::Audio,
  abuf: Vec<f32>,
  abuf_pts: f64,
  last_pts_ms: f64, // timestamp 缺失帧的兜底（last + delay）
  /// 解码位置（ms，最后解码帧 pts——含丢帧；bridge 相邻前跳窗口判定用）
  pos_pts_ms: f64,
  /// seek / 续解丢帧阈值（ms）：帧区间 [pts, pts+dur) 整体在阈值前才丢（跨点帧保留）
  drop_before: f64,
  demux_eof: bool, // 包流到底（其后只能 seek 重开，continue_to 无效）
  v_eof: bool,     // 视频解码器已送 EOF 标记
  a_eof: bool,
}

impl VideoDecoder {
  /// 打开 + 建两路解码器（audio=false / 无音轨 / 音轨初始化失败 = 静默无音，画面照常）
  pub fn open(path: &Path, audio: bool) -> Result<Self, String> {
    ffmpeg::init().map_err(|e| format!("ffmpeg init 失败: {e:?}"))?;
    let ictx = ffmpeg::format::input(path).map_err(|e| format!("打开失败: {e:?}"))?;
    let vstream = ictx
      .streams()
      .best(ffmpeg::media::Type::Video)
      .ok_or_else(|| "无视频流".to_string())?;
    let vi = vstream.index();
    let vtb = (
      vstream.time_base().numerator() as f64,
      vstream.time_base().denominator() as f64,
    );
    let vdec = ffmpeg::codec::context::Context::from_parameters(vstream.parameters())
      .map_err(|e| format!("解码上下文失败: {e:?}"))?
      .decoder()
      .video()
      .map_err(|e| format!("视频解码器失败: {e:?}"))?;
    let audio_pair = if audio {
      ictx.streams().best(ffmpeg::media::Type::Audio).and_then(|s| {
        let dec = ffmpeg::codec::context::Context::from_parameters(s.parameters())
          .ok()?
          .decoder()
          .audio()
          .ok()?;
        Some((
          s.index(),
          (
            s.time_base().numerator() as f64,
            s.time_base().denominator() as f64,
          ),
          dec,
        ))
      })
    } else {
      None
    };
    Ok(VideoDecoder {
      ictx,
      vi,
      vtb,
      vdec,
      audio: audio_pair,
      resampler: None,
      scaler: None,
      dims: (0, 0),
      frame: ffmpeg::frame::Video::empty(),
      rgb: ffmpeg::frame::Video::empty(),
      aframe: ffmpeg::frame::Audio::empty(),
      abuf: Vec::new(),
      abuf_pts: 0.0,
      last_pts_ms: 0.0,
      pos_pts_ms: 0.0,
      drop_before: 0.0,
      demux_eof: false,
      v_eof: false,
      a_eof: false,
    })
  }

  /// 就地 seek（秒）：容器定位（关键帧落点）+ 清两解码器缓冲 + 重采样器延迟线重建 +
  /// 攒块清空 + 丢帧阈值 = 目标点（解码前向丢帧至 pts 精确命中）。不重开文件。
  pub fn seek(&mut self, secs: f64) -> Result<(), String> {
    let secs = secs.max(0.0);
    let ts = (secs * 1_000_000.0) as i64;
    self.ictx.seek(ts, ..).map_err(|e| format!("视频 seek 失败: {e:?}"))?;
    self.vdec.flush();
    if let Some((_, _, adec)) = self.audio.as_mut() {
      adec.flush();
    }
    self.resampler = None;
    self.abuf.clear();
    let ms = secs * 1000.0;
    self.last_pts_ms = ms;
    self.pos_pts_ms = ms;
    self.drop_before = ms;
    self.demux_eof = false;
    self.v_eof = false;
    self.a_eof = false;
    Ok(())
  }

  /// 相邻前跳续解（逐帧步进 ±1 / 小步前跳免容器 seek）：只抬丢帧阈值，解码继续向前。
  /// 调用有效性由 bridge 判定（at_eof 为假且目标在当前位置之后 ≤2 帧距）
  pub fn continue_to(&mut self, secs: f64) {
    self.drop_before = secs.max(0.0) * 1000.0;
  }

  /// 包流已到底（demux EOF：续解不可，只能 seek 重开）
  pub fn at_eof(&self) -> bool {
    self.demux_eof
  }

  /// 解码位置（ms；含丢帧——丢帧阶段被 Interrupted 打断后仍反映真实推进位）
  pub fn pos_pts_ms(&self) -> f64 {
    self.pos_pts_ms
  }

  /// 拉下一个事件（一次一事件）：解码器存量优先于拉新包（send_packet 前排空 receive）。
  /// interrupt = 丢帧阶段逐帧检查（seek 打断粒度 = 单帧）；正常拉取不检查（单帧即返）。
  pub fn read_frame(&mut self, short_edge: &AtomicU32, interrupt: &dyn Fn() -> bool) -> Result<Pull, String> {
    loop {
      // A. 解码器存量优先
      if let Some(p) = self.pull_video(short_edge, interrupt) {
        return Ok(p);
      }
      if let Some(p) = self.pull_audio() {
        return Ok(p);
      }
      // B. 包流到底 → 冲刷两解码器 + 音频残块尾巴
      if self.demux_eof {
        if !self.v_eof {
          let _ = self.vdec.send_eof();
          self.v_eof = true;
          continue;
        }
        if self.audio.is_some() && !self.a_eof {
          let _ = self.audio.as_mut().unwrap().2.send_eof();
          self.a_eof = true;
          continue;
        }
        if let Some(p) = self.finish_audio_tail() {
          return Ok(p);
        }
        return Ok(Pull::Eof);
      }
      // C. 拉新包（Stream 借用不出本块——只取 index，Packet 拥有所有权）
      enum Feed {
        V(ffmpeg::Packet),
        A(ffmpeg::Packet),
        Skip,
        Eof,
      }
      let feed = match self.ictx.packets().next() {
        None => Feed::Eof,
        Some((s, p)) => {
          let i = s.index();
          if i == self.vi {
            Feed::V(p)
          } else if self.audio.as_ref().map(|a| a.0) == Some(i) {
            Feed::A(p)
          } else {
            Feed::Skip
          }
        }
      };
      match feed {
        Feed::V(p) => self.vdec.send_packet(&p).map_err(|e| format!("送包失败: {e:?}"))?,
        Feed::A(p) => {
          // 单包失败跳过（音轨不拖垮画面）
          let _ = self.audio.as_mut().unwrap().2.send_packet(&p);
        }
        Feed::Skip => {}
        Feed::Eof => self.demux_eof = true,
      }
    }
  }

  /// 视频帧排空（一次最多返回一帧；丢帧帧不过缩放零拷贝，逐帧查 interrupt）
  fn pull_video(&mut self, short_edge: &AtomicU32, interrupt: &dyn Fn() -> bool) -> Option<Pull> {
    while self.vdec.receive_frame(&mut self.frame).is_ok() {
      let delay_ms = (self.frame.packet().duration as f64 * self.vtb.0 / self.vtb.1 * 1000.0).max(0.0) as u32;
      let pts_ms = match self.frame.timestamp().or_else(|| self.frame.pts()) {
        Some(ts) => ts as f64 * self.vtb.0 / self.vtb.1 * 1000.0,
        None => self.last_pts_ms + delay_ms as f64,
      };
      self.last_pts_ms = pts_ms;
      self.pos_pts_ms = pts_ms;
      // seek / 续解丢帧：帧区间整体在阈值前（跨点帧保留 = 目标点所在帧精确命中）
      if pts_ms + delay_ms as f64 <= self.drop_before {
        if interrupt() {
          return Some(Pull::Interrupted);
        }
        continue;
      }
      let Some((w, h, rgba)) = scale_frame(&mut self.scaler, &mut self.dims, &self.frame, &mut self.rgb, short_edge)
      else {
        continue; // 单帧缩放失败跳过，不整体失败
      };
      return Some(Pull::Frame(DecodedFrame { w, h, delay_ms, pts_ms, rgba }));
    }
    None
  }

  /// 音频块排空：先吐已成块，再吃解码帧重采样攒块（丢帧口径同视频；单帧失败跳过）
  fn pull_audio(&mut self) -> Option<Pull> {
    if let Some(p) = self.emit_audio_chunk() {
      return Some(p);
    }
    loop {
      // 吃一帧（audio 字段借用限本块——emit / 重采样动其余字段不撞借用）
      let atb = {
        let (_, atb, adec) = self.audio.as_mut()?;
        let atb = *atb;
        if adec.receive_frame(&mut self.aframe).is_ok() { Some(atb) } else { None }
      };
      let Some(atb) = atb else { return None };
      resample_into(
        &mut self.resampler,
        &self.aframe,
        atb,
        self.drop_before,
        &mut self.abuf,
        &mut self.abuf_pts,
        self.last_pts_ms,
      );
      if let Some(p) = self.emit_audio_chunk() {
        return Some(p);
      }
    }
  }

  /// 攒块满 AUDIO_CHUNK 帧即吐（块 pts = 锚 + 已发块长累加；重采样连续流内自洽）
  fn emit_audio_chunk(&mut self) -> Option<Pull> {
    emit_chunk(&mut self.abuf, &mut self.abuf_pts).map(Pull::Audio)
  }

  /// EOF 尾巴（幂等，二次空调用返回 None）：重采样器延迟线尾料 + 残块推送
  fn finish_audio_tail(&mut self) -> Option<Pull> {
    if self.audio.is_none() {
      return None;
    }
    if let Some(rs) = self.resampler.as_mut() {
      let mut out = ffmpeg::frame::Audio::new(
        ffmpeg::format::Sample::F32(ffmpeg::format::sample::Type::Packed),
        AUDIO_CHUNK * 2,
        ffmpeg::ChannelLayout::STEREO,
      );
      while rs.flush(&mut out).ok().flatten().is_some() {
        let n = out.samples();
        if n == 0 {
          break;
        }
        if self.abuf.is_empty() {
          self.abuf_pts = self.last_pts_ms;
        }
        self.abuf.extend_from_slice(f32_plane(&out, n));
      }
      self.resampler = None; // 尾料只冲一次
    }
    if self.abuf.is_empty() {
      return None;
    }
    let pts_ms = self.abuf_pts;
    let pcm = std::mem::take(&mut self.abuf);
    Some(Pull::Audio(AudioChunk { pts_ms, pcm }))
  }
}

/// packed 立体声 f32 帧的前 n 帧样本切片（n×2 个 f32）。
/// 注意不能用 frame.plane::<f32>(0)——其长度 = samples() 不含声道数（ffmpeg-next 语义，
/// 踩坑 047），立体声交错数据会只取到一半。
fn f32_plane(frame: &ffmpeg::frame::Audio, n: usize) -> &[f32] {
  let bytes = frame.data(0);
  // 分配保证 linesize ≥ n×2×4（预分配 cap ≥ n），安全
  unsafe { std::slice::from_raw_parts(bytes.as_ptr() as *const f32, n * 2) }
}

/// 音频帧 → 重采样 48k 立体声 f32 → 追加进攒块缓冲（VideoDecoder::pull_audio 的纯逻辑段，
/// 单测直测用）。seek 丢帧：整块区间在阈值前丢弃；重采样器按首帧实际格式懒建，初始化 /
/// 单帧失败静默跳过（音轨降级不拖垮画面）。块 pts 锚定：缓冲空时 = 当前帧 pts。
#[allow(clippy::too_many_arguments)]
fn resample_into(
  resampler: &mut Option<ffmpeg::software::resampling::Context>,
  aframe: &ffmpeg::frame::Audio,
  atb: (f64, f64),
  drop_before: f64,
  abuf: &mut Vec<f32>,
  abuf_pts: &mut f64,
  tail_pts_ms: f64,
) {
  let Some(ts) = aframe.timestamp().or_else(|| aframe.pts()) else { return };
  let pts_ms = ts as f64 * atb.0 / atb.1 * 1000.0;
  let dur_ms = aframe.samples() as f64 / aframe.rate() as f64 * 1000.0;
  if pts_ms + dur_ms <= drop_before {
    return;
  }
  let _ = tail_pts_ms; // 保留语义（残块锚定由 finish_audio_tail 用；正常路径锚定帧 pts）
  if resampler.is_none() {
    match ffmpeg::software::resampling::Context::get(
      aframe.format(),
      aframe.channel_layout(),
      aframe.rate(),
      ffmpeg::format::Sample::F32(ffmpeg::format::sample::Type::Packed),
      ffmpeg::ChannelLayout::STEREO,
      AUDIO_RATE,
    ) {
      Ok(r) => *resampler = Some(r),
      Err(e) => {
        log::warn!("音频重采样器初始化失败（本趟无声）: {e:?}");
        return;
      }
    }
  }
  // 输出预分配：变率重采样产出可能多于输入样本（44.1k→48k），留 2 倍 + 一块余量
  let cap = aframe.samples() * 2 + AUDIO_CHUNK;
  let mut out = ffmpeg::frame::Audio::new(
    ffmpeg::format::Sample::F32(ffmpeg::format::sample::Type::Packed),
    cap,
    ffmpeg::ChannelLayout::STEREO,
  );
  if resampler.as_mut().unwrap().run(aframe, &mut out).is_err() {
    return;
  }
  let n = out.samples();
  if n == 0 {
    return;
  }
  if abuf.is_empty() {
    *abuf_pts = pts_ms;
  }
  abuf.extend_from_slice(f32_plane(&out, n));
}

/// 攒块满 AUDIO_CHUNK 帧即吐（块 pts = 锚 + 已发块长累加；重采样连续流内自洽）
fn emit_chunk(abuf: &mut Vec<f32>, abuf_pts: &mut f64) -> Option<AudioChunk> {
  if abuf.len() < AUDIO_CHUNK * 2 {
    return None;
  }
  let pcm: Vec<f32> = abuf.drain(..AUDIO_CHUNK * 2).collect();
  let pts_ms = *abuf_pts;
  *abuf_pts += AUDIO_CHUNK as f64 / AUDIO_RATE as f64 * 1000.0;
  Some(AudioChunk { pts_ms, pcm })
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 造一个 2 帧 GIF（image crate 编码），解码应出 2 帧 RGBA、短边缩放正确
  #[test]
  fn decode_gif_frames() {
    use image::codecs::gif::{GifEncoder, Repeat};
    use image::{Frame as GifFrame, ImageBuffer, Rgba, RgbaImage};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.gif");
    let mut out = std::fs::File::create(&path).unwrap();
    let mut enc = GifEncoder::new(&mut out);
    enc.set_repeat(Repeat::Infinite).unwrap();
    for i in 0..2u8 {
      let img: RgbaImage =
        ImageBuffer::from_fn(64, 48, |x, _y| Rgba([i * 100, x as u8, 0, 255]));
      let f = GifFrame::from_parts(img, 0, 0, image::Delay::from_numer_denom_ms(100, 1));
      enc.encode_frame(f).unwrap();
    }
    drop(enc);
    drop(out);

    let mut frames = Vec::new();
    decode_once(&path, &AtomicU32::new(32), &mut |f| {
      frames.push((f.w, f.h, f.delay_ms, f.rgba.len()));
      true
    })
    .unwrap();
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].0, 42); // 真短边缩放：64x48 → 42x32（64*32/48=42）
    assert_eq!(frames[0].1, 32);
    assert_eq!(frames[0].2, 100); // delay 100ms
    assert_eq!(frames[0].3, 42 * 32 * 4);
    // 中止：第一帧返回 false 应只收 1 帧
    let mut n = 0;
    decode_once(&path, &AtomicU32::new(32), &mut |_f| {
      n += 1;
      false
    })
    .unwrap();
    assert_eq!(n, 1);
  }

  /// 不足不放大：源 64x48 目标 320 → 原尺寸输出；运行期改目标短边 → 下一帧即按新尺寸
  #[test]
  fn no_upscale_and_live_resize() {
    use image::codecs::gif::{GifEncoder, Repeat};
    use image::{Frame as GifFrame, ImageBuffer, Rgba, RgbaImage};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.gif");
    let mut out = std::fs::File::create(&path).unwrap();
    let mut enc = GifEncoder::new(&mut out);
    enc.set_repeat(Repeat::Infinite).unwrap();
    for i in 0..2u8 {
      let img: RgbaImage =
        ImageBuffer::from_fn(64, 48, |x, _y| Rgba([i * 100, x as u8, 0, 255]));
      let f = GifFrame::from_parts(img, 0, 0, image::Delay::from_numer_denom_ms(100, 1));
      enc.encode_frame(f).unwrap();
    }
    drop(enc);
    drop(out);

    // 不足不放大
    let mut dims = Vec::new();
    decode_once(&path, &AtomicU32::new(320), &mut |f| {
      dims.push((f.w, f.h));
      true
    })
    .unwrap();
    assert_eq!(dims, vec![(64, 48), (64, 48)]);

    // 热更新：首帧回调改 320 → 16，第二帧按新尺寸（64x48 → 21x16）
    let edge = AtomicU32::new(320);
    let mut dims = Vec::new();
    decode_once(&path, &edge, &mut |f| {
      dims.push((f.w, f.h));
      edge.store(16, Ordering::Relaxed);
      true
    })
    .unwrap();
    assert_eq!(dims, vec![(64, 48), (21, 16)]);
  }

  /// pts 累计（gif 桥）：2 帧 100ms → pts 0 / 100
  #[test]
  fn gif_pts_accumulates() {
    use image::codecs::gif::{GifEncoder, Repeat};
    use image::{Frame as GifFrame, ImageBuffer, Rgba, RgbaImage};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.gif");
    let mut out = std::fs::File::create(&path).unwrap();
    let mut enc = GifEncoder::new(&mut out);
    enc.set_repeat(Repeat::Infinite).unwrap();
    for i in 0..2u8 {
      let img: RgbaImage = ImageBuffer::from_fn(64, 48, |x, _y| Rgba([i * 100, x as u8, 0, 255]));
      let f = GifFrame::from_parts(img, 0, 0, image::Delay::from_numer_denom_ms(100, 1));
      enc.encode_frame(f).unwrap();
    }
    drop(enc);
    drop(out);
    let mut pts = Vec::new();
    decode_once(&path, &AtomicU32::new(32), &mut |f| {
      pts.push(f.pts_ms);
      true
    })
    .unwrap();
    assert_eq!(pts, vec![0.0, 100.0]);
  }

  /// 音频重采样（resample_into + emit_chunk 直测，无外部资产）：44.1k mono f32 → 48k stereo f32
  /// 攒块，块长 / 块 pts 锚定与递增 / 总量换算正确
  #[test]
  fn audio_resample_chunks() {
    use ffmpeg::format::sample::Type;
    let mut rs = None;
    let mut abuf: Vec<f32> = Vec::new();
    let mut abuf_pts = 0.0f64;
    let mut chunks: Vec<(f64, usize)> = Vec::new();
    // 4 帧 44.1k mono f32 packed（各 2048 样本，pts 连续，时间基 1/44100）
    let mut pts = 0i64;
    for _ in 0..4 {
      let mut f = ffmpeg::frame::Audio::new(
        ffmpeg::format::Sample::F32(Type::Packed),
        2048,
        ffmpeg::ChannelLayout::MONO,
      );
      f.set_rate(44100);
      f.set_pts(Some(pts));
      pts += 2048;
      for (i, s) in f.plane_mut::<f32>(0).iter_mut().enumerate() {
        *s = ((pts + i as i64) as f32 * 0.01).sin() * 0.5;
      }
      resample_into(&mut rs, &f, (1.0, 44100.0), 0.0, &mut abuf, &mut abuf_pts, 0.0);
      while let Some(c) = emit_chunk(&mut abuf, &mut abuf_pts) {
        chunks.push((c.pts_ms, c.pcm.len()));
      }
    }
    // 总量 ≈ 4×2048 × 48000/44100 ≈ 8918 样本/声道 → 8 整块（重采样延迟吃掉尾部，宽容 ≥6）
    assert!(chunks.len() >= 6, "块数不足：{}", chunks.len());
    assert!(chunks.iter().all(|&(_, len)| len == AUDIO_CHUNK * 2), "块长应为 AUDIO_CHUNK×2");
    assert_eq!(chunks[0].0, 0.0, "首块 pts 应锚定 0");
    let step = AUDIO_CHUNK as f64 / AUDIO_RATE as f64 * 1000.0;
    for w in chunks.windows(2) {
      assert!((w[1].0 - w[0].0 - step).abs() < 0.01, "块 pts 应等距递增 {step}ms");
    }
  }

  /// 音频 seek 丢帧（resample_into）：整块区间在开始点之前丢弃，跨点帧保留
  #[test]
  fn audio_seek_drops_before_start() {
    use ffmpeg::format::sample::Type;
    let mut rs = None;
    let mut abuf: Vec<f32> = Vec::new();
    let mut abuf_pts = 0.0f64;
    let mut chunks: Vec<f64> = Vec::new();
    // 5 帧各 44100 样本（1 秒/帧）；start = 2.5s → 前 2 帧整块丢，第 3 帧（2.0~3.0s）跨点保留
    for i in 0..5i64 {
      let mut f = ffmpeg::frame::Audio::new(
        ffmpeg::format::Sample::F32(Type::Packed),
        44100,
        ffmpeg::ChannelLayout::MONO,
      );
      f.set_rate(44100);
      f.set_pts(Some(i * 44100));
      resample_into(&mut rs, &f, (1.0, 44100.0), 2500.0, &mut abuf, &mut abuf_pts, 0.0);
      while let Some(c) = emit_chunk(&mut abuf, &mut abuf_pts) {
        chunks.push(c.pts_ms);
      }
    }
    assert!(!chunks.is_empty(), "跨点帧之后应有产出");
    assert!(chunks[0] >= 2000.0, "首块 pts 应 ≥ 跨点帧起点（实际 {}）", chunks[0]);
  }

  /// sample.mp4（mpeg4 无音轨 10fps 2s）：视频帧 pts 单调 / 帧数 ≈20 / 无音轨不报错
  #[test]
  fn decode_video_sample() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/test-assets/sample.mp4");
    if !path.is_file() {
      eprintln!("跳过：tools/test-assets/sample.mp4 缺失");
      return;
    }
    let mut frames: Vec<(f64, u32, u32, u32)> = Vec::new();
    decode_video_once(
      &path,
      &VideoOpts { start_secs: 0.0, audio: true }, // audio=true 但无音轨：静默跳过
      &AtomicU32::new(64),
      &mut |f| {
        frames.push((f.pts_ms, f.delay_ms, f.w, f.h));
        true
      },
      &mut |_| panic!("无音轨不应出音频块"),
    )
    .unwrap();
    assert!(frames.len() >= 18, "2s 10fps 应 ≈20 帧（实际 {}）", frames.len());
    for w in frames.windows(2) {
      assert!(w[1].0 > w[0].0, "pts 应严格单调递增");
    }
    assert!(frames[0].2 >= 2 && frames[0].3 >= 2, "尺寸应有效");
  }

  /// VideoDecoder 持久语义（5.3 性能返修）：就地 seek 不重开 / 相邻前跳续解 / Eof 后 seek 重开
  #[test]
  fn video_decoder_inplace_seek_and_continue() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/test-assets/sample.mp4");
    if !path.is_file() {
      eprintln!("跳过：tools/test-assets/sample.mp4 缺失");
      return;
    }
    let edge = AtomicU32::new(64);
    let no_int = || false;
    let mut dec = VideoDecoder::open(&path, false).unwrap();
    // 顺序读 3 帧（10fps：pts 0/100/200）
    let mut pts = Vec::new();
    for _ in 0..3 {
      match dec.read_frame(&edge, &no_int).unwrap() {
        Pull::Frame(f) => pts.push(f.pts_ms),
        _ => panic!("应出视频帧"),
      }
    }
    assert_eq!(pts, vec![0.0, 100.0, 200.0], "顺序帧 pts");
    assert_eq!(dec.pos_pts_ms(), 200.0);
    // 相邻前跳续解（目标 300ms = 下一帧）：不 seek 容器，直接丢帧续解到位
    dec.continue_to(0.3);
    match dec.read_frame(&edge, &no_int).unwrap() {
      Pull::Frame(f) => assert_eq!(f.pts_ms, 300.0, "续解应精确落下一帧"),
      _ => panic!("应出视频帧"),
    }
    // 就地回退 seek（不重开）：回 0.5s → 精确落 500ms 帧
    dec.seek(0.5).unwrap();
    match dec.read_frame(&edge, &no_int).unwrap() {
      Pull::Frame(f) => assert!((f.pts_ms - 500.0).abs() <= 50.0, "回退 seek 落点（{}）", f.pts_ms),
      _ => panic!("应出视频帧"),
    }
    assert_eq!(dec.pos_pts_ms(), 500.0, "seek 后位置跟踪");
    // 读到底 → Eof；Eof 后 seek(0) 重开 → 首帧 pts≈0
    loop {
      match dec.read_frame(&edge, &no_int).unwrap() {
        Pull::Frame(_) => {}
        Pull::Eof => break,
        _ => panic!("无音轨不应有音频 / 无打断不应 Interrupted"),
      }
    }
    assert!(dec.at_eof());
    dec.seek(0.0).unwrap();
    assert!(!dec.at_eof());
    match dec.read_frame(&edge, &no_int).unwrap() {
      Pull::Frame(f) => assert_eq!(f.pts_ms, 0.0, "seek(0) 重开后首帧"),
      _ => panic!("应出视频帧"),
    }
  }

  /// 视频真 seek：start=1.0s → 首帧 = 1.0s 所在帧（10fps 帧距 100ms，落点容差 ±50ms）；
  /// 后续帧连续到底（≈10 帧）——关键帧落点 + 前向丢帧的正确性
  #[test]
  fn decode_video_seek_lands() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/test-assets/sample.mp4");
    if !path.is_file() {
      eprintln!("跳过：tools/test-assets/sample.mp4 缺失");
      return;
    }
    let mut pts: Vec<f64> = Vec::new();
    decode_video_once(
      &path,
      &VideoOpts { start_secs: 1.0, audio: false },
      &AtomicU32::new(64),
      &mut |f| {
        pts.push(f.pts_ms);
        true
      },
      &mut |_| true,
    )
    .unwrap();
    assert!(!pts.is_empty(), "seek 后应有帧");
    assert!(
      (pts[0] - 1000.0).abs() <= 50.0,
      "首帧应精确落在 1000ms 所在帧（实际 {}）",
      pts[0]
    );
    for w in pts.windows(2) {
      assert!(w[1] > w[0], "seek 后 pts 应单调递增");
    }
    assert!(pts.len() >= 8, "1.0s 起到 2s 应 ≈10 帧（实际 {}）", pts.len());
  }

  /// 带音 AV 全链路（测试内经 ffmpeg sidecar 现生成 testsrc+sine 样本；sidecar 缺失跳过）：
  /// 音画交错产出 / 音频块总量 ≈ 时长 / seek 后音画都从目标点附近起
  #[test]
  fn decode_video_with_audio() {
    let ff = Path::new(env!("CARGO_MANIFEST_DIR")).join("binaries/ffmpeg-x86_64-pc-windows-msvc.exe");
    if !ff.is_file() {
      eprintln!("跳过：ffmpeg sidecar 缺失（python tools/fetch-ffmpeg.py）");
      return;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("av.mp4");
    let out = std::process::Command::new(&ff)
      .args([
        "-y",
        "-f",
        "lavfi",
        "-i",
        "testsrc=size=64x48:rate=10",
        "-f",
        "lavfi",
        "-i",
        "sine=frequency=440:sample_rate=44100",
        "-t",
        "1.5",
        "-pix_fmt",
        "yuv420p",
      ])
      .arg(&path)
      .stdout(std::process::Stdio::null())
      .stderr(std::process::Stdio::null())
      .output()
      .unwrap();
    assert!(out.status.success(), "sidecar 生成测试视频失败");

    // 全程：帧 ≈15、音频块 pts 从 0 起连续、音频总样本 ≈1.5s
    let mut vpts: Vec<f64> = Vec::new();
    let mut chunks: Vec<(f64, usize)> = Vec::new();
    decode_video_once(
      &path,
      &VideoOpts { start_secs: 0.0, audio: true },
      &AtomicU32::new(64),
      &mut |f| {
        vpts.push(f.pts_ms);
        true
      },
      &mut |c| {
        chunks.push((c.pts_ms, c.pcm.len()));
        true
      },
    )
    .unwrap();
    assert!(vpts.len() >= 13, "1.5s 10fps 应 ≈15 帧（实际 {}）", vpts.len());
    assert!(!chunks.is_empty(), "应有音频块");
    let total: usize = chunks.iter().map(|c| c.1).sum();
    let secs = total as f64 / 2.0 / AUDIO_RATE as f64;
    assert!((secs - 1.5).abs() < 0.15, "音频总量应 ≈1.5s（实际 {secs:.2}s）");
    assert!(chunks[0].0 < 100.0, "首音频块 pts 应近 0（实际 {}）", chunks[0].0);
    for w in chunks.windows(2) {
      assert!(w[1].0 > w[0].0, "音频块 pts 应单调递增");
    }

    // seek 0.6s：视频首帧与音频首块都落在 0.6s 附近
    let mut v0 = None;
    let mut a0 = None;
    decode_video_once(
      &path,
      &VideoOpts { start_secs: 0.6, audio: true },
      &AtomicU32::new(64),
      &mut |f| {
        v0.get_or_insert(f.pts_ms);
        true
      },
      &mut |c| {
        a0.get_or_insert(c.pts_ms);
        true
      },
    )
    .unwrap();
    let v0 = v0.expect("seek 后应有视频帧");
    assert!((v0 - 600.0).abs() <= 100.0, "视频首帧应落 600ms 帧（实际 {v0}）");
    if let Some(a0) = a0 {
      assert!(a0 >= 500.0 && a0 <= 700.0, "音频首块应在 600ms 附近（实际 {a0}）");
    }
  }
}
