<script setup lang="ts">
// 图标组件：内联 SVG（currentColor / 文件夹卡片 var() 均要求内联）。
// 颜色继承父级 color（font-icon 档位）；SVG 线条层一律烘 100%，明暗全由 color 表达，
// 仅装饰填充层烘 0.12（= font-icon-4）——2026-07-23 废弃旧体系"SVG 烘透明度 + 父级 :deep 提 opacity"。
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
