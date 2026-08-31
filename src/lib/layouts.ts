// 内容区四种布局算法（PRD 04 卡片布局）：统一产出 boxes（含 item 下标）+ rows（滚动二分用）+ 总高。
// 布局全部用 metadata 宽高预计算，滚动时不读图（规范第 7 条）。
// 自适应为贪心分行（替换 justified-layout npm 库，10 万条 <100ms）。
// 盒高 = 纯图高（7.1 内容容器轮"卡即图满铺"定稿：名称/简介渐变罩叠图内，不占布局高度——
// 旧"图 + 下文字区"排版（简介区 57 / 紧凑 36 按行追加盒高）2026-08-30 用户实机废除：
// 盒高比图高多一截致盒比例 ≠ 图比例，cover 满铺必然裁切；盒比例 = 图比例时 cover 零裁）。

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

// 网格内边距（7.1 内容容器轮贴稿——中间面板-内容容器-列表容器 padding 12 12 0）：
// 左 0 / 上 0（asset-section padding 上左 12 承担），右 12（滚动条贴右边框后卡片右缘距边框恒 12，
// 滚动条 6px 与命中区 12 均在此带内）；底 66 = 悬浮栏遮挡补偿（7.1 二十二轮，2026-08-31：
// 悬浮栏距底 12 + 栏高 42 + 栏与末行卡片呼吸距 12，JS 滚动 translateY 不吃 CSS padding——
// 底部留白计入布局总高，末行卡片才能滚出遮挡区；稿值组合非新造）
export const PAD = { top: 0, right: 12, bottom: 66, left: 0 };
// 卡片间距（figma 42 文件卡片列表 gap 12，2026-08-10）：列间距 12，行间距 12
export const COL_GAP = 12;
export const ROW_GAP = 12;
/** 列表模式行高（7.1 内容容器轮贴稿——组件/卡片/列表卡片 57 高 = padding 6×2 + 内容 45；
    滑块联动作废口径不变） */
const LIST_ROW_H = 57;
/** 列表模式行间距（7.1 内容容器轮贴稿——列表显示模式 gap 6） */
const LIST_ROW_GAP = 6;

// —— 自适应（等高不等宽）：贪心分行，行内等比拉伸填满宽度；盒高 = 行高（卡即图满铺） ——
export function computeJustified(
  items: { w: number; h: number }[],
  containerWidth: number,
  targetRowHeight: number,
): LayoutResult {
  const n = items.length;
  const boxes = new Array<Box>(n);
  const rows: Row[] = [];
  const contentWidth = containerWidth - PAD.left - PAD.right;
  let y = PAD.top;
  let rowStart = 0;
  let ratioSum = 0;

  const layoutRow = (start: number, end: number, height: number): void => {
    const top = y;
    let x = PAD.left;
    for (let i = start; i <= end; i++) {
      const w = (items[i].w / items[i].h) * height;
      boxes[i] = { top, left: x, width: w, height, i };
      x += w + COL_GAP;
    }
    rows.push({ top, height, start, end: end + 1 });
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

  const totalHeight = rows.length === 0 ? 0 : y - ROW_GAP + PAD.bottom;
  return { boxes, rows, totalHeight };
}

// —— 瀑布流（等宽不等高）：列数随容器宽度，列宽弹性均分，逐条放最矮列；盒高 = 图等比高 ——
export function computeWaterfall(
  items: { w: number; h: number }[],
  containerWidth: number,
  targetColWidth: number,
): LayoutResult {
  const contentWidth = containerWidth - PAD.left - PAD.right;
  const cols = Math.max(1, Math.floor((contentWidth + COL_GAP) / (targetColWidth + COL_GAP)));
  const colW = (contentWidth - COL_GAP * (cols - 1)) / cols;
  const tops = new Array<number>(cols).fill(PAD.top);
  const boxes: Box[] = [];

  for (let i = 0; i < items.length; i++) {
    let c = 0;
    for (let k = 1; k < cols; k++) if (tops[k] < tops[c]) c = k;
    const h = (items[i].h / items[i].w) * colW;
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

// —— 网格（figma 比例，2026-08-10 用户定）：设计缩略图尺寸 260×220——格高 = 格宽 × 220/260
//（7.1 满铺定稿后盒即整卡，旧"整卡 = 缩略图 + 简介 57"口径废除）；格子随容器宽度
// 自适应填充（列数由目标格宽决定，格宽弹性均分无右侧空余） ——
const GRID_THUMB_RATIO = 220 / 260;
export function computeGrid(
  items: { w: number; h: number }[],
  containerWidth: number,
  targetCell: number,
): LayoutResult {
  const contentWidth = containerWidth - PAD.left - PAD.right;
  const cols = Math.max(1, Math.floor((contentWidth + COL_GAP) / (targetCell + COL_GAP)));
  const cell = (contentWidth - COL_GAP * (cols - 1)) / cols;
  const thumbH = cell * GRID_THUMB_RATIO;
  const rowH = thumbH + ROW_GAP;
  const boxes: Box[] = [];
  const rows: Row[] = [];

  for (let i = 0; i < items.length; i++) {
    const r = Math.floor(i / cols);
    const c = i % cols;
    const top = PAD.top + r * rowH;
    boxes.push({ top, left: PAD.left + c * (cell + COL_GAP), width: cell, height: thumbH, i });
    if (c === 0) rows.push({ top, height: thumbH, start: i, end: i + 1 });
    else rows[rows.length - 1].end = i + 1;
  }

  const totalHeight = rows.length === 0 ? 0 : rows[rows.length - 1].top + thumbH + PAD.bottom;
  return { boxes, rows, totalHeight };
}

// —— 列表（行式，figma 列表显示稿 2026-08-10）：行高固定 69（滑块联动作废），行距 4，
// 交替底色由渲染层处理 ——
export function computeList(
  items: { w: number; h: number }[],
  containerWidth: number,
  _targetRowHeight: number,
): LayoutResult {
  const rowH = LIST_ROW_H;
  const gap = LIST_ROW_GAP;
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
