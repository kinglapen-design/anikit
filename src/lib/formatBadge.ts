// 格式角标色映射（2026-08-10 用户定：弃旧 7 组实色，改用自定义颜色 9 色前景值）：
// 深色底 + 徽标档常态字；静态图片（PNG / JPG / BMP / WEBP）返回 null = 不出角标
//（图片内容本身就是格式说明）；映射理解：Photoshop 蓝 / Animate 桃红 / Illustrator 橙 /
// After Effects 蓝紫 / 动图组（GIF·动图·序列帧·视频）紫 / 音频绿 / 其他黄绿。

/** 各软件 / 动图分组（扩展名小写 → token 变量） */
const BRAND_COLORS: Record<string, string> = {
  psd: "var(--custom-blue)",
  psb: "var(--custom-blue)",
  fla: "var(--custom-peach)",
  svg: "var(--custom-orange)",
  eps: "var(--custom-orange)",
  ai: "var(--custom-orange)",
  aep: "var(--custom-blue-violet)",
  gif: "var(--custom-purple)",
  // 视频归动图组（会动的归一组，5.1）
  mp4: "var(--custom-purple)",
  mov: "var(--custom-purple)",
  avi: "var(--custom-purple)",
  mpeg4: "var(--custom-purple)",
  m4v: "var(--custom-purple)",
  // 音频绿（2026-08-10 九色化，原归"其他"中性组）
  mp3: "var(--custom-green)",
  wav: "var(--custom-green)",
  wma: "var(--custom-green)",
};

/** 未列出格式（zip / docx 等）：黄绿（九色中最低调，不抢眼） */
const OTHER_COLOR = "var(--custom-yellow-green)";

/** 静态图片格式：不出角标 */
const IMAGE_EXTS = new Set(["png", "jpg", "jpeg", "bmp", "webp"]);

/** 音频格式（5.1）：列表"尺寸"列改显示时长（metadata 宽高是波形图尺寸，不是内容尺寸） */
const AUDIO_EXTS = new Set(["mp3", "wav", "wma"]);

/** 视频格式（5.1/5.2，PRD 05-01 MVP 口径）：预览走 <video> 播放 */
const VIDEO_EXTS = new Set(["mp4", "mov", "avi", "mpeg4", "m4v"]);

/** 音频判定（5.1，PRD 05-01 MVP 口径） */
export function isAudioExt(ext: string): boolean {
  return AUDIO_EXTS.has(ext.toLowerCase());
}

/** 视频判定（5.2 预览 kind 分派） */
export function isVideoExt(ext: string): boolean {
  return VIDEO_EXTS.has(ext.toLowerCase());
}

/** 静态图片判定（4.4 文件夹卡片缩略图取子项同口径：排除 GIF / 序列帧 / 工程与矢量格式） */
export function isStaticImageExt(ext: string): boolean {
  return IMAGE_EXTS.has(ext.toLowerCase());
}

/** 角标底色：null = 不显示角标；animated（WEBP 动图，5.1）覆盖静态图片归动图组 */
export function formatBadgeColor(ext: string, isSeq = false, animated = false): string | null {
  if (isSeq || animated) return "var(--custom-purple)";
  const key = ext.toLowerCase();
  if (IMAGE_EXTS.has(key)) return null;
  return BRAND_COLORS[key] ?? OTHER_COLOR;
}
