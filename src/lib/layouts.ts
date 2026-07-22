// 内容区四种布局算法（PRD 04 卡片布局）：统一产出 boxes（含 item 下标）+ rows（滚动二分用）+ 总高。
// 布局全部用 metadata 宽高预计算，滚动时不读图（规范第 7 条）。
// 自适应为贪心分行（替换 justified-layout npm 库，10 万条 <100ms）；
// 卡片文字区（标题 + 标签）固定像素高，不参与等比缩放（justified 按行追加）。

// 卡片文字区高：标题 20 + 间距 2 + 标签 18 + 间距 2（06_卡片）
export const CARD_CAPTION_H = 42;

export interface Box {
  top: number;
  left: number;
  width: number;
  height: number;
  i: number; // items 下标（瀑布排序后仍需回指）
}

export interface Row {
  top: number;
  height: number;
  start: number; // boxes 起始下标（含）
  end: number; // 结束下标（不含）
}

export interface LayoutResult {
  boxes: Box[];
  rows: Row[];
  totalHeight: number;
}

// 网格内边距：左 0（区块容器 padding 左 8 承担），右 8（滚动条 4 + 卡片间距 4）
const PAD = { top: 0, right: 8, bottom: 0, left: 0 };
// 卡片间距（阶段3修改定案）：列间距 6，行间距 14（token --space-14）
const COL_GAP = 6;
const ROW_GAP = 14;

// —— 自适应（等高不等宽）：贪心分行，行内等比拉伸填满宽度 ——
export function computeJustified(
  items: { w: number; h: number }[],
  containerWidth: number,
  targetRowHeight: number,
  captionHeight = 0,
): LayoutResult {
  const n = items.length;
  const boxes = new Array<Box>(n);
  const rows: Row[] = [];
  const contentWidth = containerWidth - PAD.left - PAD.right;
  let y = PAD.top;
  let rowStart = 0;
  let ratioSum = 0;

  const layoutRow = (start: number, end: number, height: number): void => {
    const top = y + rows.length * captionHeight;
    let x = PAD.left;
    for (let i = start; i <= end; i++) {
      const w = (items[i].w / items[i].h) * height;
      boxes[i] = { top, left: x, width: w, height: height + captionHeight, i };
      x += w + COL_GAP;
    }
    rows.push({ top, height: height + captionHeight, start, end: end + 1 });
    y += height + ROW_GAP;
  };

  for (let i = 0; i < n; i++) {
    ratioSum += items[i].w / items[i].h;
    const count = i - rowStart + 1;
    if ((contentWidth - COL_GAP * (count - 1)) / ratioSum <= targetRowHeight) {
      layoutRow(rowStart, i, (contentWidth - COL_GAP * (count - 1)) / ratioSum);
      rowStart = i + 1;
      ratioSum = 0;
    }
  }
  if (rowStart < n) {
    const count = n - rowStart;
    const h = Math.min(targetRowHeight, (contentWidth - COL_GAP * (count - 1)) / ratioSum);
    layoutRow(rowStart, n - 1, h);
  }

  const totalHeight = rows.length === 0 ? 0 : y - ROW_GAP + PAD.bottom + rows.length * captionHeight;
  return { boxes, rows, totalHeight };
}

// —— 瀑布流（等宽不等高）：列数随容器宽度，列宽弹性均分，逐条放最矮列 ——
export function computeWaterfall(
  items: { w: number; h: number }[],
  containerWidth: number,
  targetColWidth: number,
  captionHeight = 0,
): LayoutResult {
  const contentWidth = containerWidth - PAD.left - PAD.right;
  const cols = Math.max(1, Math.floor((contentWidth + COL_GAP) / (targetColWidth + COL_GAP)));
  const colW = (contentWidth - COL_GAP * (cols - 1)) / cols;
  const tops = new Array<number>(cols).fill(PAD.top);
  const boxes: Box[] = [];

  for (let i = 0; i < items.length; i++) {
    let c = 0;
    for (let k = 1; k < cols; k++) if (tops[k] < tops[c]) c = k;
    const h = (items[i].h / items[i].w) * colW + captionHeight;
    boxes.push({ top: tops[c], left: PAD.left + c * (colW + COL_GAP), width: colW, height: h, i });
    tops[c] += h + ROW_GAP;
  }

  // 瀑布无天然行：按 top 排序后每卡一行（滚动二分用）
  const order = boxes.map((_, i) => i).sort((a, b) => boxes[a].top - boxes[b].top);
  const rows: Row[] = order.map((boxIdx, k) => ({
    top: boxes[boxIdx].top,
    height: boxes[boxIdx].height,
    start: k,
    end: k + 1,
  }));
  return { boxes: order.map((boxIdx) => boxes[boxIdx]), rows, totalHeight: Math.max(...tops, 0) + PAD.bottom };
}

// —— 网格（正方形）：格子随容器宽度自适应填充（列数由目标格宽决定，格宽弹性均分无右侧空余） ——
export function computeGrid(
  items: { w: number; h: number }[],
  containerWidth: number,
  targetCell: number,
  captionHeight = 0,
): LayoutResult {
  const contentWidth = containerWidth - PAD.left - PAD.right;
  const cols = Math.max(1, Math.floor((contentWidth + COL_GAP) / (targetCell + COL_GAP)));
  const cell = (contentWidth - COL_GAP * (cols - 1)) / cols;
  const rowH = cell + captionHeight + ROW_GAP;
  const boxes: Box[] = [];
  const rows: Row[] = [];

  for (let i = 0; i < items.length; i++) {
    const r = Math.floor(i / cols);
    const c = i % cols;
    const top = PAD.top + r * rowH;
    boxes.push({ top, left: PAD.left + c * (cell + COL_GAP), width: cell, height: cell + captionHeight, i });
    if (c === 0) rows.push({ top, height: cell + captionHeight, start: i, end: i + 1 });
    else rows[rows.length - 1].end = i + 1;
  }

  const totalHeight = rows.length === 0 ? 0 : rows[rows.length - 1].top + cell + captionHeight + PAD.bottom;
  return { boxes, rows, totalHeight };
}

// —— 列表（行式，02_item 10）：行高随滑块（57 基准 × 行高 / 220），item 间距 6，交替底色由渲染层处理 ——
export function computeList(
  items: { w: number; h: number }[],
  containerWidth: number,
  targetRowHeight: number,
  _captionHeight = 0,
): LayoutResult {
  const rowH = Math.max(21, Math.round((57 * targetRowHeight) / 220));
  const gap = ROW_GAP; // 行间距 14（token，阶段3修改定案）
  const width = containerWidth - PAD.left - PAD.right;
  const boxes: Box[] = [];
  const rows: Row[] = [];

  for (let i = 0; i < items.length; i++) {
    const top = PAD.top + i * (rowH + gap);
    boxes.push({ top, left: PAD.left, width, height: rowH, i });
    rows.push({ top, height: rowH, start: i, end: i + 1 });
  }

  const totalHeight = items.length === 0 ? 0 : PAD.top + items.length * (rowH + gap) - gap + PAD.bottom;
  return { boxes, rows, totalHeight };
}

// —— 行顶二分：第一个 top >= v 的行下标 ——
export function lowerBound(rows: Row[], v: number): number {
  let lo = 0;
  let hi = rows.length;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (rows[mid].top < v) lo = mid + 1;
    else hi = mid;
  }
  return lo;
}
