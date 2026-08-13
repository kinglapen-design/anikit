<script setup lang="ts">
// 悬浮滚轮（主窗口统一交互，2026-08-10 用户定案"持久存在视觉噪音"→ 悬停显隐）：
// 封装 useSideThumb——宿主用法：relative 包裹层内放滚动容器（ref 传入 body）+ 本组件即可；
// hover 态自管（监听父包裹层 mouseenter/leave），拖拽中锁显；实色 C4 / 悬停 C5，
// absolute 距右 2px 不占布局（不挤压内容、不挡 inset 边框线）。
// 纵向滚动区统一走本组件（见 base.css .kit-scroll 注释）；横向滚动区仍原生 6px（.kit-scroll-x）。
import { onBeforeUnmount, onMounted, ref, toRef } from "vue";
import { useSideThumb } from "../lib/useSideThumb";

const props = defineProps<{
  body: HTMLElement | undefined; // 滚动容器（模板 ref 自动解包传入；须为包裹层直接子元素——thumb top 按其 offsetTop 换算）
}>();
const thumb = useSideThumb(toRef(props, "body"));

// hover 自管：鼠标在父包裹层上即显（滚轮坏了也能用——鼠标在内容区滚轮必在可拖）
const hover = ref(false);
const rootEl = ref<HTMLElement>();
let wrap: HTMLElement | null = null;
const onEnter = (): void => {
  hover.value = true;
};
const onLeave = (): void => {
  hover.value = false;
};
onMounted(() => {
  wrap = rootEl.value?.parentElement ?? null;
  wrap?.addEventListener("mouseenter", onEnter);
  wrap?.addEventListener("mouseleave", onLeave);
});
onBeforeUnmount(() => {
  wrap?.removeEventListener("mouseenter", onEnter);
  wrap?.removeEventListener("mouseleave", onLeave);
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
/* 与主窗口 side-thumb 同款：6px 胶囊 / 实色 C4 常态 C5 悬停（透明色交叉叠加作废，2026-08-10 定案）；
   距右 2px（悬浮不占布局，避开 inset 边框线与通栏分隔线端点） */
.scroll-thumb {
  position: absolute;
  right: 2px;
  width: 6px;
  border-radius: var(--radius-pill);
  background: var(--c4);
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
  background: var(--c5);
}
</style>
