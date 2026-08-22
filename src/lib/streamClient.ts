// 帧流客户端（阶段四·自研解码器；5.3 视频接入 + 协议 v2）：start_anim_stream / start_video_stream
// 拿会话 → WS 收流 → 组件渲染。
// 协议 v2（services/frame_stream 推送）：
// - 视频帧（二进制 tag 0）：`[0][w u32le][h u32le][idx u32le][pts_ms f64le][seq u64le][rgba]`
//   （idx = 源帧索引 0 起循环回绕，预览播放条帧号 / 进度用；pts_ms = 帧时刻——音画同步基准）
// - 音频块（二进制 tag 1，视频有音轨时）：`[1][pts_ms f64le][frames u32le][pcm f32le × frames × 2]`
//   （48kHz 立体声；调度播放委托 streamAudio——静音 / 音量 / 倍速 / flush 全在那里）
// - 文本控制消息："flush"（seek 重推——清音频队列与待渲帧）/ "ended"（loop=false 到底）
// 出帧尺寸 = 调用方显示短边（真短边等比、不足不放大）；setSize 热更新（不重启会话、不回第一帧）。
// 播放控制（预览窗口）：setPlaying / setRate / seek（动图帧索引，GIF/WEBP 无索引顺序解码——
// 前进 = 后端快进，回退 = 重开从头快进；暂停 + seek±1 = 逐帧步进）/ seekVideo（视频秒，
// 后端 ffmpeg 真 seek）/ setLoop（视频循环钮）。
// 音画同步：音频可闻（未静音且有音轨）时帧按 pts 排队等音频钟（rAF 驱动）；否则到达即渲。
// 一卡一会话 / 预览单实例；失败（非动图视频 / 上限 / 解码失败 / WS 不通）一律 null——调用方回退静态封面。
import { invoke } from "@tauri-apps/api/core";
import { StreamAudioPlayer } from "./streamAudio";

/** 音频块采样率（Hz，与后端 decoder::AUDIO_RATE 一致；streamAudio 建 AudioBuffer 用） */
export const AUDIO_RATE = 48000;

export interface StreamFrame {
  w: number;
  h: number;
  /** 源帧索引（0 起，循环回绕归 0；视频 = round(pts×fps) 推导） */
  frameIndex: number;
  /** 帧时刻（ms；动图 = 趟内累计 delay，视频 = 流时间轴绝对值） */
  ptsMs: number;
  seq: number;
  rgba: Uint8ClampedArray<ArrayBuffer>;
}

/** start_video_stream 返回（serde camelCase） */
interface VideoStreamStart {
  session: number;
  port: number;
  hasAudio: boolean;
  fps: number | null;
  duration: number | null;
}

export interface StreamCbs {
  onFrame: (f: StreamFrame) => void;
  /** "flush" = seek 重推（队列已自清，调用方只需关注语义如 seek 目标等待）；
   * "ended" = loop=false 到底；"error" = 会话线程死亡（解码错误等——调用方降级静态封面/报错，防静默冻结） */
  onControl?: (c: "flush" | "ended" | "error") => void;
}

/** 待渲帧队列上限（音画同步）：超限丢到最新帧（追帧防膨胀；正常播放恒 ≤2） */
const PENDING_MAX = 8;
/** 待渲帧等音频钟的墙钟上限（ms，2026-08-22 实机/探针两侧定稿）：音频钟异常（无头环境假音频设备
 * currentTime 跳变、真机音频设备异常）时帧不许无限等钟——超时即渲，音画偏差封顶，防"假冻结" */
const PENDING_MAX_WAIT = 120;

export class StreamSession {
  private ws: WebSocket | null = null;
  private stopped = false;
  /** 音频播放器（hasAudio 才建；静音 / 音量 / 倍速 / flush / 时钟全委托） */
  private audio: StreamAudioPlayer | null = null;
  /** 音画同步待渲队列（仅音频可闻时非空；at = 入队墙钟，防音频钟异常时无限等钟） */
  private pending: { f: StreamFrame; at: number }[] = [];
  private raf = 0;

