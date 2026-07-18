<script setup lang="ts">
// 文件树 item（token 4）：展开按钮 + 基础 item（图标 + 标题）+ 数量标签。
// 无子项时展开按钮保留占位（透明度 0），保证装饰图标竖向对齐（token 4 备注）。
import AppIcon from "./AppIcon.vue";

withDefaults(
  defineProps<{
    icon: string;
    label: string;
    count?: number;
    depth?: number;
    expandable?: boolean;
    open?: boolean;
    selected?: boolean;
  }>(),
  { count: undefined, depth: 0, expandable: false, open: false, selected: false },
);
const emit = defineEmits<{ toggle: []; select: [] }>();
</script>

<template>
  <div class="tree-item" :style="{ paddingLeft: `${depth * 12}px` }">
    <button class="expand" :class="{ placeholder: !expandable }" @click.stop="emit('toggle')">
      <AppIcon name="chevron-small" :size="8" class="chev" :class="{ closed: !open }" />
    </button>
    <button class="body" :class="{ selected }" @click="emit('select')">
      <AppIcon :name="icon" class="deco" />
      <span class="label">{{ label }}</span>
      <span v-if="count !== undefined" class="count">{{ count.toLocaleString() }}</span>
    </button>
  </div>
</template>

<style scoped>
.tree-item {
  display: flex;
  align-items: center;
}
/* 展开按钮：图标 8×8 C4-40，旋转动画 */
.expand {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 14px;
  height: 28px;
  color: var(--c4);
  flex: none;
}
.expand.placeholder {
  opacity: 0;
  pointer-events: none;
}
.chev {
  transition: transform 0.15s ease;
}
.chev.closed {
  transform: rotate(-90deg);
}
.body {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  flex: 1;
  min-width: 0;
  padding: var(--space-4);
  border-radius: var(--radius-6);
  color: var(--c4);
  transition: background 0.12s ease;
}
.body:hover {
  background: var(--c4-7);
}
.body.selected {
  background: var(--c4-15);
}
/* 选中：装饰图标线条 C4、数量标签 C4 */
.body.selected .deco :deep(svg [stroke-opacity="0.4"]),
.body.selected .deco :deep(svg [fill-opacity="0.4"]) {
  stroke-opacity: 1;
  fill-opacity: 1;
}
.label {
  flex: 1;
  min-width: 0;
  text-align: left;
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.count {
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--c4-40);
  flex: none;
}
.body.selected .count {
  color: var(--c4);
}
</style>
