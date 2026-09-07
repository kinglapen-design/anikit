<script setup lang="ts">
// 拆分条：拖拽调整侧栏宽度。样式 2026-09-07 实机走查第一批条 4/5 用户供稿：
// 左栏 = 亮色/四级渐变线（沿条长两头 0%、中间 50%）；右栏（详情面板侧）= 零绘制
// 简约——无常态线、无悬停效果，仅 col-resize 指针提示。
import { ref } from "vue";
import { useUiStore } from "../stores/ui";

const props = defineProps<{ side: "left" | "right" }>();
const ui = useUiStore();

const dragging = ref(false);
let startX = 0;
let startW = 0;

function onDown(e: PointerEvent): void {
  dragging.value = true;
  startX = e.clientX;
  startW = props.side === "left" ? ui.leftWidth : ui.rightWidth;
  (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  e.preventDefault();
}
function onMove(e: PointerEvent): void {
  if (!dragging.value) return;
  const delta = e.clientX - startX;
  const w = props.side === "left" ? startW + delta : startW - delta;
  // 左栏最小 246（2026-08-30 用户实机：再窄菜单栏四文字钮被裁，246 = 菜单栏恰好全显宽）；右栏 200
  const min = props.side === "left" ? 246 : 200;
  const clamped = Math.max(min, Math.min(480, w));
  if (props.side === "left") ui.leftWidth = clamped;
  else ui.rightWidth = clamped;
}
function onUp(e: PointerEvent): void {
  if (!dragging.value) return;
  dragging.value = false;
  (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
}
</script>

<template>
  <div
    class="split-bar"
    :class="[side, { dragging }]"
    style="touch-action: none"
    @pointerdown="onDown"
    @pointermove="onMove"
    @pointerup="onUp"
  >
    <span v-if="side === 'left'" class="line" />
  </div>
</template>

<style scoped>
/* 三栏贴边（间距 0）：拆分条负 margin 重叠在 1px 分割线上、不占布局 */
.split-bar {
  width: 4px;
  margin: 0 -2px;
  display: flex;
  justify-content: center;
  cursor: col-resize;
  flex: none;
  z-index: 20;
}
/* 左栏拆分条线：亮色/四级渐变——沿条长方向两头 0%、中间 50%（实机走查条 4 用户供稿，
   替代旧「常态透明 + 悬停品牌色高亮」，旧 token 一并清出） */
.line {
  width: 2px;
  height: 100%;
  border-radius: 1px;
  background: linear-gradient(
    to bottom,
    transparent 0%,
    color-mix(in srgb, var(--n-l4) 50%, transparent) 50%,
    transparent 100%
  );
}
/* 右栏拆分条：零绘制（条 5 用户定——不需要悬停效果，简约，仅指针提示），
   模板不渲染 .line，此处无样式 */
</style>
