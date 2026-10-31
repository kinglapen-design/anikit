// 视频播放条 spec（旧 PlayBarVideo 业务口径全量迁移）：控件全量组；快退/快进 ∓5 秒；
// 逐帧步进 = 暂停 + ±1/fps（fps 缺失回退 24）；时间码 = 帧数优先（查看方式「时间显示 = 帧数」且 fps 可用，
// 回退 mm:ss）；进度拖动挂 scrubbing（拖动期 timeupdate 不回写）；当前画面设为预览图 = set_video_thumb。
// 工作空间磁盘实况（PRD 02 步骤 1）：cover（set_video_thumb）/ note（逐帧笔记）走 assetId 命令——
// 合成假 id 严禁下发，show 组滤除（其余控件纯前端 / 按路径，照常）
// per-panel 会话化（2026-09-25）：preview / media 会话由工厂入参闭包捕获（属主 panel），不再抓全局单例
import { useUiStore } from "../../stores/ui";
import type { PreviewSession } from "../../stores/preview";
import type { PreviewMediaSession } from "../../stores/previewMedia";
import { formatDuration } from "../formatDuration";
import { frameOfTime, totalFrames } from "../formatFrame";
import type { PlayBarSpec } from "./types";

export function makeVideoSpec(preview: PreviewSession, media: PreviewMediaSession): PlayBarSpec {
  const ui = useUiStore();
  const useFrames = (): boolean => ui.timeDisplay === "frames" && media.fps != null && media.fps > 0;
  const start = (): number => (media.segment?.startMs ?? 0) / 1000;
  const end = (): number => media.segment ? media.segment.endMs / 1000 : media.duration;
  const total = (): number => totalFrames(end(), media.fps ?? 24);
  const groups = (): PlayBarSpec["show"] =>
    preview.current?.assetId.startsWith("ws:")
      ? (["transport", "step", "volume", "rate", "loop", "fullscreen"] as const)
      : (["transport", "step", "volume", "rate", "loop", "note", "cover", "fullscreen"] as const);
  return {
    show: groups(),
    playing: () => media.playing,
    disabled: () => media.transcodingSlow,
    progressGet: () => (end() > start() ? (media.currentTime - start()) / (end() - start()) : 0),
    progressSet: (p) => {
      if (end() > start()) media.seek(start() + p * (end() - start()));
    },
    tcCurr: () =>
      useFrames() && media.fps
        ? String(frameOfTime(media.currentTime, media.fps, total()))
        : formatDuration(media.currentTime),
    tcTotal: () => (useFrames() ? String(total()) : formatDuration(end())),
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
