// 自适应布局（等高不等宽）：贪心分行算法，替代 justified-layout npm 库。
// 换库原因：该库 10 万条布局实测约 2.6s（内部对象分配过重），resize 重排卡死主线程；
// 算法本身简单（按目标行高贪心填行、行内等比拉伸对齐），自写单循环即可，实测 <100ms。
// 卡片文字区（标题 + 标签）为固定像素高，不参与等比缩放：按行追加到 top / height，
// 使 box 覆盖整张卡片而图片区仍严格等比（见 AssetGrid / AssetCard）。

// 卡片文字区高：标题 20 + 间距 2 + 标签 18 + 间距 2（token 17）
export const CARD_CAPTION_H = 42;

export interface Box {
  top: number;
  left: number;
  width: number;
  height: number;
}

export interface Row {
  top: number;
  height: number;
  start: number; // boxes 起始下标（含）
  end: number; // 结束下标（不含）
}

export interface JustifiedResult {
  boxes: Box[];
  rows: Row[];
  totalHeight: number;
}

// 网格内边距：左 0（由素材区块容器 padding 左 8 承担），右 8（滚动条 4 + 卡片间距 4）
const PAD = { top: 0, right: 8, bottom: 0, left: 0 };
const SPACING = 6;

export function computeJustified(
  items: { w: number; h: number }[],
  containerWidth: number,
  targetRowHeight: number,
  captionHeight = 0,
  offsetTop = 0,
): JustifiedResult {
  const n = items.length;
  const boxes = new Array<Box>(n);
  const rows: Row[] = [];
  const contentWidth = containerWidth - PAD.left - PAD.right;
  let y = PAD.top + offsetTop;
  let rowStart = 0;
  let ratioSum = 0;

  const layoutRow = (start: number, end: number, height: number): void => {
    const top = y + rows.length * captionHeight;
    let x = PAD.left;
    for (let i = start; i <= end; i++) {
      const w = (items[i].w / items[i].h) * height;
      boxes[i] = { top, left: x, width: w, height: height + captionHeight };
      x += w + SPACING;
    }
    rows.push({ top, height: height + captionHeight, start, end: end + 1 });
    y += height + SPACING;
  };

  for (let i = 0; i < n; i++) {
    ratioSum += items[i].w / items[i].h;
    const count = i - rowStart + 1;
    // 加入第 i 项后行高压到目标行高以下即封行（行内等比拉伸填满宽度）
    if ((contentWidth - SPACING * (count - 1)) / ratioSum <= targetRowHeight) {
      layoutRow(rowStart, i, (contentWidth - SPACING * (count - 1)) / ratioSum);
      rowStart = i + 1;
      ratioSum = 0;
    }
  }
  // 末行：不拉伸，左对齐
  if (rowStart < n) {
    const count = n - rowStart;
    const h = Math.min(targetRowHeight, (contentWidth - SPACING * (count - 1)) / ratioSum);
    layoutRow(rowStart, n - 1, h);
  }

  const totalHeight = rows.length === 0 ? 0 : y - SPACING + PAD.bottom + rows.length * captionHeight;
  return { boxes, rows, totalHeight };
}

// 行顶二分：第一个 top >= v 的行下标
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
