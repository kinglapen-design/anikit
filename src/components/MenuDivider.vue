<script setup lang="ts">
// 菜单分割线（02_通用组件/02_item 04）：内边距 2 0，线条 1px C4-15 圆头。
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
  border-radius: 1px;
  background: var(--c4-15);
}
</style>
