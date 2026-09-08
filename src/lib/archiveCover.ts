// 档案封面 URL 链（2026-09-08 用户定）：自定义封面（coverPath）→ 子项首图（索引回退，
// folderThumbMap 既有口径）→ 默认封面图（assets/icons/03_装饰图标/档案默认封面.png——
// 索引没有可用封面时展示）。三处共用：ArchiveStrip 档案卡 / SectionPinned pinned 封面 /
// CoverSettings 封面框。
// ?v=coverVer = asset 协议同 URL 启发式缓存的版本戳（同 AssetCard 缩略图 ?v=modifiedAt 口径）：
// 覆盖式烘培同路径重写（cache/covers/{folderId}.webp），mtime 戳驱动 img 重载；
// undo 改名还原把旧文件连 mtime 一并带回 → 还原后 URL 恰与旧内容同戳，缓存命中即正确图。
import { convertFileSrc } from "@tauri-apps/api/core";
import defaultCoverUrl from "../../assets/icons/03_装饰图标/档案默认封面.png?url";

/** 档案封面 URL：无自定义封面且无子项首图 = 默认封面图（恒非空） */
export function archiveCoverUrl(
  coverPath: string | null | undefined,
  coverVer: string | null | undefined,
  childThumb: string | null | undefined,
): string {
  const p = coverPath ?? childThumb;
  if (!p) return defaultCoverUrl;
  const url = convertFileSrc(p);
  return coverPath && coverVer ? `${url}?v=${encodeURIComponent(coverVer)}` : url;
}
