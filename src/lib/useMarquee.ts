// 框选 composable（6.1 B3 自 AssetGrid 抽出，阶段 C 为控制 AssetGrid 行数 ≤500 落 lib）：
// 视口空白处按住左键拖出矩形，实时把命中集写入 selection store；
// 无修饰 = 替换选择，Ctrl / Shift = 在起点选择上追加；未过 4px 阈值松开 = 点击空白 = 清空选择。
// 命中判定由调用方注入（viewport 坐标矩形 → 命中 key 清单），本模块只管手势与选择合并。
// 兜底（2026-08-10 用户实测 BUG）：系统截图工具 / Win11 屏幕边缘 snap 预览直接抢走指针，
// pointerup / pointercancel 双双丢失 → 框选矩形永久残留；lostpointercapture（capture 被系统
// 剥夺的对症事件）+ window blur 两道兜底强制取消。
import { onBeforeUnmount, onMounted, ref, type Ref } from "vue";
import { useSelectionStore } from "../stores/selection";

export interface MarqueeRect {
  left: number;
  top: number;
  width: number;
  height: number;
}

const MARQUEE_THRESHOLD = 4;

// 网格 boxes 命中工厂（AssetGrid 注入几何；框选与卡片同套坐标换算——
// viewport y = box.top + 头部偏移 − 滚动 pos，canvas transform 同构；scale 补偿机制已废 2026-08-10）
export function makeBoxHitTest(ctx: {
  boxes: () => { top: number; left: number; width: number; height: number; i: number }[];
  offsetY: () => number; // 头部偏移（headerH + 间距 + 列头）− 滚动 pos
  keyAt: (i: number) => string;
}): (rect: MarqueeRect) => string[] {
  return (rect) => {
    const offY = ctx.offsetY();
    const hits: string[] = [];
    for (const b of ctx.boxes()) {
      const by = b.top + offY;
      if (
        b.left < rect.left + rect.width &&
        b.left + b.width > rect.left &&
        by < rect.top + rect.height &&
        by + b.height > rect.top
      ) {
        hits.push(ctx.keyAt(b.i));
      }
    }
    return hits;
  };
}

// 头部空白 pointerdown = 清空选择（2026-07-28 复验反馈 4，与底部空白同语义）；
// 文件夹卡片 / 收件箱卡片 / 按钮 / 输入框不挡——卡片多选按下不得先清上一项
export function headerBlankClear(e: PointerEvent): void {
  const t = e.target as HTMLElement;
  if (t.closest(".folder-card, .disk-card, button, input")) return;
  useSelectionStore().clear();
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
  let capturedId: number | null = null; // 当前 capture 的 pointerId（兜底取消用）

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
    capturedId = e.pointerId;
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
    capturedId = null;
    if (active) {
      marquee.value = null;
      active = false;
    } else if (!cancelled) {
      selection.clear(); // 未过阈值 = 点击空白 = 清空选择（2026-07-28 用户定）
    }
  }

  // 强制取消（不依赖 pointer 事件送达）：capture 被系统剥夺（lostpointercapture）或
  // 窗口失焦（截图工具抢焦点）时调用；手势已不可能继续，直接清矩形不碰选择集
  function forceCancel(): void {
    if (capturedId != null && opts.viewport.value?.hasPointerCapture(capturedId)) {
      opts.viewport.value.releasePointerCapture(capturedId);
    }
    capturedId = null;
    if (active) {
      marquee.value = null;
      active = false;
    }
  }

  onMounted(() => {
    // lostpointercapture 挂在 viewport（capture 宿主），blur 挂在 window（截图工具抢焦点）
    opts.viewport.value?.addEventListener("lostpointercapture", forceCancel);
    window.addEventListener("blur", forceCancel);
  });
  onBeforeUnmount(() => {
    opts.viewport.value?.removeEventListener("lostpointercapture", forceCancel);
    window.removeEventListener("blur", forceCancel);
  });

  return {
    marquee,
    onPointerDown,
    onPointerMove,
    onPointerUp: (e: PointerEvent) => end(e, false),
    onPointerCancel: (e: PointerEvent) => end(e, true),
  };
}