  private constructor(
    private readonly session: number,
    readonly hasAudio: boolean,
    readonly fps: number | null,
    readonly durationSec: number | null,
    private readonly cbs: StreamCbs,
  ) {}

  /** 动图会话（卡片 / 预览）：shortEdge = 显示短边（后端 clamp）。失败返回 null（回退静态封面） */
  static async start(assetId: string, shortEdge: number, onFrame: (f: StreamFrame) => void): Promise<StreamSession | null> {
    let info: { session: number; port: number };
    try {
      info = await invoke<{ session: number; port: number }>("start_anim_stream", { assetId, shortEdge });
    } catch {
      return null;
    }
    if (!info.port) return null; // WS server 未起（初始化失败）：同样回退
    const s = new StreamSession(info.session, false, null, null, { onFrame });
    s.connect(info.port);
    return s;
  }

  /** 视频会话（5.3）：startSecs = 起播点（卡片自动播放 = 封面帧；预览 = 0）；loop = 循环。失败返回 null */
  static async startVideo(
    assetId: string,
    opts: { shortEdge: number; startSecs?: number; loop?: boolean },
    cbs: StreamCbs,
  ): Promise<StreamSession | null> {
    let info: VideoStreamStart;
    try {
      info = await invoke<VideoStreamStart>("start_video_stream", {
        assetId,
        shortEdge: opts.shortEdge,
        startSecs: opts.startSecs ?? 0,
        loopEnabled: opts.loop ?? true,
      });
    } catch {
      return null;
    }
    if (!info.port) return null;
    const s = new StreamSession(info.session, info.hasAudio, info.fps, info.duration, cbs);
    if (info.hasAudio) s.audio = new StreamAudioPlayer();
    s.connect(info.port);
    return s;
  }

  /** 热更新出帧尺寸（显示尺寸跟随）：幂等，失败静默（会话可能已消亡） */
  setSize(shortEdge: number): void {
    if (this.stopped) return;
    void invoke("set_anim_stream_size", { session: this.session, shortEdge }).catch(() => {});
  }

  /** 播放 / 暂停（预览）：幂等，失败静默 */
  setPlaying(playing: boolean): void {
    if (this.stopped) return;
    void invoke("set_anim_stream_playing", { session: this.session, playing }).catch(() => {});
  }

  /** 倍速（预览播放条）：帧节奏 + 音频 playbackRate 同步；幂等，失败静默 */
  setRate(rate: number): void {
    if (this.stopped) return;
    this.audio?.setRate(rate);
    void invoke("set_anim_stream_rate", { session: this.session, rate }).catch(() => {});
  }

  /** 定位到目标帧（动图预览播放条拖动 / 逐帧步进）：幂等，失败静默 */
  seek(frameIndex: number): void {
    if (this.stopped) return;
    void invoke("seek_anim_stream", { session: this.session, frameIndex }).catch(() => {});
  }

  /** 定位到目标秒（视频）：后端真 seek（任意方向）；幂等，失败静默 */
  seekVideo(secs: number): void {
    if (this.stopped) return;
    void invoke("seek_video_stream", { session: this.session, secs }).catch(() => {});
  }

  /** 循环开关（视频播放条循环钮）：幂等，失败静默 */
  setLoop(loopEnabled: boolean): void {
    if (this.stopped) return;
    void invoke("set_stream_loop", { session: this.session, loopEnabled }).catch(() => {});
  }

  /** 静音（卡片音量钮 / 预览播放条；静音 = 音频块丢弃不调度，帧到达即渲） */
  setMuted(muted: boolean): void {
    this.audio?.setMuted(muted);
  }

  /** 音量 0-1（拖离 0 自动解除静音） */
  setVolume(volume: number): void {
    this.audio?.setVolume(volume);
  }

