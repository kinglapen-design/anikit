<script setup lang="ts">
// 区块副标题（token 5）：展开按钮 + 标题 + 数量标签，可选添加按钮。
// 展开箭头带旋转动画；v-model:open 控制下方内容展开 / 折叠。
import AppIcon from "./AppIcon.vue";

withDefaults(defineProps<{ title: string; count?: number; addable?: boolean }>(), {
  count: undefined,
  addable: false,
});
const open = defineModel<boolean>("open", { default: true });
const emit = defineEmits<{ add: [] }>();
</script>

<template>
  <div class="section-header">
    <button class="head-btn" @click="open = !open">
      <AppIcon name="chevron-small" :size="8" class="chev" :class="{ closed: !open }" />
    </button>
    <span class="title">{{ title }}</span>
    <span v-if="count !== undefined" class="count">({{ count.toLocaleString() }})</span>
    <span class="spacer" />
    <button v-if="addable" class="head-btn" @click.stop="emit('add')">
      <AppIcon name="add-small" :size="8" />
    </button>
  </div>
</template>

<style scoped>
.section-header {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  /* 组件布局：区块副标题内边距 0 8 0 12 */
  padding: 0 var(--space-8) 0 var(--space-12);
}
.head-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  padding: var(--space-6);
  border-radius: var(--radius-6);
  color: var(--c4);
  flex: none;
  transition: background 0.12s ease;
}
.head-btn:hover {
  background: var(--c4-7);
}
/* 悬停图标高亮：烘的 0.4 提到 1 */
.head-btn:hover :deep(svg [stroke-opacity="0.4"]),
.head-btn:hover :deep(svg [fill-opacity="0.4"]) {
  stroke-opacity: 1;
  fill-opacity: 1;
}
.chev {
  transition: transform 0.15s ease;
}
.chev.closed {
  transform: rotate(-90deg);
}
.title,
.count {
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--c4-40);
  white-space: nowrap;
}
.spacer {
  flex: 1;
}
</style>
