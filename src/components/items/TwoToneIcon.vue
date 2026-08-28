<script setup lang="ts">
// 双色调结构图标（文件夹 / 档案 / 图片 / 警告-small）：文件保留 Figma 原值，
// getTwoToneIcon 换 --tone-1 / --tone-2 变量通道；tint = 自定义九色对（树文件夹设色用）。
import { computed } from "vue";
import { getTwoToneIcon } from "../../lib/icons";

const props = withDefaults(defineProps<{ name: string; size?: number; tint?: string }>(), {
  size: 18,
});
const svg = computed(() => getTwoToneIcon(props.name));
</script>

<template>
  <span
    class="tt-icon"
    :style="{ width: `${size}px`, height: `${size}px` }"
    :data-tint="tint || undefined"
    v-html="svg"
  />
</template>

<style scoped>
.tt-icon {
  display: inline-flex;
  flex: none;
  --tone-1: var(--n-custom-1);
  --tone-2: var(--n-custom-2);
}
.tt-icon[data-tint="red"] {
  --tone-1: var(--n-custom-red-1);
  --tone-2: var(--n-custom-red-2);
}
.tt-icon[data-tint="blue"] {
  --tone-1: var(--n-custom-blue-1);
  --tone-2: var(--n-custom-blue-2);
}
.tt-icon[data-tint="yellow"] {
  --tone-1: var(--n-custom-yellow-1);
  --tone-2: var(--n-custom-yellow-2);
}
.tt-icon[data-tint="yellowgreen"] {
  --tone-1: var(--n-custom-yellowgreen-1);
  --tone-2: var(--n-custom-yellowgreen-2);
}
.tt-icon[data-tint="green"] {
  --tone-1: var(--n-custom-green-1);
  --tone-2: var(--n-custom-green-2);
}
.tt-icon[data-tint="cyan"] {
  --tone-1: var(--n-custom-cyan-1);
  --tone-2: var(--n-custom-cyan-2);
}
.tt-icon[data-tint="purple"] {
  --tone-1: var(--n-custom-purple-1);
  --tone-2: var(--n-custom-purple-2);
}
.tt-icon[data-tint="pink"] {
  --tone-1: var(--n-custom-pink-1);
  --tone-2: var(--n-custom-pink-2);
}
.tt-icon[data-tint="peach"] {
  --tone-1: var(--n-custom-peach-1);
  --tone-2: var(--n-custom-peach-2);
}
.tt-icon :deep(svg) {
  width: 100%;
  height: 100%;
  display: block;
}
</style>
