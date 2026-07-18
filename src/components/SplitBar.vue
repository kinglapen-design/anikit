<script setup lang="ts">
// 拆分条：拖拽调整侧栏宽度；悬停 / 拖拽时 2px 品牌色-1 高亮（token 线框）。
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
  const clamped = Math.max(200, Math.min(480, w));
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
    :class="{ dragging }"
    style="touch-action: none"
    @pointerdown="onDown"
    @pointermove="onMove"
    @pointerup="onUp"
  >
    <span class="line" />
  </div>
</template>

<style scoped>
/* 占用主窗口子项间距的 4px 槽位 */
.split-bar {
  width: 4px;
  display: flex;
  justify-content: center;
  cursor: col-resize;
  flex: none;
}
.line {
  width: 2px;
  border-radius: 1px;
  background: transparent;
  transition: background 0.12s ease;
}
.split-bar:hover .line,
.split-bar.dragging .line {
  background: var(--brand-1);
}
</style>
