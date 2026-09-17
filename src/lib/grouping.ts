// 分组（2026-08-19，3.5 二轮 / PRD 02-04 / 04-07）：**显示层归组计算**——规则存 folders.json
// 文件夹节点，归组不改任何数据；仅文件夹 / 档案视图生效（快速访问无分组，用户定）。
// 归组语义（用户定）：组内多规则 AND 全命中；多组同时命中归**命中关键字（值）更多者**（更具体——
// 2026-09-17 用户实机修订：旧"规则条数更多者"在规则数相同、名称词数不同时误判——P1 vs P1_2K 案：
// 组一 名称{P1}、组二 名称{P1,2K}，P1_02_2K 命中词 1 vs 2，应归组二），平局按分组定义顺序先建者优先；
// 未命中任何组 = 未分组（在头）。
// 组内顺序 = 输入流相对序（placementsFor 输出已含新增区置顶 + 排序键，stable 分割自然带入组内）。
import type { PlacementView } from "../stores/content";
import { isAudioExt, isStaticImageExt, isVideoExt } from "./formatBadge";

// —— 数据形状（与后端 core/models.rs GroupDef / GroupRule 对应，camelCase） ——
export type GroupRuleKind = "ext" | "type" | "name" | "importDate" | "modifyDate" | "tags";
export interface GroupRule {
  kind: GroupRuleKind;
  values: string[];
}
export interface GroupDef {
  id: string;
  name: string;
  rules: GroupRule[];
  /** 自由组成员（assetIds；2026-09-17 无规则空组）：rules 为空 = 自由组——手动归组、
   *  移动语义唯一归属；规则组无此字段（两态不混） */
  members?: string[];
}

export const RULE_KIND_LABELS: Record<GroupRuleKind, string> = {
  ext: "扩展名",
  type: "类型",
  name: "名称",
  importDate: "导入日期",
  modifyDate: "修改日期",
  tags: "标签",
};
export const RULE_KIND_ORDER: readonly GroupRuleKind[] = ["ext", "type", "name", "importDate", "modifyDate", "tags"];

// —— 类型 8 类（2026-08-19 用户定，贴 formatBadge 口径） ——
export type GroupType = "image" | "anim" | "video" | "audio" | "text" | "doc" | "project" | "other";
export const GROUP_TYPE_LABELS: Record<GroupType, string> = {
  image: "图片",
  anim: "动图",
  video: "视频",
  audio: "音频",
  text: "文本",
  doc: "文档",
  project: "工程",
  other: "其他",
};
export const GROUP_TYPE_ORDER: readonly GroupType[] = ["image", "anim", "video", "audio", "text", "doc", "project", "other"];

const TEXT_EXTS = new Set(["md", "txt"]);
const DOC_EXTS = new Set(["doc", "docx", "xls", "xlsx", "ppt", "pptx", "pdf"]);
const PROJECT_EXTS = new Set(["psd", "psb", "ai", "eps", "svg", "fla", "aep"]);

/** 素材 → 类型 8 类（动图 = GIF/WEBP 动图 + 序列帧，与卡片角标"会动的归一组"同口径） */
export function groupTypeOf(p: Pick<PlacementView, "ext" | "frameCount" | "animated">): GroupType {
  if (p.frameCount != null || p.animated) return "anim";
  const ext = p.ext.toLowerCase();
  if (isStaticImageExt(ext)) return "image";
  if (isVideoExt(ext)) return "video";
  if (isAudioExt(ext)) return "audio";
  if (TEXT_EXTS.has(ext)) return "text";
  if (DOC_EXTS.has(ext)) return "doc";
  if (PROJECT_EXTS.has(ext)) return "project";
  return "other";
}

// —— 日期分档六档（02-04：互斥——一个文件只落一档；本周以周一为起点） ——
export type DateBucket = "today" | "yesterday" | "earlierThisWeek" | "lastWeek" | "earlierThisMonth" | "earlier";
export const DATE_BUCKET_LABELS: Record<DateBucket, string> = {
  today: "今天",
  yesterday: "昨天",
  earlierThisWeek: "本周早些时候",
  lastWeek: "上周",
  earlierThisMonth: "本月早些时候",
  earlier: "更早",
};
export const DATE_BUCKET_ORDER: readonly DateBucket[] = [
  "today",
  "yesterday",
  "earlierThisWeek",
  "lastWeek",
  "earlierThisMonth",
  "earlier",
];

/** RFC3339 本地时间串 → 日期档（createdAt 为 null 的调用方不应传入——创建时间不做分组键） */
export function dateBucket(iso: string, now: Date = new Date()): DateBucket {
  const t = new Date(iso);
  if (Number.isNaN(t.getTime())) return "earlier";
  const day0 = (d: Date): number => new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
  const today = day0(now);
  const yesterday = today - 86400000;
  // 本周一 00:00（getDay：周日 = 0 → 本周一 = 今天 -（6 天前））
  const dow = (now.getDay() + 6) % 7;
  const weekStart = today - dow * 86400000;
  const lastWeekStart = weekStart - 7 * 86400000;
  const monthStart = new Date(now.getFullYear(), now.getMonth(), 1).getTime();
  const ts = day0(t);
  if (ts >= today) return "today";
  if (ts >= yesterday) return "yesterday";
  if (ts >= weekStart) return "earlierThisWeek";
  if (ts >= lastWeekStart) return "lastWeek";
  if (ts >= monthStart) return "earlierThisMonth";
  return "earlier";
}

