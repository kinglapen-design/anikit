// 自定义 transform 滚动（踩坑 005 定案）：禁原生滚动，wheel / 拖块 / 点轨道 / 键盘
// 统一走 scrollTo —— 先回调渲染目标区域，同帧再由组件用 transform 挪 canvas，
// 同一 JS 任务同一帧原子提交，杜绝滚动条拖拽的合成器先行闪白。
// 2026-09-23：onUserScroll 回调（用户源滚差——快速到顶/到底按钮显隐与放弃待恢复位用）；
// smoothScrollTo 平滑滚动（快速到顶/到底点击），用户滚动经 scrollTo 随时打断动画。
import { ref } from "vue";

export function useVirtualScroll(
  onUpdate: (pos: number) => void,
  onUserScroll?: (delta: number) => void,
) {
  const pos = ref(0);
  const thumbTop = ref(0);
  const thumbHeight = ref(30);
  // 有可滚内容才显示滚动条（用户验收：无需滚动时轨道要隐藏）
  const scrollable = ref(false);
  // thumb 拖拽中为 true——滚动条悬停显隐（2026-08-10）的拖拽锁，拖出面板不消失
  const dragging = ref(false);
  let viewport: HTMLElement | null = null;
  let total = 0;
  // 平滑滚动动画句柄（0 = 无动画）：用户路径 scrollTo 打断，程序 scrollTo 同样取消
  let animId = 0;

  const viewH = () => viewport?.clientHeight ?? 0;

  function cancelAnim(): void {
    if (animId) cancelAnimationFrame(animId);
    animId = 0;
  }

  function syncThumb(): void {
    const vh = viewH();
    scrollable.value = total > 0 && vh > 0 && total > vh;
    if (total <= 0 || vh <= 0) {
      thumbHeight.value = 30;
      thumbTop.value = 0;
      return;
    }
    thumbHeight.value = Math.max(30, (vh / total) * vh);
    thumbTop.value = total <= vh ? 0 : (pos.value / (total - vh)) * (vh - thumbHeight.value);
  }

  function scrollTo(next: number, user = false): void {
    cancelAnim();
    const max = Math.max(0, total - viewH());
    const clamped = Math.max(0, Math.min(next, max));
    const delta = clamped - pos.value;
    pos.value = clamped;
    onUpdate(clamped);
    syncThumb();
    // 用户源滚动（wheel / 键盘 / 拖块 / 点轨道）上报真实位移（顶 / 底续滚被夹为 0 不上报）
    if (user && delta !== 0) onUserScroll?.(delta);
  }

  // 平滑滚动（2026-09-23 快速到顶 / 到底点击）：300ms 缓入缓出（分段滑块 300ms 动画档）；
  // 不走路径回调上报（程序源），任何 scrollTo（含用户滚动）打断
  function smoothScrollTo(target: number): void {
    cancelAnim();
    const max = Math.max(0, total - viewH());
    const to = Math.max(0, Math.min(target, max));
    const from = pos.value;
    if (from === to) return;
    const DUR = 300;
    const t0 = performance.now();
    const step = (t: number): void => {
      const k = Math.min(1, (t - t0) / DUR);
      const e = k < 0.5 ? 4 * k * k * k : 1 - Math.pow(-2 * k + 2, 3) / 2; // easeInOutCubic
      const p = from + (to - from) * e;
      pos.value = p;
      onUpdate(p);
      syncThumb();
      if (k < 1) animId = requestAnimationFrame(step);
      else animId = 0;
    };
    animId = requestAnimationFrame(step);
  }

  // 布局总高变化（重排 / 视口高度变化）后重夹位置
  function setTotal(t: number): void {
    total = t;
    scrollTo(pos.value);
  }

  function onWheel(e: WheelEvent): void {
    // ctrl + 滚轮为卡片缩放（03_UI 布局/03_中间面板 02），交给 AssetGrid 处理
    if (e.ctrlKey) return;
    e.preventDefault();
    scrollTo(pos.value + e.deltaY * 1.5, true);
  }

  function onKeydown(e: KeyboardEvent): void {
    const page = viewH() * 0.9;
    let next: number | null = null;
    if (e.key === "PageDown") next = pos.value + page;
    else if (e.key === "PageUp") next = pos.value - page;
    else if (e.key === "Home") next = 0;
    else if (e.key === "End") next = total;
    else if (e.key === "ArrowDown") next = pos.value + 120;
    else if (e.key === "ArrowUp") next = pos.value - 120;
    if (next === null) return;
    e.preventDefault();
    scrollTo(next, true);
  }

  let detachFns: (() => void)[] = [];

  function attach(vp: HTMLElement, track: HTMLElement, thumb: HTMLElement): void {
    viewport = vp;
    vp.addEventListener("wheel", onWheel, { passive: false });
    vp.addEventListener("keydown", onKeydown);

    let dragY = 0;
    let dragPos = 0;
    const onThumbDown = (e: PointerEvent) => {
      dragY = e.clientY;
      dragPos = pos.value;
      dragging.value = true;
      thumb.setPointerCapture(e.pointerId);
      e.preventDefault();
      e.stopPropagation();
    };
    const onThumbMove = (e: PointerEvent) => {
      if (!thumb.hasPointerCapture(e.pointerId)) return;
      const trackH = viewH() - thumbHeight.value;
      if (trackH <= 0) return;
      scrollTo(dragPos + ((e.clientY - dragY) / trackH) * (total - viewH()), true);
    };
    const onThumbUp = (e: PointerEvent) => {
      dragging.value = false;
      if (thumb.hasPointerCapture(e.pointerId)) thumb.releasePointerCapture(e.pointerId);
    };
    const onTrackDown = (e: PointerEvent) => {
      if (e.target === thumb) return;
      const rect = thumb.getBoundingClientRect();
      const dir = e.clientY > rect.bottom ? 1 : -1;
      scrollTo(pos.value + dir * viewH() * 0.9, true);
    };
    thumb.addEventListener("pointerdown", onThumbDown);
    thumb.addEventListener("pointermove", onThumbMove);
    thumb.addEventListener("pointerup", onThumbUp);
    track.addEventListener("pointerdown", onTrackDown);

    detachFns = [
      () => vp.removeEventListener("wheel", onWheel),
      () => vp.removeEventListener("keydown", onKeydown),
      () => thumb.removeEventListener("pointerdown", onThumbDown),
      () => thumb.removeEventListener("pointermove", onThumbMove),
      () => thumb.removeEventListener("pointerup", onThumbUp),
      () => track.removeEventListener("pointerdown", onTrackDown),
    ];
  }

  function detach(): void {
    cancelAnim();
    for (const f of detachFns) f();
    detachFns = [];
    viewport = null;
  }

  return { pos, thumbTop, thumbHeight, scrollable, dragging, scrollTo, smoothScrollTo, setTotal, attach, detach };
}
