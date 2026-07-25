// 格式角标色映射（01_样式 token 01-6，2026-07-23 降噪定案）：深色底 + 60% 字。
// 静态图片（PNG / JPG / BMP / WEBP）返回 null = 不出角标（图片内容本身就是格式说明）；
// 序列帧与 GIF 归动图组；其他未列出格式用 C1-40。

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

/** 静态图片格式：不出角标 */
const IMAGE_EXTS = new Set(["png", "jpg", "jpeg", "bmp", "webp"]);

/** 静态图片判定（4.4 文件夹卡片缩略图取子项同口径：排除 GIF / 序列帧 / 工程与矢量格式） */
export function isStaticImageExt(ext: string): boolean {
  return IMAGE_EXTS.has(ext.toLowerCase());
}

/** 角标底色：null = 不显示角标 */
export function formatBadgeColor(ext: string, isSeq = false): string | null {
  if (isSeq) return "var(--fmt-anim)";
  const key = ext.toLowerCase();
  if (IMAGE_EXTS.has(key)) return null;
  return BRAND_COLORS[key] ?? "var(--c1-40)";
}
