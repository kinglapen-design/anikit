// 自定义 transform 滚动（踩坑 005 定案）：禁原生滚动，wheel / 拖块 / 点轨道 / 键盘
// 统一走 scrollTo —— 先回调渲染目标区域，同帧再由组件用 transform 挪 canvas，
// 同一 JS 任务同一帧原子提交，杜绝滚动条拖拽的合成器先行闪白。
import { ref } from "vue";

export function useVirtualScroll(onUpdate: (pos: number) => void) {
  const pos = ref(0);
  const thumbTop = ref(0);
  const thumbHeight = ref(30);
  // 有可滚内容才显示滚动条（用户验收：无需滚动时轨道要隐藏）
  const scrollable = ref(false);
  let viewport: HTMLElement | null = null;
  let total = 0;

  const viewH = () => viewport?.clientHeight ?? 0;

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

  function scrollTo(next: number): void {
    const max = Math.max(0, total - viewH());
    const clamped = Math.max(0, Math.min(next, max));
    pos.value = clamped;
    onUpdate(clamped);
    syncThumb();
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
    scrollTo(pos.value + e.deltaY * 1.5);
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
    scrollTo(next);
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
      thumb.setPointerCapture(e.pointerId);
      e.preventDefault();
      e.stopPropagation();
    };
    const onThumbMove = (e: PointerEvent) => {
      if (!thumb.hasPointerCapture(e.pointerId)) return;
      const trackH = viewH() - thumbHeight.value;
      if (trackH <= 0) return;
      scrollTo(dragPos + ((e.clientY - dragY) / trackH) * (total - viewH()));
    };
    const onThumbUp = (e: PointerEvent) => {
      if (thumb.hasPointerCapture(e.pointerId)) thumb.releasePointerCapture(e.pointerId);
    };
    const onTrackDown = (e: PointerEvent) => {
      if (e.target === thumb) return;
      const rect = thumb.getBoundingClientRect();
      const dir = e.clientY > rect.bottom ? 1 : -1;
      scrollTo(pos.value + dir * viewH() * 0.9);
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
    for (const f of detachFns) f();
    detachFns = [];
    viewport = null;
  }

  return { pos, thumbTop, thumbHeight, scrollable, scrollTo, setTotal, attach, detach };
}
