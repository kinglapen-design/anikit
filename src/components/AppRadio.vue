<script setup lang="ts">
// 单选框（figma-css 01_按钮 1.5）：外圈 20×20 圆形（C4 底 + 暗-悬停边框，hover C5）；
// 选中圆形 16×16 品牌色 + 亮-选中边框，中心缩放——弹出带轻微过冲回弹（00_要求）；
// 收回（取消选中）不回弹、纯缓入快收（2026-08-12 用户：回弹缩小到一个点收得不干脆）。
// 标题 常态档。v-model 值不等时点击选中（成组使用，每组一个 v-model 值）。
import { computed } from "vue";

const model = defineModel<string | number | boolean>({ required: true });
const props = defineProps<{ value: string | number | boolean; label?: string }>();
const checked = computed(() => model.value === props.value);
</script>

<template>
  <button class="app-radio" :class="{ on: checked }" @click="model = value">
    <span class="ring">
      <span class="dot" />
    </span>
    <span v-if="label" class="label">{{ label }}</span>
  </button>
</template>

<style scoped>
.app-radio {
  display: inline-flex;
  align-items: center;
  gap: var(--space-6);
  padding: 0;
  height: 20px;
  color: var(--text-normal);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  flex: none;
}
.ring {
  width: 20px;
  height: 20px;
  border-radius: var(--radius-circle);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--line-hover);
  display: flex;
  align-items: center;
  justify-content: center;
  flex: none;
  transition: background 0.15s ease;
}
.app-radio:hover .ring {
  background: var(--c5);
}
.dot {
  width: 16px;
  height: 16px;
  border-radius: var(--radius-circle);
  background: var(--brand);
  box-shadow: inset 0 0 0 1px var(--line-active);
  transform: scale(0);
  /* 收回（取消选中）：纯缓入、缩短时长，收得干脆不回弹 */
  transition: transform 0.15s cubic-bezier(0.42, 0, 1, 1);
}
.app-radio.on .dot {
  transform: scale(1);
  /* 弹出（选中）：过冲回弹（00_要求） */
  transition: transform 0.25s cubic-bezier(0.34, 1.56, 0.64, 1);
}
</style>
