// 文本卡片正文懒加载（2026-09-22 R6 卡片三变体接线 · text 变体，稿 = 文件卡片-txt-md）：
// txt / md / docx 卡内容 = 词干标题 + 正文滚动预览。虚拟滚动只渲染可见卡 → onMounted 触发即
// 视口懒加载；会话级缓存 key = assetId，附带 modifiedAt 版本——编辑保存 / 设封面等任何
// modifiedAt 变化后自动重拉（免手动失效登记）；inflight 去重防同卡并发。
// 截断 8000 字符：卡片滚动浏览远超够用，全量灌 DOM 会拖垮渲染（2MB 文本上限场景）。
// docx 卡片复用 read_docx_asset 全量解析——文件小（常规几十 KB）时无感，
// 大文档卡片解析浪费记待办观察项（必要时后端加「读头部」轻命令）。
// 工作空间分支（2026-09-26 填充卡对接）：条目带 wsPath（绝对路径）时走 by-path 命令
//（read_text_by_path / read_docx_by_path，越界校验在后端）——assetId = "ws:"+rel 合成假 id
// 严禁发后端，仅作前端缓存键（纯前端键允许）；版本戳 = modifiedAt（ws 条目由 mtimeMs 转来）。
import { readDocxAsset, readDocxByPath } from "./docxOps";
import { readTextAsset, readTextByPath } from "./textOps";
import { isDocxExt, isTextExt } from "./formatBadge";
import type { PlacementView } from "../stores/content";

/// 卡片正文截断上限（字符）
const MAX_CARD_CHARS = 8000;

interface CacheEntry {
  modifiedAt: string | null;
  text: string;
}

const cache = new Map<string, CacheEntry>();
const inflight = new Map<string, Promise<string>>();

/** 判定该资产是否文本卡（txt / md / docx） */
export function isTextCard(asset: Pick<PlacementView, "ext">): boolean {
  const ext = asset.ext.toLowerCase();
  return isTextExt(ext) || isDocxExt(ext);
}

/** 懒拉卡片正文（缓存命中且版本一致直给；失败回退空串并缓存，modifiedAt 变化后自然重试） */
export function cardTextFor(asset: Pick<PlacementView, "assetId" | "ext" | "modifiedAt" | "wsPath">): Promise<string> {
  const id = asset.assetId;
  const hit = cache.get(id);
  if (hit && hit.modifiedAt === asset.modifiedAt) return Promise.resolve(hit.text);
  const flying = inflight.get(id);
  if (flying) return flying;
  const p = fetchText(asset)
    .then((text) => {
      cache.set(id, { modifiedAt: asset.modifiedAt, text });
      return text;
    })
    .catch(() => {
      cache.set(id, { modifiedAt: asset.modifiedAt, text: "" });
      return "";
    })
    .finally(() => inflight.delete(id));
  inflight.set(id, p);
  return p;
}

async function fetchText(asset: Pick<PlacementView, "assetId" | "ext" | "wsPath">): Promise<string> {
  const ext = asset.ext.toLowerCase();
  const wsPath = asset.wsPath ?? null;
  let full: string;
  if (isDocxExt(ext)) {
    const r = wsPath ? await readDocxByPath(wsPath) : await readDocxAsset(asset.assetId);
    full = r.paragraphs.map((p) => p.runs.map((run) => run.text).join("")).join("\n");
  } else if (isTextExt(ext)) {
    full = wsPath ? (await readTextByPath(wsPath)).content : (await readTextAsset(asset.assetId)).content;
  } else {
    return "";
  }
  return full.length > MAX_CARD_CHARS ? full.slice(0, MAX_CARD_CHARS) : full;
}
