<script setup lang="ts">
// 菜单分割线（figma-css 04_右键菜单）：内边距 2 0，线条 1px 暗-悬停（去圆头——1px 线圆头上下各吃约 0.5px 抗锯齿，会渲染成粗细不一）。
</script>

<template>
  <div class="menu-divider" />
</template>

<style scoped>
.menu-divider {
  padding: var(--space-2) 0;
}
.menu-divider::before {
  content: "";
  display: block;
  height: 1px;
  background: var(--line-hover);
}
</style>
