// 内容区素材排序（3.5，28 稿排序方式行业务）：**显示层排序，不动存储序**（order 字段）。
// - 自定义（"" / "custom"）= 存储序原样返回；其余键全局统一（ui.sortBy / ui.sortAsc，应用级）。
// - 名称 = 自然排序：阿拉伯数字串按数值；中文数字（一~万常见写法）解析成数值；
//   天干（甲乙丙…）按序列（2026-08-19 用户补充：中文序列不处理会按字符码乱序）。
// - 空值（无时长 / 存量无创建时间）恒排尾，不随升降序反转；平局恒按名称自然排序 + assetId 兜底（不反转）。
import type { PlacementView } from "../stores/content";

/** 排序键（ui.sortBy 取值；"" = 自定义存储序） */
export type SortKey = "" | "name" | "size" | "duration" | "ext" | "createdAt" | "modifiedAt" | "importedAt";

// —— 中文数字解析（零一二两三四五六七八九 + 十百千万单位，覆盖常见序号写法） ——
const CN_DIGITS: Record<string, number> = { 零: 0, 一: 1, 二: 2, 两: 2, 三: 3, 四: 4, 五: 5, 六: 6, 七: 7, 八: 8, 九: 9 };
const CN_UNITS: Record<string, number> = { 十: 10, 百: 100, 千: 1000, 万: 10000 };
const CN_NUM_CHARS = new Set([...Object.keys(CN_DIGITS), ...Object.keys(CN_UNITS)]);
// 天干序列（甲乙丙丁戊己庚辛壬癸）
const STEMS = "甲乙丙丁戊己庚辛壬癸";

/** 中文数字串 → 数值（"十一"=11、"一百零五"=105、"二十"=20；纯字符集由 chunkize 保证） */
export function parseCnNumber(s: string): number {
  let total = 0;
  let section = 0;
  let digit = 0; // 当前待乘单位的数字（"十"前无数字按 1 计）
  for (const ch of s) {
    const d = CN_DIGITS[ch];
    if (d !== undefined) {
      digit = d;
      continue;
    }
    const unit = CN_UNITS[ch];
    if (unit === 10000) {
      total += (section + digit) * unit;
      section = 0;
    } else {
      section += (digit === 0 ? 1 : digit) * unit;
    }
    digit = 0;
  }
  return total + section + digit;
}

type Chunk = { kind: "num"; value: number } | { kind: "stem"; index: number } | { kind: "text"; text: string };

const isAsciiDigit = (c: string): boolean => c >= "0" && c <= "9";

/** 名称切分：数字串 / 中文数字串 / 天干单字 / 普通文本段 */
function chunkize(s: string): Chunk[] {
  const chars = [...s];
  const chunks: Chunk[] = [];
  let i = 0;
  while (i < chars.length) {
    const c = chars[i];
    if (isAsciiDigit(c)) {
      let j = i;
      while (j < chars.length && isAsciiDigit(chars[j])) j++;
      chunks.push({ kind: "num", value: parseInt(chars.slice(i, j).join(""), 10) });
      i = j;
    } else if (CN_NUM_CHARS.has(c)) {
      let j = i;
      while (j < chars.length && CN_NUM_CHARS.has(chars[j])) j++;
      chunks.push({ kind: "num", value: parseCnNumber(chars.slice(i, j).join("")) });
      i = j;
    } else if (STEMS.includes(c)) {
      chunks.push({ kind: "stem", index: STEMS.indexOf(c) });
      i++;
    } else {
      let j = i;
      while (j < chars.length && !isAsciiDigit(chars[j]) && !CN_NUM_CHARS.has(chars[j]) && !STEMS.includes(chars[j])) j++;
      chunks.push({ kind: "text", text: chars.slice(i, j).join("") });
      i = j;
    }
  }
  return chunks;
}

const cmpText = (a: string, b: string): number => a.localeCompare(b, "zh-Hans-CN");

