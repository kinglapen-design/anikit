// 帧流客户端（阶段四·自研解码器，协议 v2）：start_anim_stream 拿会话 → WS 收帧 → 组件渲染。
// 协议 v2（services/frame_stream 推送）：视频帧（二进制 tag 0）：
//   `[0][w u32le][h u32le][idx u32le][pts_ms f64le][seq u64le][rgba]`
//   （idx = 源帧索引 0 起循环回绕，预览播放条帧号 / 进度用；pts_ms = 帧时刻，趟内累计 delay）
// 出帧尺寸 = 调用方显示短边（真短边等比、不足不放大）；setSize 热更新（不重启会话、不回第一帧）。
// 播放控制（预览窗口）：setPlaying / setRate / seek（动图帧索引，GIF/WEBP 无索引顺序解码——
// 前进 = 后端快进，回退 = 重开从头快进；暂停 + seek±1 = 逐帧步进）。
// 一预览单实例（卡片层 2026-09-02 回滚原生 <img> 播放，帧流仅预览面板在用）；失败（非动图 / 上限 /
// 解码失败 / WS 不通）一律 null——调用方回退静态封面。
//（5.3 视频接入已回滚，2026-08-22：视频播放回 <video> 元素 asset 直读，帧流仅动图在用——
// 视频会话 / 音轨 PCM / 音画同步随之拆除，存档见 12d8bc6）
import { invoke } from "@tauri-apps/api/core";
import type { SegmentRange } from "./semantic";

export interface StreamFrame {
  w: number;
  h: number;
  /** 源帧索引（0 起，循环回绕归 0） */
  frameIndex: number;
  /** 帧时刻（ms，趟内累计 delay） */
  ptsMs: number;
  seq: number;
  rgba: Uint8ClampedArray<ArrayBuffer>;
}

export class StreamSession {
  private ws: WebSocket | null = null;
  private stopped = false;

  private constructor(
    private readonly session: number,
    private readonly onFrame: (f: StreamFrame) => void,
    readonly segment: SegmentRange | null,
  ) {}

  /** 动图会话（预览）：shortEdge = 显示短边（后端 clamp）。失败返回 null（回退静态封面） */
  static async start(assetId: string, shortEdge: number, onFrame: (f: StreamFrame) => void): Promise<StreamSession | null> {
    let info: { session: number; port: number; segment?: SegmentRange };
    try {
      info = await invoke<{ session: number; port: number; segment?: SegmentRange }>("start_anim_stream", { assetId, shortEdge });
    } catch {
      return null;
    }
    if (!info.port) return null; // WS server 未起（初始化失败）：同样回退
    const s = new StreamSession(info.session, onFrame, info.segment ?? null);
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

  /** 倍速（预览播放条）：幂等，失败静默 */
  setRate(rate: number): void {
    if (this.stopped) return;
    void invoke("set_anim_stream_rate", { session: this.session, rate }).catch(() => {});
  }

  /** 定位到目标帧（动图预览播放条拖动 / 逐帧步进）：幂等，失败静默 */
  seek(frameIndex: number): void {
    if (this.stopped) return;
    void invoke("seek_anim_stream", { session: this.session, frameIndex }).catch(() => {});
  }

  private connect(port: number): void {
    const ws = new WebSocket(`ws://127.0.0.1:${port}/${this.session}`);
    ws.binaryType = "arraybuffer";
    ws.onmessage = (ev) => {
      if (this.stopped || !(ev.data instanceof ArrayBuffer)) return;
      const buf = ev.data;
      // 视频帧（tag 0）：[0][w u32][h u32][idx u32][pts_ms f64][seq u64][rgba]
      //（文本控制消息 "flush"/"ended"/"error" 动图路径不消费，忽略）
      if (buf.byteLength < 29) return;
      const dv = new DataView(buf);
      if (dv.getUint8(0) !== 0) return;
      const w = dv.getUint32(1, true);
      const h = dv.getUint32(5, true);
      const frameIndex = dv.getUint32(9, true);
      const ptsMs = dv.getFloat64(13, true);
      const seq = Number(dv.getBigUint64(21, true));
      this.onFrame({ w, h, frameIndex, ptsMs, seq, rgba: new Uint8ClampedArray(buf, 29) });
    };
    // 服务端断开（会话消亡 / 库切换 stop_all）：不自动重连，维持静态封面
    this.ws = ws;
  }

  /** 停止会话（幂等）：关 WS + 后端解码线程停 */
  stop(): void {
    if (this.stopped) return;
    this.stopped = true;
    this.ws?.close();
    this.ws = null;
    void invoke("stop_anim_stream", { session: this.session }).catch(() => {});
  }
}
