<script setup lang="ts">
// 图标组件：内联 SVG（currentColor / var() 均要求内联）。
// 颜色约定（2026-07-23 新体系）：线条层一律 100%（不烘透明度），fill="currentColor" 吃父级
// color（font-icon 档位表达明暗）；装饰填充层 fill="var(--font-icon-4)" 恒 12% 不随 color。
import { computed } from "vue";
import { getIcon } from "../lib/icons";

const props = withDefaults(defineProps<{ name: string; size?: number }>(), { size: 16 });
const svg = computed(() => getIcon(props.name));
</script>

<template>
  <span class="app-icon" :style="{ width: `${size}px`, height: `${size}px` }" v-html="svg" />
</template>

<style scoped>
.app-icon {
  display: inline-flex;
  flex: none;
}
.app-icon :deep(svg) {
  width: 100%;
  height: 100%;
  display: block;
}
</style>
