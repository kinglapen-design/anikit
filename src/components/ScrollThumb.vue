<script setup lang="ts">
// 悬浮滚轮（主窗口统一交互，2026-08-10 用户定案"持久存在视觉噪音"→ 悬停显隐）：
// 封装 useSideThumb——宿主用法：relative 包裹层内放滚动容器（ref 传入 body）+ 本组件即可；
// hover 态自管（监听父包裹层 mouseenter/leave），拖拽中锁显；
// 色档 = **通用/四级常态、通用/六级 hover**（2026-09-06 用户定，实机走查第一批末项口径——
// 旧实色 C4/C5（#212323/#282a2a）在 g3 底上几乎不可见，实机踩坑）；absolute 贴右（right: 0）不占布局。
// 纵向滚动区统一走本组件（见 base.css .kit-scroll 注释）；横向滚动区仍原生 6px（.kit-scroll-x）。
import { onBeforeUnmount, onMounted, ref, toRef } from "vue";
import { useSideThumb } from "../lib/useSideThumb";

const props = defineProps<{
  body: HTMLElement | undefined; // 滚动容器（模板 ref 自动解包传入；须为包裹层直接子元素——thumb top 按其 offsetTop 换算）
}>();
const thumb = useSideThumb(toRef(props, "body"));

// hover 自管：鼠标在父包裹层上即显（滚轮坏了也能用——鼠标在内容区滚轮必在可拖）。
// mouseenter/leave = 快速路径；**window mousemove（capture）= 兜底校正**（2026-09-06 七轮用户实机：
// 子窗口打开后首次悬停滚轮不显、要点一下——WebView2 新窗未激活态首次 mouseenter 丢失，
// 点击激活后事件链才恢复；mousemove 不依赖「进入」语义，鼠标一动即按 contains 实时校正，
// 离开窗口由 mouseleave 置 false 防滞留）。
const hover = ref(false);
const rootEl = ref<HTMLElement>();
let wrap: HTMLElement | null = null;
const onEnter = (): void => {
  hover.value = true;
};
const onLeave = (): void => {
  hover.value = false;
};
const onMove = (e: MouseEvent): void => {
  hover.value = !!(wrap && e.target instanceof Node && wrap.contains(e.target));
};
onMounted(() => {
  wrap = rootEl.value?.parentElement ?? null;
  wrap?.addEventListener("mouseenter", onEnter);
  wrap?.addEventListener("mouseleave", onLeave);
  window.addEventListener("mousemove", onMove, { capture: true, passive: true });
});
onBeforeUnmount(() => {
  wrap?.removeEventListener("mouseenter", onEnter);
  wrap?.removeEventListener("mouseleave", onLeave);
  window.removeEventListener("mousemove", onMove, { capture: true });
});
</script>

<template>
  <span
    ref="rootEl"
    class="scroll-thumb"
    :class="{ visible: thumb.show && (hover || thumb.dragging) }"
    :style="{ top: `${thumb.top}px`, height: `${thumb.height}px` }"
    @pointerdown="thumb.onThumbDown"
  />
</template>

<style scoped>
/* 6px 胶囊 / 通用/四级常态、通用/六级 hover（2026-09-06 实机走查第一批末项口径，废旧 C4/C5）；
   贴右（right: 0，2026-08-13 用户定与主窗口统一——实色盖分隔线/边框线一小段可接受，同主窗口外框） */
.scroll-thumb {
  position: absolute;
  right: 0;
  width: 6px;
  border-radius: var(--n-radius-pill);
  background: var(--n-g4);
  z-index: 10;
  opacity: 0;
  pointer-events: none;
  transition:
    opacity 0.15s ease,
    background 0.12s ease;
}
.scroll-thumb.visible {
  opacity: 1;
  pointer-events: auto;
}
.scroll-thumb:hover {
  background: var(--n-g6);
}
</style>
