<script setup lang="ts">
// 滑块（token 10，无百分比）：滑轨 3px / 120px 胶囊 C4-15，拖钮 10×10 C4。
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
.side-slider {
  position: relative;
  width: 120px;
  height: 16px;
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
  background: var(--c4-15);
}
.thumb {
  position: absolute;
  top: 50%;
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--c4);
  transform: translate(-50%, -50%);
  pointer-events: none;
}
</style>
