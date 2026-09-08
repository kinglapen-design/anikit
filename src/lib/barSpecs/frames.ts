// 帧播放条 spec（序列帧 / 动图同工厂，旧 PlayBarFrames + PreviewFrameStrip 业务口径迁移）：
// 控件去 transport/step/volume/loop（上/下帧归帧条两端尖括号钮、动图无音量、帧播放无循环钮）；
// 进度 = 帧索引 /（总数-1）；时间码恒帧号；动图帧流回退态（animTotal=0）由宿主判定退场。
// 帧条数据：序列帧 = 帧清单 stripPath；动图 = animStrips 渐进填充（未就绪 null 空槽）。
import { convertFileSrc } from "@tauri-apps/api/core";
import { usePreviewStore } from "../../stores/preview";
import type { PlayBarSpec } from "./types";

export function makeFramesSpec(): PlayBarSpec {
  const preview = usePreviewStore();
  const count = (): number =>
    preview.kind === "anim" ? preview.animTotal : (preview.seq?.frames.length ?? 0);
  return {
    show: ["rate", "note", "cover", "fullscreen"],
    playing: () => preview.playing,
    disabled: () => false,
    progressGet: () => (count() > 1 ? preview.frame / (count() - 1) : 0),
    progressSet: (p) => {
      if (count() > 0) preview.setFrame(Math.round(p * (count() - 1)));
    },
    tcCurr: () => String(preview.frame + 1),
    tcTotal: () => String(count()),
    rate: () => preview.rate,
    setRate: (r) => preview.setRate(r),
    togglePlay: () => preview.togglePlay(),
    setCover: () => preview.setAsThumb(),
    fullscreen: () => preview.fullscreen,
    toggleFullscreen: () => preview.setFullscreen(!preview.fullscreen),
    frames: {
      count,
      thumb: (i) => {
        if (preview.kind === "anim") {
          const p = preview.animStrips[`f:${i}`];
          return p ? convertFileSrc(p) : null;
        }
        const f = preview.seq?.frames[i];
        return f?.stripPath ? convertFileSrc(f.stripPath) : null;
      },
      frame: () => preview.frame,
      setFrame: (i) => preview.setFrame(i),
      stepFrame: (d) => preview.stepFrame(d),
    },
  };
}
