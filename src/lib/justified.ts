// 自适应布局（等高不等宽）封装：justified-layout 预计算，滚动时不读图（规范第 7 条）。
// 卡片文字区（标题 + 标签）为固定像素高，不参与等比缩放：
// 按行分组后把每行 top 下移"行号 × 文字区高"、box 高度加上文字区高，
// 使 box 覆盖整张卡片而图片区仍严格等比（见 AssetGrid / AssetCard）。
import justifiedLayout from "justified-layout";

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

export function computeJustified(
  items: { w: number; h: number }[],
  containerWidth: number,
  targetRowHeight: number,
  captionHeight = 0,
): JustifiedResult {
  const geo = justifiedLayout(
    items.map((i) => ({ width: i.w, height: i.h })),
    // 素材卡片容器内边距 8 8 0 8、素材列表子项间距 6（02_组件布局 05）
    { containerWidth, targetRowHeight, boxSpacing: 6, containerPadding: { top: 8, right: 8, bottom: 0, left: 8 } },
  );
  const boxes = geo.boxes as unknown as Box[];

  const rows: Row[] = [];
  for (let i = 0; i < boxes.length; i++) {
    const b = boxes[i];
    let row = rows[rows.length - 1];
    if (!row || row.top !== b.top) {
      row = { top: b.top, height: b.height, start: i, end: i + 1 };
      rows.push(row);
    } else {
      row.end = i + 1;
    }
  }

  if (captionHeight > 0) {
    for (let r = 0; r < rows.length; r++) {
      const row = rows[r];
      for (let i = row.start; i < row.end; i++) {
        boxes[i].top += r * captionHeight;
        boxes[i].height += captionHeight;
      }
      row.top += r * captionHeight;
      row.height += captionHeight;
    }
  }

  return { boxes, rows, totalHeight: geo.containerHeight + rows.length * captionHeight };
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
