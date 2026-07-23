// 格式角标色映射（01_样式 token 01-6，2026-07-23 用户新 token 定案）：7 组实色。
// 角标样式（实底白字 / 位置右下）归组件层，过组件时统一改；本函数只负责"格式 → 颜色"。

/** 各软件 / 动图分组（扩展名小写 → token 变量） */
const BRAND_COLORS: Record<string, string> = {
  psd: "var(--fmt-psd)",
  psb: "var(--fmt-psd)",
  fla: "var(--fmt-fla)",
  svg: "var(--fmt-svg)",
  eps: "var(--fmt-svg)",
  ai: "var(--fmt-svg)",
  aep: "var(--fmt-aep)",
  gif: "var(--fmt-anim)", // WEBP 动图归此组（动图识别 5.1 才做，静态 WEBP 暂归静态图片）
};

/** 静态图片格式统一一色 */
const IMAGE_EXTS = new Set(["png", "jpg", "jpeg", "bmp", "webp"]);

/** 角标色：序列帧与 GIF 归动图组（品牌紫），静态图片统一 --fmt-image，其余未列出格式 --fmt-other */
export function formatBadgeColor(ext: string, isSeq = false): string {
  if (isSeq) return "var(--fmt-anim)";
  const key = ext.toLowerCase();
  if (IMAGE_EXTS.has(key)) return "var(--fmt-image)";
  return BRAND_COLORS[key] ?? "var(--fmt-other)";
}
