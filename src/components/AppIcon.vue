<script setup lang="ts">
// 图标组件：内联 SVG（currentColor / 文件夹卡片 var() 均要求内联）。
// 颜色一律 color: var(--c4)，明暗由 SVG 烘的透明度决定（线条 0.4 / 填充 0.07）；
// 需要高亮（悬停 / 选中）时由父级 :deep 覆盖 opacity 至 1。
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
