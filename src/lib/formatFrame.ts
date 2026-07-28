// 视频帧号换算（5.3 帧化改造，2026-07-28 用户定：帧数优先——视频播放条 / 悬停胶囊 / 笔记
// 以帧号显示，按素材自身帧率）。帧号口径与帧播放条（PlayBarFrames）对齐：1 起始，"x / y"。
// fps 缺失时调用方回退时间显示（formatDuration），本文件不处理

/** 总帧数：时长 × 帧率四舍五入，至少 1 */
export function totalFrames(duration: number, fps: number): number {
  return Math.max(1, Math.round(Math.max(0, duration) * fps));
}

/** 当前帧号（1 起始）：floor 取所在帧区间，钳到 [1, total]（片尾边界不越界） */
export function frameOfTime(secs: number, fps: number, total: number): number {
  const n = Math.floor(Math.max(0, secs) * fps) + 1;
  return Math.min(Math.max(1, n), Math.max(1, total));
}

/** "当前帧 / 总帧数"（播放条用，与 PlayBarFrames 的 "x / y" 同格式） */
export function formatFrameRange(secs: number, duration: number, fps: number): string {
  const total = totalFrames(duration, fps);
  return `${frameOfTime(secs, fps, total)} / ${total}`;
}
