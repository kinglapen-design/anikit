<script setup lang="ts">
// 开关按钮（figma-css 01_按钮 1.9）：34×18 圆钮 14，padding 2，胶囊圆角；
// 关 = C4 底 + 暗-悬停边框（hover C5），开 = 品牌色底、圆钮右移（右开左关，用户注明 figma 方向画反）；
// 圆钮 = 文字高亮色，位移动画（00_要求：切换按钮要有位移动画）。
const model = defineModel<boolean>({ required: true });
</script>

<template>
  <button class="app-switch" :class="{ on: model }" @click="model = !model">
    <span class="knob" />
  </button>
</template>

<style scoped>
.app-switch {
  display: flex;
  align-items: center;
  width: 34px;
  height: 18px;
  padding: var(--space-2);
  border-radius: var(--radius-pill);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--line-hover);
  flex: none;
  transition: background 0.2s ease;
}
.app-switch:hover {
  background: var(--c5);
}
.app-switch.on {
  background: var(--brand);
  box-shadow: none;
}
.app-switch.on:hover {
  background: var(--brand);
}
.knob {
  width: 14px;
  height: 14px;
  border-radius: var(--radius-circle);
  background: var(--text-highlight);
  flex: none;
  transition: transform 0.2s ease;
}
/* 开：圆钮平移至右端（轨道宽 - 圆钮 - 两侧 padding） */
.app-switch.on .knob {
  transform: translateX(16px);
}
</style>
