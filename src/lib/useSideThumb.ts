// 侧栏悬浮滚动条（border-1：4px 胶囊、距右 4px、不影响布局）。
// 原生滚动条占位会把内容区向左挤、导致各区块右缘错位；Chromium 已弃用 overflow: overlay，
// 故隐藏原生条（base.css 全局 width 0），自绘悬浮 thumb 同步滚动位置。
import { onBeforeUnmount, onMounted, reactive, type Ref } from "vue";

export function useSideThumb(bodyRef: Ref<HTMLElement | undefined>) {
  const thumb = { show: false, top: 0, height: 30 };
  const state = reactive(thumb);
  let ro: ResizeObserver | undefined;
  let mo: MutationObserver | undefined;
  let observedChild: Element | null = null;

  function sync(): void {
    const el = bodyRef.value;
    if (!el) return;
    // 内容根元素可能因视图切换 / 展开折叠而替换或变高，需跟随重观测
    if (ro && el.firstElementChild !== observedChild) {
      if (observedChild) ro.unobserve(observedChild);
      observedChild = el.firstElementChild;
      if (observedChild) ro.observe(observedChild);
    }
    const { scrollHeight, clientHeight, scrollTop } = el;
    state.show = scrollHeight > clientHeight;
    if (!state.show) return;
    state.height = Math.max(30, (clientHeight / scrollHeight) * clientHeight);
    state.top =
      el.offsetTop + (scrollTop / (scrollHeight - clientHeight)) * (clientHeight - state.height);
  }

  onMounted(() => {
    const el = bodyRef.value;
    if (!el) return;
    el.addEventListener("scroll", sync, { passive: true });
    ro = new ResizeObserver(sync);
    ro.observe(el);
    // 内容根元素替换时 RO 不一定触发（尺寸未变 / 旧观测元素已卸载）——补 childList 监听兜底
    mo = new MutationObserver(sync);
    mo.observe(el, { childList: true });
    sync();
  });

  onBeforeUnmount(() => {
    bodyRef.value?.removeEventListener("scroll", sync);
    ro?.disconnect();
    mo?.disconnect();
  });

  return state;
}