// —— 归组计算 ——

/** 单规则命中判定：ext / type = 值集含项；name = 分词 AND（每词包含于名称，不区分大小写）；
 *  importDate / modifyDate = 命中间任一档（OR）；tags = 标签词 AND（资产须全带，精确匹配——
 *  2026-09-17，PRD 02-04）。空 values = 恒不命中（后端保存前已拦，防御） */
function ruleHits(p: PlacementView, rule: GroupRule): boolean {
  if (rule.values.length === 0) return false;
  switch (rule.kind) {
    case "ext":
      return rule.values.includes(p.ext.toLowerCase());
    case "type":
      return rule.values.includes(groupTypeOf(p));
    case "name": {
      const name = p.name.toLowerCase();
      return rule.values.every((w) => w.trim() !== "" && name.includes(w.trim().toLowerCase()));
    }
    case "importDate":
      return rule.values.includes(dateBucket(p.importedAt));
    case "modifyDate":
      return rule.values.includes(dateBucket(p.modifiedAt));
    case "tags":
      return rule.values.every((w) => w.trim() !== "" && p.tags.includes(w.trim()));
  }
}

/** 单规则命中权重 = 命中的值（关键字）个数：ext / type / 日期 = 单值域命中恒计 1；
 *  name / tags = 命中词数（AND 语义下命中即全词，显式计数写法与复值域同口径） */
function ruleWeight(p: PlacementView, rule: GroupRule): number {
  if (rule.kind === "name") {
    const name = p.name.toLowerCase();
    return rule.values.filter((w) => w.trim() !== "" && name.includes(w.trim().toLowerCase())).length;
  }
  if (rule.kind === "tags") {
    return rule.values.filter((w) => w.trim() !== "" && p.tags.includes(w.trim())).length;
  }
  return ruleHits(p, rule) ? 1 : 0;
}

/** 资产归组：**手动成员 > 规则**（2026-09-17 自由组定案：显式动作胜）——先在自由组
 * （rules 为空）members 里命中即归该组（定义顺序先者；移动语义实际保证唯一）；
 * 否则 AND 全命中的规则组里取**命中关键字（值）总数**最多者；平局按定义顺序先者（groups 数组序）；
 *  全不命中 = null（未分组） */
export function matchGroup(p: PlacementView, groups: readonly GroupDef[]): GroupDef | null {
  for (const g of groups) {
    if (g.rules.length === 0 && g.members?.includes(p.assetId)) return g;
  }
  let best: GroupDef | null = null;
  let bestW = 0;
  for (const g of groups) {
    if (g.rules.length === 0) continue;
    if (!g.rules.every((r) => ruleHits(p, r))) continue;
    const w = g.rules.reduce((sum, r) => sum + ruleWeight(p, r), 0);
    if (best === null || w > bestW) {
      best = g;
      bestW = w;
    }
  }
  return best;
}

export interface GroupSection<T> {
  /** null = 未分组（恒在头，2026-08-19 用户定） */
  groupId: string | null;
  name: string;
  items: T[];
  /** true = 自由组（2026-09-17 无规则空组）：**空组恒渲染**（小高度 + 「将文件拖入分组」提示，
   *  供拖入落点）；规则组空组不出（视觉平铺定案不动） */
  free?: boolean;
}

/**
 * 列表分组：返回 null = 不分组；否则 = 未分组（在头，非空才出）+ 各组（定义序；
 * 规则组空组不出，自由组空组恒出——2026-09-17 无规则空组）。
 * 组内保持输入流相对序（stable——新增区置顶 / 排序键效果自然带入）。
 * 两种 null 情形：① 无任何组配置；② 只有规则组且没有任何组命中（只剩"未分组"一段——
 * 2026-08-19 复验定案：0 命中组仍在管理窗口可见可编，但内容区不挂"未分组"孤标题，视觉回到平铺）。
 */
export function groupPlacements<T extends PlacementView>(list: T[], groups: readonly GroupDef[] | null | undefined): GroupSection<T>[] | null {
  if (!groups || groups.length === 0) return null;
  const byGroup = new Map<string, T[]>();
  const ungrouped: T[] = [];
  for (const p of list) {
    const g = matchGroup(p, groups);
    if (g === null) {
      ungrouped.push(p);
    } else {
      const arr = byGroup.get(g.id) ?? [];
      arr.push(p);
      byGroup.set(g.id, arr);
    }
  }
  const out: GroupSection<T>[] = [];
  if (ungrouped.length > 0) out.push({ groupId: null, name: "未分组", items: ungrouped });
  for (const g of groups) {
    if (g.rules.length === 0) {
      // 自由组：恒出（空组给拖入落点与提示；成员可为空）
      out.push({ groupId: g.id, name: g.name, items: byGroup.get(g.id) ?? [], free: true });
      continue;
    }
    const items = byGroup.get(g.id);
    if (items && items.length > 0) out.push({ groupId: g.id, name: g.name, items });
  }
  return out.some((s) => s.groupId !== null) ? out : null;
}
