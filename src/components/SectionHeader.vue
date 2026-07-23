<script setup lang="ts">
// 区块副标题 item（02_通用组件/02_item 05）：展开按钮（小图标 24×24）+ 标题 + 数量标签，
// 可选添加按钮；v-model:open 控制下方内容展开 / 折叠。
// pad 由布局处给（如左栏快速访问 0 8 0 16、文件夹 0 10 0 16）。
import AppIcon from "./AppIcon.vue";

withDefaults(defineProps<{ title: string; count?: number; addable?: boolean; pad?: string }>(), {
  count: undefined,
  addable: false,
  pad: "0",
});
const open = defineModel<boolean>("open", { default: true });
const emit = defineEmits<{ add: [] }>();
</script>

<template>
  <div class="section-header" :style="{ padding: pad }">
    <button class="head-btn" @click="open = !open">
      <AppIcon name="chevron-small" class="chev" :class="{ closed: !open }" />
    </button>
    <span class="title">{{ title }}</span>
    <span v-if="count !== undefined" class="count">({{ count.toLocaleString() }})</span>
    <span class="spacer" />
    <button v-if="addable" class="head-btn" @click.stop="emit('add')">
      <AppIcon name="add-small" />
    </button>
  </div>
</template>

<style scoped>
.section-header {
  display: flex;
  align-items: center;
  gap: var(--space-4);
}
.head-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  padding: var(--space-4);
  border-radius: var(--radius-4);
  color: var(--font-icon-1);
  flex: none;
  transition: background 0.12s ease;
}
/* 容器内图标 font-icon-3（02_item 05：SVG 烘 0.7，此处压到 0.4） */
.head-btn :deep(svg [stroke-opacity="0.7"]),
.head-btn :deep(svg [fill-opacity="0.7"]) {
  stroke-opacity: 0.4;
  fill-opacity: 0.4;
}
/* hover：底色 font-icon-6，图标 font-icon-2（:hover 伪类提权，覆盖常态 0.4 恢复 SVG 烘的 0.7） */
.head-btn:hover {
  background: var(--font-icon-6);
}
.head-btn:hover :deep(svg [stroke-opacity="0.7"]),
.head-btn:hover :deep(svg [fill-opacity="0.7"]) {
  stroke-opacity: 0.7;
  fill-opacity: 0.7;
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
  color: var(--font-icon-3);
  white-space: nowrap;
}
.spacer {
  flex: 1;
}
</style>
