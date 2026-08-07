<script setup lang="ts">
// 快捷键徽章（figma-css 04_右键菜单 item / 06_悬停 tip）：C3 底 + 暗-悬停边框 radius 6，标签字 标签档；
// 多字母 padding 2/4，单字母 padding 2/6（呼吸空间，figma 注）。
import { computed } from "vue";

const props = defineProps<{ label: string }>();
const single = computed(() => props.label.length <= 1);
</script>

<template>
  <kbd class="kbd" :class="{ single }">{{ label }}</kbd>
</template>

<style scoped>
.kbd {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: var(--space-2) var(--space-4);
  border-radius: var(--radius-6);
  background: var(--c3);
  box-shadow: inset 0 0 0 1px var(--line-hover);
  color: var(--text-label);
  font-family: inherit;
  font-size: var(--text-label-size);
  line-height: var(--text-label-line);
  flex: none;
}
.kbd.single {
  padding: var(--space-2) var(--space-6);
}
</style>
