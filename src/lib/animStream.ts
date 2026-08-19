// 动图帧流客户端（阶段四·自研解码器）：start_anim_stream 拿会话 → WS 收帧 → 组件渲染。
// 帧格式 = [w u32le][h u32le][seq u64le][rgba...]（services/frame_stream 推送，320p RGBA）。
// 一卡一会话；失败（非动图 / 上限 / 解码失败 / WS 不通）一律 null——调用方回退静态封面。
import { invoke } from "@tauri-apps/api/core";

export interface AnimFrame {
  w: number;
  h: number;
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

  /** 启动会话并连 WS；失败返回 null（回退静态封面，不报错打扰） */
  static async start(assetId: string, onFrame: (f: AnimFrame) => void): Promise<AnimStreamSession | null> {
    let info: { session: number; port: number };
    try {
      info = await invoke<{ session: number; port: number }>("start_anim_stream", { assetId });
    } catch {
      return null;
    }
    if (!info.port) return null; // WS server 未起（初始化失败）：同样回退
    const s = new AnimStreamSession(info.session, onFrame);
    s.connect(info.port);
    return s;
  }

  private connect(port: number): void {
    const ws = new WebSocket(`ws://127.0.0.1:${port}/${this.session}`);
    ws.binaryType = "arraybuffer";
    ws.onmessage = (ev) => {
      if (this.stopped || !(ev.data instanceof ArrayBuffer)) return;
      const buf = ev.data;
      if (buf.byteLength < 16) return;
      const dv = new DataView(buf);
      const w = dv.getUint32(0, true);
      const h = dv.getUint32(4, true);
      const seq = Number(dv.getBigUint64(8, true));
      this.onFrame({ w, h, seq, rgba: new Uint8ClampedArray(buf, 16) });
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
