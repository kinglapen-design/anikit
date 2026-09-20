<script setup lang="ts">
// 滑块（figma 41_工具栏：100×10）：滑轨 3px 暗-悬停，拖钮 10×10 文字-常态档（2026-08-08 用户定：
// 宽固定 100、拖钮不用高亮档）。
// 拖动中实时更新 v-model，pointerup 时 emit commit（用于触发布局重算等重操作）。
import { computed, ref } from "vue";

const props = withDefaults(defineProps<{ min?: number; max?: number }>(), { min: 0, max: 100 });
const model = defineModel<number>({ required: true });
const emit = defineEmits<{ commit: [value: number] }>();

const trackRef = ref<HTMLElement>();
let dragging = false;

const pct = computed(() => ((model.value - props.min) / (props.max - props.min)) * 100);

function apply(e: PointerEvent): void {
  const rect = trackRef.value!.getBoundingClientRect();
  const ratio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
  model.value = Math.round(props.min + ratio * (props.max - props.min));
}

function onDown(e: PointerEvent): void {
  dragging = true;
  trackRef.value!.setPointerCapture(e.pointerId);
  apply(e);
  e.preventDefault();
}
function onMove(e: PointerEvent): void {
  if (dragging) apply(e);
}
function onUp(e: PointerEvent): void {
  if (!dragging) return;
  dragging = false;
  trackRef.value!.releasePointerCapture(e.pointerId);
  emit("commit", model.value);
}
</script>

<template>
  <div
    ref="trackRef"
    class="side-slider"
    style="touch-action: none"
    @pointerdown="onDown"
    @pointermove="onMove"
    @pointerup="onUp"
  >
    <span class="thumb" :style="{ left: `${pct}%` }" />
  </div>
</template>

<style scoped>
/* 总容器：宽 100 高 10（figma 41）；滑轨 3px 线（垂直居中），拖钮 10×10 */
.side-slider {
  position: relative;
  width: 100px;
  height: 10px;
  flex: none;
  cursor: pointer;
}
.side-slider::before {
  content: "";
  position: absolute;
  left: 0;
  right: 0;
  top: 50%;
  height: 3px;
  transform: translateY(-50%);
  border-radius: 2px;
  background: var(--n-l-a3); /* 滑轨 = 暗-悬停（figma 41） */
}
.thumb {
  position: absolute;
  top: 50%;
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--n-l2); /* 拖钮 = 文字/常态（2026-08-08 用户定，不用高亮档） */
  transform: translate(-50%, -50%);
  pointer-events: none;
}
</style>
