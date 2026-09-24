// 音频播放条 spec（旧 PlayBarAudio 删减口径）：无 快退/快进/逐帧/笔记/设封面/全屏（无画面）；
// 进度区 = 竖条波形（wave 标志——音乐预览稿 prose「只在播放条上放波形图即可」；2026-09-25 竖条化
// 后 peaks 由 WaveProgress 自拉 cardPeaksFor，spec 只标族不供数）；时间码恒 mm:ss（音频无 fps 天然回退）。
import { usePreviewStore } from "../../stores/preview";
import { usePreviewMediaStore } from "../../stores/previewMedia";
import { formatDuration } from "../formatDuration";
import type { PlayBarSpec } from "./types";

export function makeAudioSpec(): PlayBarSpec {
  const preview = usePreviewStore();
  const media = usePreviewMediaStore();
  return {
    show: ["volume", "rate", "loop"],
    playing: () => media.playing,
    disabled: () => media.transcodingSlow,
    progressGet: () => (media.duration > 0 ? media.currentTime / media.duration : 0),
    progressSet: (p) => {
      if (media.duration > 0) media.seek(p * media.duration);
    },
    tcCurr: () => formatDuration(media.currentTime),
    tcTotal: () => formatDuration(media.duration),
    rate: () => media.rate,
    setRate: (r) => media.setRate(r),
    togglePlay: () => media.togglePlay(),
    muted: () => media.muted,
    volumeGet: () => (media.muted ? 0 : media.volume),
    volumeSet: (v) => media.setVolume(v),
    toggleMute: () => media.toggleMute(),
    loopOn: () => media.loop,
    toggleLoop: () => media.toggleLoop(),
    fullscreen: () => preview.fullscreen,
    toggleFullscreen: () => preview.setFullscreen(!preview.fullscreen),
    dragStart: () => media.setScrubbing(true),
    dragEnd: () => media.setScrubbing(false),
    wave: true,
  };
}
