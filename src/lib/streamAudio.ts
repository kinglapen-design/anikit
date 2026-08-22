// 帧流音频播放（5.3 全自研 PCM 链路，用户定）：后端 ffmpeg-next 解码音轨 → 48kHz 立体声 f32
// PCM 块经 WS 推前端 → 本模块 WebAudio 调度播放。
// 模型：
// - AudioContext 全局单例 lazy（浏览器自动播放策略：suspended 需用户手势恢复——取消静音的
//   点击即手势，resume() 后从下一音频块开始调度；静音期天然不调度 = 丢块，无任何积压）；
// - 每会话一个 GainNode → destination：音量 / 静音纯本地（增益 0），后端不感知；
// - 块调度：首块锚定 anchor = (块 pts, ctx.currentTime + LEAD)，其后逐块拼接
//   （nextAt += 块时长 / rate；source.playbackRate = rate——v1 音调随速变，时间拉伸保调记 09 待办）；
// - flush（seek 重推）：停全部已排源 + 锚定失效，下一块重新锚定（天然无旧位残响）；
// - clockPts() = 当前可闻 pts（音画同步基准；静音 / 未锚定 / suspended = null——帧到达即渲）。
// 漂移处理：调度时刻落后 ctx 时钟（块迟到 / 主线程卡顿）→ 重锚到"现在 + LEAD"，不追赶旧账。
import { AUDIO_RATE } from "./streamClient";

/// 首块预滚提前量（秒）：吸收 WS / 主线程抖动（本地回环亚毫秒级，150ms 足够）
const LEAD = 0.15;
/// 迟到重锚阈值（秒）：调度落后超过它放弃追平，重锚（防咔嗒连发）
const LATE_REANCHOR = 0.08;

let sharedCtx: AudioContext | null = null;

/** 全局共享 AudioContext（卡片多实例复用——浏览器单域 AudioContext 数有上限）。
 * 自动播放策略：无用户手势创建的 ctx 起步 suspended（独立预览窗口直接打开即此场景）——
 * 挂一次性手势监听（pointerdown/keydown 捕获）恢复，恢复前 push 丢块不积压（静音同径） */
function audioContext(): AudioContext {
  if (!sharedCtx) {
    sharedCtx = new AudioContext();
    if (sharedCtx.state === "suspended") {
      const resume = () => {
        void sharedCtx?.resume();
        document.removeEventListener("pointerdown", resume, true);
        document.removeEventListener("keydown", resume, true);
      };
      document.addEventListener("pointerdown", resume, true);
      document.addEventListener("keydown", resume, true);
    }
  }
  return sharedCtx;
}

/** 单会话音频播放器：GainNode + 块调度 + 锚定时钟 */
export class StreamAudioPlayer {
  private readonly gain: GainNode;
  private muted = true; // 默认静音（卡片自动播放恒静音；悬停 / 预览可开）
  private volume = 1;
  private rate = 1;
  private anchored = false;
  private anchorPts = 0; // 锚定块 pts（ms）
  private anchorCtx = 0; // 锚定块调度时刻（ctx 秒）
  private nextAt = 0; // 下一块调度时刻（ctx 秒）
  private nextPts = 0; // 下一块 pts（ms）
  private sources = new Set<AudioBufferSourceNode>();
  private disposed = false;

  constructor() {
    const ctx = audioContext();
    this.gain = ctx.createGain();
    this.gain.gain.value = 0;
    this.gain.connect(ctx.destination);
  }

  private get ctx(): AudioContext {
    return audioContext();
  }

  /** 静音切换（卡片音量钮 / 预览播放条）：静音 = 丢块不调度 */
  setMuted(muted: boolean): void {
    this.muted = muted;
    this.gain.gain.value = muted ? 0 : this.volume;
    if (!muted && this.ctx.state === "suspended") void this.ctx.resume(); // 取消静音的点击 = 用户手势
  }

  /** 音量 0-1（拖离 0 自动解除静音——对齐 previewMedia 既有语义） */
  setVolume(volume: number): void {
    this.volume = Math.min(1, Math.max(0, volume));
    if (this.volume > 0 && this.muted) this.muted = false;
    this.gain.gain.value = this.muted ? 0 : this.volume;
    if (!this.muted && this.ctx.state === "suspended") void this.ctx.resume();
  }

  /** 倍速（播放条倍率）：source.playbackRate——v1 音调随速变（时间拉伸保调待评估，09 待办） */
  setRate(rate: number): void {
    if (rate > 0 && Number.isFinite(rate)) this.rate = rate;
  }

  /** 收音频块：静音 / suspended 丢块；锚定后逐块拼接调度 */
  push(ptsMs: number, pcm: Float32Array): void {
    if (this.disposed || this.muted || pcm.length === 0) return;
    if (this.ctx.state !== "running") return; // suspended：等手势恢复，丢块不积压
    const frames = pcm.length / 2;
    if (!this.anchored || this.nextAt < this.ctx.currentTime - LATE_REANCHOR) {
      // 首块 / 迟到重锚：从"现在 + LEAD"起播本块
      this.anchored = true;
      this.anchorPts = ptsMs;
      this.anchorCtx = this.ctx.currentTime + LEAD;
      this.nextAt = this.anchorCtx;
      this.nextPts = ptsMs;
    }
    const buf = this.ctx.createBuffer(2, frames, AUDIO_RATE);
    // 交错 LRLR → 分声道拷入（Float32Array 视图直接填）
    const l = buf.getChannelData(0);
    const r = buf.getChannelData(1);
    for (let i = 0; i < frames; i++) {
      l[i] = pcm[i * 2];
      r[i] = pcm[i * 2 + 1];
    }
    const src = this.ctx.createBufferSource();
    src.buffer = buf;
    src.playbackRate.value = this.rate;
    src.connect(this.gain);
    src.onended = () => this.sources.delete(src);
    this.sources.add(src);
    // 块 pts 应接续上一块（后端攒块 pts 等距）；微小错位以调度链为准不纠（防叠音）
    src.start(this.nextAt);
    this.nextAt += frames / AUDIO_RATE / this.rate;
    this.nextPts = ptsMs + (frames / AUDIO_RATE) * 1000;
    void this.nextPts; // 保留语义（clockPts 推导用 anchor 而非逐块，见下）
  }

  /** 当前可闻 pts（ms；音画同步基准）：静音 / 未锚定 / 非运行态 = null（帧到达即渲） */
  clockPts(): number | null {
    if (this.disposed || this.muted || !this.anchored || this.ctx.state !== "running") return null;
    return this.anchorPts + (this.ctx.currentTime - this.anchorCtx) * this.rate * 1000;
  }

  /** seek 重推（后端 flush 控制消息）：停全部已排源 + 锚定失效（下一块重新锚定） */
  flush(): void {
    for (const s of this.sources) {
      try {
        s.stop();
      } catch {
        // 未 start / 已停的源忽略
      }
      s.disconnect();
    }
    this.sources.clear();
    this.anchored = false;
  }

  /** 会话结束：flush + 断开增益节点 */
  dispose(): void {
    this.disposed = true;
    this.flush();
    this.gain.disconnect();
  }
}
