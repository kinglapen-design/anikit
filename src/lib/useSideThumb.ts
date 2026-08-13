// 侧栏悬浮滚动条（font-icon-5：4px 胶囊、距右 4px、不影响布局）。
// 原生滚动条占位会把内容区向左挤、导致各区块右缘错位；Chromium 已弃用 overflow: overlay，
// 故隐藏原生条（base.css 全局 width 0），自绘悬浮 thumb 同步滚动位置。
// 拖拽：pointerdown 后按 内容可滚动高 / 可视高 比例换算 scrollTop（2026-07-23 补，
// 此前 pointer-events none 纯指示——事件穿透导致右侧误触窗口 resize 热区）。
// 初始化走 watch（2026-08-13 ScrollThumb 组件化修复）：bodyRef 可能经 props 传值——
// 模板 ref 填充晚于子组件 mounted（prop 初值 undefined），onMounted 一次性初始化会永远 show=false；
// watch immediate + 元素就位重跑，两种传法（同组件 ref 对象 / 跨组件 props 值）都健壮。
import { onBeforeUnmount, reactive, watch, type Ref } from "vue";

export function useSideThumb(bodyRef: Ref<HTMLElement | undefined>) {
  // dragging：thumb 拖拽中为 true——滚动条悬停显隐（2026-08-10）的拖拽锁，拖出面板不消失
  const state = reactive({ show: false, top: 0, height: 30, dragging: false, onThumbDown });

  function sync(): void {
    const el = bodyRef.value;
    if (!el) return;
    const { scrollHeight, clientHeight, scrollTop } = el;
    state.show = scrollHeight > clientHeight;
    if (!state.show) return;
    state.height = Math.max(30, (clientHeight / scrollHeight) * clientHeight);
    state.top =
      el.offsetTop + (scrollTop / (scrollHeight - clientHeight)) * (clientHeight - state.height);
  }

  /** 拖拽滚动：thumb 位移按比例映射到 scrollTop */
  function onThumbDown(e: PointerEvent): void {
    const el = bodyRef.value;
    if (!el || !state.show) return;
    e.preventDefault();
    e.stopPropagation();
    state.dragging = true;
    const startY = e.clientY;
    const startScroll = el.scrollTop;
    const ratio = (el.scrollHeight - el.clientHeight) / (el.clientHeight - state.height);
    const onMove = (ev: PointerEvent): void => {
      el.scrollTop = startScroll + (ev.clientY - startY) * ratio;
    };
    const onUp = (): void => {
      state.dragging = false;
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
  }

  watch(
    bodyRef,
    (el, _prev, onCleanup) => {
      if (!el) return;
      // 内容根元素可能因视图切换 / 展开折叠而替换或变高，需跟随重观测（原 onMounted 版逻辑保留）；
      // RO 观测容器与内容根（内容高变化容器尺寸不变，靠内容根 RO 捕获），MO 兜内容根替换
      el.addEventListener("scroll", sync, { passive: true });
      const ro = new ResizeObserver(sync);
      ro.observe(el);
      let observedChild: Element | null = null;
      const watchChild = (): void => {
        if (el.firstElementChild === observedChild) return;
        if (observedChild) ro.unobserve(observedChild);
        observedChild = el.firstElementChild;
        if (observedChild) ro.observe(observedChild);
      };
      watchChild();
      const mo = new MutationObserver(() => {
        watchChild();
        sync();
      });
      mo.observe(el, { childList: true });
      onCleanup(() => {
        el.removeEventListener("scroll", sync);
        ro.disconnect();
        mo.disconnect();
      });
      sync();
    },
    { immediate: true, flush: "post" }, // post：DOM 稳定后再量（props 传值场景模板 ref 异步填充）
  );

  onBeforeUnmount(() => {
    state.dragging = false;
  });

  return state;
}
