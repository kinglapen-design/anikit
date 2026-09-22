// 字体卡片懒加载（2026-09-22 R6 卡片三变体接线 · font 变体，稿 = 文件卡片-字体）：
// 字体卡内容区 = 示例字块（FontContent）以该字体真渲染——数据两路：fontInfo（style 简介行 /
// installed 安装钮 / hasCjk 示例字分流）+ FontFace 注册家族名。
// FontFace 会话级注册常驻（family = anikit-card-font-{assetId}，与预览面板 anikit-font- 命名
// 空间隔离——预览卸载 fonts.delete 不误伤卡片）：卡片滚出视口不回收，网格来回滚动重载字体的
// 开销大于常驻内存（Eagle 同全加载口径）。加载失败（损坏字体）回退空 family = 系统字体渲染。
// 缓存口径同 cardText / cardPeaks：会话级 Map + modifiedAt 版本戳 + inflight 去重 + 失败缓存。
import { convertFileSrc } from "@tauri-apps/api/core";
import { getFontInfo, type FontInfo } from "./fontOps";
import type { PlacementView } from "../stores/content";

export interface CardFont {
  info: FontInfo;
  /** FontFace 注册成功的家族名；"" = 加载失败回退系统字体 */
  family: string;
}

interface CacheEntry {
  modifiedAt: string | null;
  font: CardFont | null;
}

const cache = new Map<string, CacheEntry>();
const inflight = new Map<string, Promise<CardFont | null>>();

/** 懒拉字体卡数据（fontInfo + FontFace 家族名；缓存命中且版本一致直给，失败回退 null 并缓存） */
export function cardFontFor(asset: Pick<PlacementView, "assetId" | "modifiedAt">): Promise<CardFont | null> {
  const id = asset.assetId;
  const hit = cache.get(id);
  if (hit && hit.modifiedAt === asset.modifiedAt) return Promise.resolve(hit.font);
  const flying = inflight.get(id);
  if (flying) return flying;
  const p = load(asset)
    .then((font) => {
      cache.set(id, { modifiedAt: asset.modifiedAt, font });
      return font;
    })
    .catch(() => {
      cache.set(id, { modifiedAt: asset.modifiedAt, font: null });
      return null;
    })
    .finally(() => inflight.delete(id));
  inflight.set(id, p);
  return p;
}

/** 安装态写回缓存（卡片安装钮切换后同步，防滚出视口复用后读旧态） */
export function cardFontSetInstalled(assetId: string, installed: boolean): void {
  const hit = cache.get(assetId);
  if (hit?.font) hit.font = { ...hit.font, info: { ...hit.font.info, installed } };
}

async function load(asset: Pick<PlacementView, "assetId">): Promise<CardFont | null> {
  const info = await getFontInfo(asset.assetId);
  const family = `anikit-card-font-${asset.assetId}`;
  // 幂等：同 family 已注册不重复加载（FontFace 不可枚举查重，以 cache 命中为已注册证据——
  // 走到此即缓存未命中 = 未注册或版本过期，重复 new FontFace 覆盖同名注册亦无害）
  try {
    const face = new FontFace(family, `url("${convertFileSrc(info.originPath)}")`);
    await face.load();
    document.fonts.add(face);
    return { info, family };
  } catch {
    return { info, family: "" };
  }
}
