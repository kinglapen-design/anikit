// 格式角标色映射（2026-08-10 用户定：弃旧 7 组实色，改用自定义颜色 9 色前景值）：
// 深色底 + 徽标档常态字；静态图片（PNG / JPG / BMP / WEBP）返回 null = 不出角标
//（图片内容本身就是格式说明）；映射理解：Photoshop 蓝 / Animate 桃红 / Illustrator 橙 /
// After Effects 蓝紫 / 动图组（GIF·动图·序列帧·视频）紫 / 音频绿 / 其他黄绿。
// 附：isSequence / displayName 派生工具同驻本文件（卡片 / 列表 / 浮层 / 菜单共用口径）。
import type { PlacementView } from "../stores/content";

/** 各软件 / 动图分组（扩展名小写 → token 变量） */
const BRAND_COLORS: Record<string, string> = {
  psd: "var(--n-custom-blue-2)",
  psb: "var(--n-custom-blue-2)",
  fla: "var(--n-custom-peach-2)",
  svg: "var(--n-custom-yellow-2)",
  eps: "var(--n-custom-yellow-2)",
  ai: "var(--n-custom-yellow-2)",
  aep: "var(--n-custom-purple-2)",
  gif: "var(--n-custom-purple-2)",
  // 视频归动图组（会动的归一组，5.1）
  mp4: "var(--n-custom-purple-2)",
  mov: "var(--n-custom-purple-2)",
  avi: "var(--n-custom-purple-2)",
  mpeg4: "var(--n-custom-purple-2)",
  m4v: "var(--n-custom-purple-2)",
  // 音频绿（2026-08-10 九色化，原归"其他"中性组）
  mp3: "var(--n-custom-green-2)",
  wav: "var(--n-custom-green-2)",
  wma: "var(--n-custom-green-2)",
};

/** 未列出格式（zip / docx 等）：黄绿（九色中最低调，不抢眼） */
const OTHER_COLOR = "var(--n-custom-yellowgreen-2)";

/** 静态图片格式：不出角标 */
const IMAGE_EXTS = new Set(["png", "jpg", "jpeg", "bmp", "webp"]);

/** 音频格式（5.1）：列表"尺寸"列改显示时长（metadata 宽高是波形图尺寸，不是内容尺寸） */
const AUDIO_EXTS = new Set(["mp3", "wav", "wma"]);

/** 视频格式（5.1/5.2，PRD 05-01 MVP 口径）：预览走 <video> 播放 */
const VIDEO_EXTS = new Set(["mp4", "mov", "avi", "mpeg4", "m4v"]);

/** 字体格式（5.6）：预览走字体页（演示文本 + 元数据 + 系统级安装） */
const FONT_EXTS = new Set(["ttf", "otf", "ttc"]);

/** 纯文本格式（5.6）：预览走文本页（txt = APP 内编辑，md = markdown-it 只读渲染） */
const TEXT_EXTS = new Set(["txt", "md"]);

/** 音频判定（5.1，PRD 05-01 MVP 口径） */
export function isAudioExt(ext: string): boolean {
  return AUDIO_EXTS.has(ext.toLowerCase());
}

/** 视频判定（5.2 预览 kind 分派） */
export function isVideoExt(ext: string): boolean {
  return VIDEO_EXTS.has(ext.toLowerCase());
}

/** 字体判定（5.6 预览 kind 分派） */
export function isFontExt(ext: string): boolean {
  return FONT_EXTS.has(ext.toLowerCase());
}

/** 纯文本判定（5.6 预览 kind 分派：txt/md） */
export function isTextExt(ext: string): boolean {
  return TEXT_EXTS.has(ext.toLowerCase());
}

/** 静态图片判定（4.4 文件夹卡片缩略图取子项同口径：排除 GIF / 序列帧 / 工程与矢量格式） */
export function isStaticImageExt(ext: string): boolean {
  return IMAGE_EXTS.has(ext.toLowerCase());
}

/** 可作为伴生缩略图的格式（静态图 + gif 动图）——前端镜像后端 asset_menu.rs `is_thumb_capable`。
 *  「转换为缩略图链接并复制」按此条件出现（2026-09-05 用户定：不能作缩略图的格式不列该项，点了报错是误导） */
export function isThumbCapableExt(ext: string): boolean {
  const k = ext.toLowerCase();
  return IMAGE_EXTS.has(k) || k === "gif";
}

/** 角标底色：null = 不显示角标；animated（WEBP 动图，5.1）覆盖静态图片归动图组 */
export function formatBadgeColor(ext: string, isSeq = false, animated = false): string | null {
  if (isSeq || animated) return "var(--n-custom-purple-2)";
  const key = ext.toLowerCase();
  if (IMAGE_EXTS.has(key)) return null;
  return BRAND_COLORS[key] ?? OTHER_COLOR;
}

/** 序列帧判定（2026-07-22 用户定）：frameCount 非空且非动图（动图同带 frameCount 字段，靠 animated 区分） */
export function isSequence(asset: Pick<PlacementView, "frameCount" | "animated">): boolean {
  return asset.frameCount != null && !asset.animated;
}

/** 卡片 / 列表显示名：序列帧 = 词干 + .序列帧 后缀（仅显示层，数据不动）；
 *  28 查看方式"显示扩展名"（1.4）：关 = 名称只显示词干 */
export function displayName(
  asset: Pick<PlacementView, "name" | "frameCount" | "animated">,
  showExt: boolean,
): string {
  const stem = asset.name.replace(/\.[^.]+$/, "");
  if (isSequence(asset)) return `${stem}.序列帧`;
  return showExt ? asset.name : stem;
}
