<script setup lang="ts">
// 色块 item（颜色列表）——docs/UI 重构/Design Token/组件/item/色块 item/
// 壳 = 横排 padding 4 / gap 4 / 圆角 14，figma-css 无 background → 透明布局壳；
// 色块 = 16×16 圆形（figma-css 未写 radius，以稿图为准）+ 1px 亮色/半透明-1 描边；
// hover 单块放大 1.25（16→20，2026-08-29 用户定），transform 不影响布局、不撑容器。
withDefaults(
  defineProps<{
    colors: string[]; // #RRGGBB，按占比降序（业务侧截前 7）
    forceState?: "" | "hover"; // 预览页强制态：全块放大示意（实际交互仅悬停块放大）
  }>(),
  { forceState: "" },
);
</script>

<template>
  <div class="color-swatches" :class="forceState ? `force-${forceState}` : ''">
    <span v-for="c in colors" :key="c" class="cs-dot" :style="{ background: c }" />
  </div>
</template>

<style scoped>
.color-swatches {
  display: inline-flex;
  flex-direction: row;
  align-items: center;
  padding: var(--n-space-4);
  gap: var(--n-space-4);
  border-radius: 14px;
}
.cs-dot {
  box-sizing: border-box;
  width: 16px;
  height: 16px;
  flex: none;
  border-radius: 50%;
  border: 1px solid var(--n-l-a1);
  transition: transform var(--n-dur-150) var(--n-ease-in-out);
}
/* 放大块提层：避免被 DOM 后继块盖住边缘 */
.cs-dot:hover {
  position: relative;
  z-index: 1;
  transform: scale(1.25);
}
.color-swatches.force-hover .cs-dot {
  transform: scale(1.25);
}
</style>
