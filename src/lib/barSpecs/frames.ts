// 帧播放条 spec（序列帧 / 动图同工厂，旧 PlayBarFrames + PreviewFrameStrip 业务口径迁移）：
// 控件去 transport/step/volume/loop（上/下帧归帧条两端尖括号钮、动图无音量、帧播放无循环钮）；
// 进度 = 帧索引 /（总数-1）；时间码恒帧号；动图帧流回退态（animTotal=0）由宿主判定退场。
// 帧条数据：序列帧 = 帧清单 stripPath；动图 = animStrips 渐进填充（未就绪 null 空槽）。
// per-panel 会话化（2026-09-25）：preview 会话由工厂入参闭包捕获（属主 panel），不再抓全局单例
import { convertFileSrc } from "@tauri-apps/api/core";
import type { PreviewSession } from "../../stores/preview";
import { segmentFrameCount } from "../semantic";
import type { PlayBarSpec } from "./types";

export function makeFramesSpec(preview: PreviewSession): PlayBarSpec {
  const count = (): number => preview.segment
    ? segmentFrameCount(preview.segment)
    : preview.kind === "anim" ? preview.animTotal : (preview.seq?.frames.length ?? 0);
  const start = (): number => preview.segment?.startFrame ?? 0;
  const end = (): number => preview.segment?.endFrame ?? Math.max(0, count() - 1);
  return {
    show: ["rate", "note", "cover", "fullscreen"],
    playing: () => preview.playing,
    disabled: () => false,
    progressGet: () => (end() > start() ? (preview.frame - start()) / (end() - start()) : 0),
    progressSet: (p) => {
      if (count() > 0) preview.setFrame(start() + Math.round(p * (end() - start())));
    },
    tcCurr: () => String(preview.frame + 1),
    tcTotal: () => String(preview.segment ? end() + 1 : count()),
    rate: () => preview.rate,
    setRate: (r) => preview.setRate(r),
    togglePlay: () => preview.togglePlay(),
    setCover: () => preview.setAsThumb(),
    fullscreen: () => preview.fullscreen,
    toggleFullscreen: () => preview.setFullscreen(!preview.fullscreen),
    frames: {
      count,
      thumb: (i) => {
        const sourceIndex = start() + i;
        if (preview.kind === "anim") {
          const p = preview.animStrips[`f:${sourceIndex}`];
          return p ? convertFileSrc(p) : null;
        }
        const f = preview.seq?.frames[sourceIndex];
        return f?.stripPath ? convertFileSrc(f.stripPath) : null;
      },
      frame: () => preview.frame - start(),
      setFrame: (i) => preview.setFrame(start() + i),
      stepFrame: (d) => preview.stepFrame(d),
    },
  };
}