/** 自然排序比较：chunk 逐段比——同类数值 / 天干按序，异类退化文本比较 */
export function naturalCompare(a: string, b: string): number {
  const ca = chunkize(a);
  const cb = chunkize(b);
  const n = Math.min(ca.length, cb.length);
  for (let i = 0; i < n; i++) {
    const x = ca[i];
    const y = cb[i];
    if (x.kind === "num" && y.kind === "num") {
      if (x.value !== y.value) return x.value - y.value;
      continue;
    }
    if (x.kind === "stem" && y.kind === "stem") {
      if (x.index !== y.index) return x.index - y.index;
      continue;
    }
    // 异类 chunk（含 text-text）：文本比较
    const tx = x.kind === "text" ? x.text : x.kind === "num" ? String(x.value) : STEMS[x.index];
    const ty = y.kind === "text" ? y.text : y.kind === "num" ? String(y.value) : STEMS[y.index];
    const r = cmpText(tx, ty);
    if (r !== 0) return r;
  }
  if (ca.length !== cb.length) return ca.length - cb.length;
  return cmpText(a, b); // 全等兜底（确定性）
}

type Cmp = (a: PlacementView, b: PlacementView) => number;

function comparatorFor(key: Exclude<SortKey, "">): { cmp: Cmp; empty: (p: PlacementView) => boolean } {
  switch (key) {
    case "name":
      return { cmp: (a, b) => naturalCompare(a.name, b.name), empty: () => false };
    case "size":
      return { cmp: (a, b) => a.size - b.size, empty: () => false };
    case "duration":
      return {
        cmp: (a, b) => (a.duration ?? 0) - (b.duration ?? 0),
        empty: (p) => p.duration == null,
      };
    case "ext":
      return { cmp: (a, b) => cmpText(a.ext, b.ext), empty: () => false };
    case "createdAt":
      return {
        // RFC3339 同格式同偏移（本地秒级），字典序 = 时序
        cmp: (a, b) => cmpText(a.createdAt ?? "", b.createdAt ?? ""),
        empty: (p) => p.createdAt == null,
      };
    case "modifiedAt":
      return { cmp: (a, b) => cmpText(a.modifiedAt, b.modifiedAt), empty: () => false };
    case "importedAt":
      return { cmp: (a, b) => cmpText(a.importedAt, b.importedAt), empty: () => false };
  }
}

/** 平局兜底：名称自然排序 + assetId（恒升序，不随方向反转——防降序时同名项乱跳） */
function tieBreak(a: PlacementView, b: PlacementView): number {
  const r = naturalCompare(a.name, b.name);
  return r !== 0 ? r : a.assetId.localeCompare(b.assetId);
}

/**
 * 素材列表显示排序：自定义（"" / 非键值）原样返回存储序；
 * 其余键返回新数组（不改原数组）——空值恒排尾，主键随 sortAsc 反转，平局兜底不反转。
 * 泛型保持元素类型（快速访问视图的 RecentAssetView / RecycleEntryView 扩展字段不丢）。
 */
export function sortPlacements<T extends PlacementView>(list: T[], sortBy: string, asc: boolean): T[] {
  const key = (sortBy || "") as SortKey;
  if (key === "") return list;
  const { cmp, empty } = comparatorFor(key);
  return [...list].sort((a, b) => {
    const ea = empty(a);
    const eb = empty(b);
    if (ea || eb) {
      if (ea && eb) return tieBreak(a, b);
      return ea ? 1 : -1; // 空值恒排尾
    }
    const r = cmp(a, b);
    if (r !== 0) return asc ? r : -r;
    return tieBreak(a, b);
  });
}

/** 当前是否处于显示排序态（非自定义）：物化判定用——true 时手动结构操作先物化可见序再切回自定义 */
export function isDisplaySorted(sortBy: string): boolean {
  return sortBy !== "" && sortBy !== "custom";
}
