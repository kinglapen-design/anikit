// 动图帧流客户端（阶段四·自研解码器）：start_anim_stream 拿会话 → WS 收帧 → 组件渲染。
// 帧格式 = [w u32le][h u32le][idx u32le][seq u64le][rgba...]（services/frame_stream 推送，RGBA；
// idx = 源帧索引 0 起循环回绕，预览播放条帧号 / 进度用）。
// 出帧尺寸 = 调用方显示短边（真短边等比、不足不放大）；setSize 热更新（不重启会话、不回第一帧）。
// 播放控制（预览窗口）：setPlaying / setRate / seek（GIF/WEBP 无索引只能顺序解码——
// 前进 = 后端快进，回退 = 重开从头快进；暂停 + seek±1 = 逐帧步进）。
// 一卡一会话 / 预览单实例；失败（非动图 / 上限 / 解码失败 / WS 不通）一律 null——调用方回退静态封面。
import { invoke } from "@tauri-apps/api/core";

export interface AnimFrame {
  w: number;
  h: number;
  /** 源帧索引（0 起，循环回绕归 0） */
  frameIndex: number;
  seq: number;
  rgba: Uint8ClampedArray<ArrayBuffer>;
}

export class AnimStreamSession {
  private ws: WebSocket | null = null;
  private stopped = false;

  private constructor(
    private readonly session: number,
    private readonly onFrame: (f: AnimFrame) => void,
  ) {}

  /** 启动会话并连 WS；shortEdge = 显示短边（后端 clamp）。失败返回 null（回退静态封面，不报错打扰） */
  static async start(
    assetId: string,
    shortEdge: number,
    onFrame: (f: AnimFrame) => void,
  ): Promise<AnimStreamSession | null> {
    let info: { session: number; port: number };
    try {
      info = await invoke<{ session: number; port: number }>("start_anim_stream", { assetId, shortEdge });
    } catch {
      return null;
    }
    if (!info.port) return null; // WS server 未起（初始化失败）：同样回退
    const s = new AnimStreamSession(info.session, onFrame);
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

  /** 定位到目标帧（预览播放条拖动 / 逐帧步进）：幂等，失败静默 */
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
      if (buf.byteLength < 20) return;
      const dv = new DataView(buf);
      const w = dv.getUint32(0, true);
      const h = dv.getUint32(4, true);
      const frameIndex = dv.getUint32(8, true);
      const seq = Number(dv.getBigUint64(12, true));
      this.onFrame({ w, h, frameIndex, seq, rgba: new Uint8ClampedArray(buf, 20) });
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
