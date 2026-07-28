// 框选 composable（6.1 B3 自 AssetGrid 抽出，阶段 C 为控制 AssetGrid 行数 ≤500 落 lib）：
// 视口空白处按住左键拖出矩形，实时把命中集写入 selection store；
// 无修饰 = 替换选择，Ctrl / Shift = 在起点选择上追加；未过 4px 阈值松开 = 点击空白 = 清空选择。
// 命中判定由调用方注入（viewport 坐标矩形 → 命中 key 清单），本模块只管手势与选择合并。
import { ref, type Ref } from "vue";
import { useSelectionStore } from "../stores/selection";

export interface MarqueeRect {
  left: number;
  top: number;
  width: number;
  height: number;
}

const MARQUEE_THRESHOLD = 4;

// 网格 boxes 命中工厂（AssetGrid 注入几何；框选与卡片同套坐标换算——
// viewport y = box.top × scale + 头部偏移 − 滚动 pos，canvas transform 同构）
export function makeBoxHitTest(ctx: {
  boxes: () => { top: number; left: number; width: number; height: number; i: number }[];
  scale: () => number;
  offsetY: () => number; // 头部偏移（headerH + 间距 + 列头）− 滚动 pos
  keyAt: (i: number) => string;
}): (rect: MarqueeRect) => string[] {
  return (rect) => {
    const offY = ctx.offsetY();
    const s = ctx.scale();
    const hits: string[] = [];
    for (const b of ctx.boxes()) {
      const bx = b.left * s;
      const by = b.top * s + offY;
      if (
        bx < rect.left + rect.width &&
        bx + b.width * s > rect.left &&
        by < rect.top + rect.height &&
        by + b.height * s > rect.top
      ) {
        hits.push(ctx.keyAt(b.i));
      }
    }
    return hits;
  };
}

export function useMarquee(opts: {
  viewport: Ref<HTMLElement | undefined>;
  ignoreSelector: string; // 这些元素上不启动框选（卡片 / 头部 / 滚动条 / 按钮等）
  hitTest: (rect: MarqueeRect) => string[]; // viewport 坐标矩形 → 命中 key
}) {
  const selection = useSelectionStore();
  const marquee = ref<MarqueeRect | null>(null);
  let startX = 0;
  let startY = 0;
  let active = false;
  let additive = false;
  let base = new Set<string>(); // 追加模式的起点选中集（拖动期间并集运算）

  function vpPoint(e: PointerEvent): { x: number; y: number } {
    const r = opts.viewport.value!.getBoundingClientRect();
    return { x: e.clientX - r.left, y: e.clientY - r.top };
  }

  function onPointerDown(e: PointerEvent): void {
    if (e.button !== 0 || !e.isPrimary) return;
    const t = e.target as HTMLElement;
    if (t.closest(opts.ignoreSelector)) return;
    e.preventDefault(); // 框选期间禁止触发文本选择 / 原生拖拽
    const p = vpPoint(e);
    startX = p.x;
    startY = p.y;
    active = false;
    additive = e.ctrlKey || e.metaKey || e.shiftKey;
    base = new Set(selection.selected);
    opts.viewport.value!.setPointerCapture(e.pointerId);
  }

  function onPointerMove(e: PointerEvent): void {
    if (!opts.viewport.value?.hasPointerCapture(e.pointerId)) return;
    const p = vpPoint(e);
    if (!active) {
      if (Math.hypot(p.x - startX, p.y - startY) < MARQUEE_THRESHOLD) return;
      active = true;
    }
    const rect: MarqueeRect = {
      left: Math.min(startX, p.x),
      top: Math.min(startY, p.y),
      width: Math.abs(p.x - startX),
      height: Math.abs(p.y - startY),
    };
    marquee.value = rect;
    const hits = new Set<string>(additive ? base : []);
    for (const k of opts.hitTest(rect)) hits.add(k);
    selection.setSelection(hits);
  }

  function end(e: PointerEvent, cancelled: boolean): void {
    const vp = opts.viewport.value;
    if (!vp?.hasPointerCapture(e.pointerId)) return;
    vp.releasePointerCapture(e.pointerId);
    if (active) {
      marquee.value = null;
      active = false;
    } else if (!cancelled) {
      selection.clear(); // 未过阈值 = 点击空白 = 清空选择（2026-07-28 用户定）
    }
  }

  return {
    marquee,
    onPointerDown,
    onPointerMove,
    onPointerUp: (e: PointerEvent) => end(e, false),
    onPointerCancel: (e: PointerEvent) => end(e, true),
  };
}
