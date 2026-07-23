// 格式角标文字色映射（01_样式 token 01-5）：各软件品牌色，未列出的格式回退 C4-70 通用色。
// 角标底色恒为 C1-80，仅文字带色（节制用色，2026-07-23 用户定）。

/** 各软件品牌色格式（扩展名小写 → token 变量） */
const BRAND_COLORS: Record<string, string> = {
  psd: "var(--fmt-psd)",
  psb: "var(--fmt-psd)",
  ai: "var(--fmt-ai)",
  fla: "var(--fmt-fla)",
  aep: "var(--fmt-aep)",
  svg: "var(--fmt-svg)",
};

/** 图片格式（与序列帧）统一一色 */
const IMAGE_EXTS = new Set(["png", "jpg", "jpeg", "bmp", "gif", "webp"]);

/** 角标文字色：序列帧与图片格式统一 --fmt-image，品牌格式按映射，其余 C4-70 */
export function formatBadgeColor(ext: string, isSeq = false): string {
  if (isSeq) return "var(--fmt-image)";
  const key = ext.toLowerCase();
  if (IMAGE_EXTS.has(key)) return "var(--fmt-image)";
  return BRAND_COLORS[key] ?? "var(--c4-70)";
}