  private connect(port: number): void {
    const ws = new WebSocket(`ws://127.0.0.1:${port}/${this.session}`);
    ws.binaryType = "arraybuffer";
    ws.onmessage = (ev) => {
      if (this.stopped) return;
      if (typeof ev.data === "string") {
        this.onControlMsg(ev.data);
        return;
      }
      if (!(ev.data instanceof ArrayBuffer)) return;
      const buf = ev.data;
      if (buf.byteLength < 1) return;
      const dv = new DataView(buf);
      const tag = dv.getUint8(0);
      if (tag === 0) {
        // 视频帧：[0][w u32][h u32][idx u32][pts_ms f64][seq u64][rgba]
        if (buf.byteLength < 29) return;
        const w = dv.getUint32(1, true);
        const h = dv.getUint32(5, true);
        const frameIndex = dv.getUint32(9, true);
        const ptsMs = dv.getFloat64(13, true);
        const seq = Number(dv.getBigUint64(21, true));
        this.onFrame({ w, h, frameIndex, ptsMs, seq, rgba: new Uint8ClampedArray(buf, 29) });
      } else if (tag === 1) {
        // 音频块：[1][pts_ms f64][frames u32][pcm f32 × frames × 2]
        if (buf.byteLength < 13) return;
        const ptsMs = dv.getFloat64(1, true);
        const frames = dv.getUint32(9, true);
        if (buf.byteLength < 13 + frames * 8) return;
        const pcm = new Float32Array(buf.slice(13, 13 + frames * 8));
        this.audio?.push(ptsMs, pcm);
      }
    };
    // 服务端断开（会话消亡 / 库切换 stop_all）：不自动重连，维持静态封面
    this.ws = ws;
  }

  /** 帧到达：音频可闻（未静音 + 有音轨）按 pts 排队等音频钟；否则到达即渲 */
  private onFrame(f: StreamFrame): void {
    const clock = this.audio?.clockPts() ?? null;
    if (clock == null) {
      if (this.pending.length) this.pending.length = 0; // 音频钟消失（静音/暂停）：队列废弃
      this.cbs.onFrame(f);
      return;
    }
    this.pending.push({ f, at: performance.now() });
    if (this.pending.length > PENDING_MAX) {
      this.pending.splice(0, this.pending.length - 2); // 追帧：只留最新两帧
    }
    this.drain(clock);
  }

  /** 按音频钟出帧：渲染最后一个到点帧；等钟超 PENDING_MAX_WAIT 的帧强渲（音频钟异常自愈） */
  private drain(clock: number): void {
    const now = performance.now();
    let last: StreamFrame | null = null;
    while (this.pending.length && (this.pending[0].f.ptsMs <= clock || now - this.pending[0].at > PENDING_MAX_WAIT)) {
      last = this.pending.shift()!.f;
    }
    if (last) this.cbs.onFrame(last);
    if (!this.pending.length || this.raf) return;
    this.raf = requestAnimationFrame(() => {
      this.raf = 0;
      if (this.stopped) return;
      const c = this.audio?.clockPts() ?? null;
      if (c == null) {
        const last2 = this.pending.pop();
        this.pending.length = 0;
        if (last2) this.cbs.onFrame(last2.f);
        return;
      }
      this.drain(c);
    });
  }

  private onControlMsg(msg: string): void {
    if (msg === "flush") {
      this.pending.length = 0;
      this.audio?.flush();
    }
    if (msg === "flush" || msg === "ended" || msg === "error") this.cbs.onControl?.(msg);
  }

  /** 停止会话（幂等）：关 WS + 音频释放 + 后端解码线程停 */
  stop(): void {
    if (this.stopped) return;
    this.stopped = true;
    if (this.raf) {
      cancelAnimationFrame(this.raf);
      this.raf = 0;
    }
    this.pending.length = 0;
    this.audio?.dispose();
    this.audio = null;
    this.ws?.close();
    this.ws = null;
    void invoke("stop_anim_stream", { session: this.session }).catch(() => {});
  }
}
