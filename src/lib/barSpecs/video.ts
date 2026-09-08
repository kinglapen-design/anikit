// 视频播放条 spec（旧 PlayBarVideo 业务口径全量迁移）：控件全量组；快退/快进 ∓5 秒；
// 逐帧步进 = 暂停 + ±1/fps（fps 缺失回退 24）；时间码 = 帧数优先（查看方式「时间显示 = 帧数」且 fps 可用，
// 回退 mm:ss）；进度拖动挂 scrubbing（拖动期 timeupdate 不回写）；当前画面设为预览图 = set_video_thumb。
import { useUiStore } from "../../stores/ui";
import { usePreviewStore } from "../../stores/preview";
import { usePreviewMediaStore } from "../../stores/previewMedia";
import { formatDuration } from "../formatDuration";
import { frameOfTime, totalFrames } from "../formatFrame";
import type { PlayBarSpec } from "./types";

export function makeVideoSpec(): PlayBarSpec {
  const ui = useUiStore();
  const preview = usePreviewStore();
  const media = usePreviewMediaStore();
  const useFrames = (): boolean => ui.timeDisplay === "frames" && media.fps != null && media.fps > 0;
  const total = (): number => totalFrames(media.duration, media.fps ?? 24);
  return {
    show: ["transport", "step", "volume", "rate", "loop", "note", "cover", "fullscreen"],
    playing: () => media.playing,
    disabled: () => media.transcodingSlow,
    progressGet: () => (media.duration > 0 ? media.currentTime / media.duration : 0),
    progressSet: (p) => {
      if (media.duration > 0) media.seek(p * media.duration);
    },
    tcCurr: () =>
      useFrames() && media.fps
        ? String(frameOfTime(media.currentTime, media.fps, total()))
        : formatDuration(media.currentTime),
    tcTotal: () => (useFrames() ? String(total()) : formatDuration(media.duration)),
    rate: () => media.rate,
    setRate: (r) => media.setRate(r),
    togglePlay: () => media.togglePlay(),
    seekBack: () => media.seekBy(-5),
    seekForward: () => media.seekBy(5),
    stepPrev: () => stepOneFrame(-1),
    stepNext: () => stepOneFrame(1),
    muted: () => media.muted,
    volumeGet: () => (media.muted ? 0 : media.volume),
    volumeSet: (v) => media.setVolume(v),
    toggleMute: () => media.toggleMute(),
    loopOn: () => media.loop,
    toggleLoop: () => media.toggleLoop(),
    setCover: () => media.setVideoThumb(),
    fullscreen: () => preview.fullscreen,
    toggleFullscreen: () => preview.setFullscreen(!preview.fullscreen),
    dragStart: () => media.setScrubbing(true),
    dragEnd: () => media.setScrubbing(false),
  };

  function stepOneFrame(delta: number): void {
    media.pause();
    media.seekBy(delta / (media.fps != null && media.fps > 0 ? media.fps : 24));
  }
}
