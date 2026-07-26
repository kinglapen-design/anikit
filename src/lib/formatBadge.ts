// 格式角标色映射（01_样式 token 01-6，2026-07-23 降噪定案）：深色底 + 60% 字。
// 静态图片（PNG / JPG / BMP / WEBP）返回 null = 不出角标（图片内容本身就是格式说明）；
// 序列帧 / GIF / WEBP 动图（5.1）/ 视频（5.1）归动图组；音频归"其他"组；未列出格式用 C1-40。

/** 各软件 / 动图分组（扩展名小写 → token 变量） */
const BRAND_COLORS: Record<string, string> = {
  psd: "var(--fmt-psd)",
  psb: "var(--fmt-psd)",
  fla: "var(--fmt-fla)",
  svg: "var(--fmt-svg)",
  eps: "var(--fmt-svg)",
  ai: "var(--fmt-svg)",
  aep: "var(--fmt-aep)",
  gif: "var(--fmt-anim)",
  // 视频归动图组（会动的归一组，5.1）；音频不列 = 落默认"其他"组
  mp4: "var(--fmt-anim)",
  mov: "var(--fmt-anim)",
  avi: "var(--fmt-anim)",
  mpeg4: "var(--fmt-anim)",
  m4v: "var(--fmt-anim)",
};

/** 静态图片格式：不出角标 */
const IMAGE_EXTS = new Set(["png", "jpg", "jpeg", "bmp", "webp"]);

/** 静态图片判定（4.4 文件夹卡片缩略图取子项同口径：排除 GIF / 序列帧 / 工程与矢量格式） */
export function isStaticImageExt(ext: string): boolean {
  return IMAGE_EXTS.has(ext.toLowerCase());
}

/** 角标底色：null = 不显示角标；animated（WEBP 动图，5.1）覆盖静态图片归动图组 */
export function formatBadgeColor(ext: string, isSeq = false, animated = false): string | null {
  if (isSeq || animated) return "var(--fmt-anim)";
  const key = ext.toLowerCase();
  if (IMAGE_EXTS.has(key)) return null;
  return BRAND_COLORS[key] ?? "var(--c1-40)";
}
