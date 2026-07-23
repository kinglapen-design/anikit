<script setup lang="ts">
// 区块副标题 item（02_通用组件/02_item 05）：标题 + 数量标签 + 展开按钮（小图标 24×24，
// 2026-07-23 用户定：箭头与标题交换位置），可选添加按钮；v-model:open 控制下方内容展开 / 折叠。
// pad 由布局处给（如左栏快速访问 0 8 0 16、文件夹 0 10 0 16）。
import AppIcon from "./AppIcon.vue";

withDefaults(defineProps<{ title: string; count?: number; addable?: boolean; pad?: string; grip?: boolean }>(), {
  count: undefined,
  addable: false,
  pad: "0",
  grip: false,
});
const open = defineModel<boolean>("open", { default: true });
const emit = defineEmits<{ add: [] }>();
</script>

<template>
  <div class="section-header" :class="{ grip }" :style="{ padding: pad }">
    <span class="title">{{ title }}</span>
    <span v-if="count !== undefined" class="count">({{ count.toLocaleString() }})</span>
    <button class="head-btn" @click="open = !open">
      <AppIcon name="chevron-small" class="chev" :class="{ closed: !open }" />
    </button>
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
  /* 区块副标题图标 60%（2026-07-23 用户定：font-icon-3；SVG 一律烘 100%，明暗全由 color 档位表达） */
  color: var(--font-icon-3);
  flex: none;
  transition:
    background 0.12s ease,
    color 0.12s ease;
}
/* hover：底色 font-icon-6，图标提亮 font-icon-2 */
.head-btn:hover {
  background: var(--font-icon-6);
  color: var(--font-icon-2);
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
  transition: color 0.12s ease;
}
.spacer {
  flex: 1;
}
/* 详情面板变体（2026-07-23 用户定）：整行 hover 抓手（为拖拽排序准备），
   展开箭头 + 标题提亮 font-icon-1，无底色 */
.section-header.grip,
.section-header.grip .head-btn {
  cursor: grab;
}
.section-header.grip:hover .title,
.section-header.grip:hover .count,
.section-header.grip:hover .head-btn {
  color: var(--font-icon-1);
}
.section-header.grip:hover .head-btn {
  background: none;
}
</style>
