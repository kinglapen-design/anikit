// 时长格式化（5.1 音视频/动图卡片标签）：< 1 小时 mm:ss（分钟两位补零，Eagle 同格式
// ——"00:03"、"01:42"，2026-07-26 用户截图定），≥ 1 小时 h:mm:ss；秒四舍五入
export function formatDuration(secs: number): string {
  const total = Math.max(0, Math.round(secs));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  const mm = String(m).padStart(2, "0");
  return `${h > 0 ? `${h}:` : ""}${mm}:${String(s).padStart(2, "0")}`;
}
