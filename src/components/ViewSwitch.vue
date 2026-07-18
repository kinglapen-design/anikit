<script setup lang="ts">
// 视图切换按钮组（token 13）：永远有一个选中，切换时选中底色 C4-15 平移动画。
import { computed } from "vue";
import AppIcon from "./AppIcon.vue";

export interface ViewOption {
  value: string;
  icon: string;
  title: string;
}

const props = defineProps<{ options: ViewOption[] }>();
const model = defineModel<string>({ required: true });
const index = computed(() => Math.max(0, props.options.findIndex((o) => o.value === model.value)));
</script>

<template>
  <div class="view-switch">
    <span class="indicator" :style="{ transform: `translateX(${index * 34}px)` }" />
    <button
      v-for="o in options"
      :key="o.value"
      class="item"
      :class="{ on: o.value === model }"
      :title="o.title"
      @click="model = o.value"
    >
      <AppIcon :name="o.icon" />
    </button>
  </div>
</template>

<style scoped>
.view-switch {
  position: relative;
  display: flex;
  gap: var(--space-6);
  padding: var(--space-6);
  border-radius: var(--radius-8);
  background: var(--c1-50);
  border: 1px solid var(--c4-15);
  width: max-content;
}
/* 选中底色：按按钮步距（28 + 间距 6 = 34px）平移 */
.indicator {
  position: absolute;
  left: var(--space-6);
  top: var(--space-6);
  width: 28px;
  height: 28px;
  border-radius: var(--radius-6);
  background: var(--c4-15);
  transition: transform 0.18s ease;
}
.item {
  position: relative;
  z-index: 1;
  width: 28px;
  height: 28px;
  padding: var(--space-6);
  border-radius: var(--radius-6);
  color: var(--c4);
  flex: none;
  transition: background 0.12s ease;
}
.item:hover {
  background: var(--c4-7);
}
/* 选中图标高亮（C4-40 → C4） */
.item.on :deep(svg [stroke-opacity="0.4"]),
.item.on :deep(svg [fill-opacity="0.4"]) {
  stroke-opacity: 1;
  fill-opacity: 1;
}
</style>
