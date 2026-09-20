<script setup lang="ts">
// 区块标题 item（figma-css 05_区块标题 item）：高 20，标题+数量（标签字 标签档）+ 展开箭头 12 + 添加按钮；
// group 变体 = 分组标题（高 30，标题档 700 常态 + 分组菜单按钮 group-menu 30×30）；
// 整行（标题/数量/箭头）为折叠判定区；添加按钮独立；pad 由布局处给；
// grip 变体（详情面板）：折叠区整行 hover 抓手 + 标题提亮，为拖拽排序准备。
import AppIcon from "./AppIcon.vue";
import { dragView } from "../lib/dragOperation";

withDefaults(
  defineProps<{
    title: string;
    count?: number;
    addable?: boolean;
    group?: boolean;
    pad?: string;
    grip?: boolean;
  }>(),
  { count: undefined, addable: false, group: false, pad: "0", grip: false },
);
const open = defineModel<boolean>("open", { default: true });
const emit = defineEmits<{ add: []; menu: [e: MouseEvent] }>();

// 折叠切换：有效拖拽结束后的紧随 click 不触发（2026-08-14 区块子系统——区块标题可拖拽排序，
// 拖完松手的 click 落在 fold-area 上会误折叠；与树 item suppressClick 同口径）
function onFold(): void {
  if (dragView.suppressClick) return;
  open.value = !open.value;
}
</script>

<template>
  <div class="section-header" :class="{ grip, group }" :style="{ padding: pad }">
    <button class="fold-area" @click="onFold">
      <span class="title">{{ title }}</span>
      <span v-if="count !== undefined" class="count">- {{ count.toLocaleString() }}</span>
      <span class="chev-slot">
        <AppIcon name="tree-expand-arrow" :size="12" class="chev" :class="{ closed: !open }" />
      </span>
    </button>
    <span class="spacer" />
    <!-- tail 插槽：标题行右侧自定义内容（figma 42 文件夹区块 = 递归显示复选框，2026-08-10） -->
    <slot name="tail" />
    <button v-if="group" class="group-menu-btn" v-tip="'分组菜单'" @click.stop="emit('menu', $event)">
      <AppIcon name="group-menu" :size="18" />
    </button>
    <button v-else-if="addable" class="add-btn" v-tip="'添加'" @click.stop="emit('add')">
      <AppIcon name="add-small" :size="12" />
    </button>
  </div>
</template>

<style scoped>
.section-header {
  display: flex;
  align-items: center;
  gap: var(--n-space-4);
  height: 20px;
}
.section-header.group {
  height: 30px;
}
/* 折叠判定区：标题 + 数量 + 箭头整行可点 */
.fold-area {
  display: inline-flex;
  align-items: center;
  gap: var(--n-space-6);
  border-radius: var(--n-radius-4);
}
.chev-slot {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 12px;
  height: 12px;
  color: var(--n-l3);
  flex: none;
}
.chev {
  transition: transform 0.15s ease;
}
/* 箭头源图形向下：折叠 = 朝右 */
.chev.closed {
  transform: rotate(-90deg);
}
.title,
.count {
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  color: var(--n-l3);
  white-space: nowrap;
  transition: color 0.15s ease;
}
/* 分组标题：标题档 700 常态 */
.section-header.group .title,
.section-header.group .count {
  font-size: var(--n-fs-btn-lg);
  line-height: var(--n-lh-btn-lg);
  font-weight: var(--n-fw-bold);
  color: var(--n-l2);
}
.spacer {
  flex: 1;
}
/* 添加按钮：20×20 圆角 4，标签档，hover 叠暗-悬停 */
.add-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  border-radius: var(--n-radius-4);
  color: var(--n-l3);
  flex: none;
  transition: background 0.15s ease;
}
.add-btn:hover {
  background: var(--n-l-a3);
}
/* 分组菜单按钮：30×30 圆角 6，标签档，hover 叠暗-悬停 */
.group-menu-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  border-radius: var(--n-radius-6);
  color: var(--n-l3);
  flex: none;
  transition: background 0.15s ease;
}
.group-menu-btn:hover {
  background: var(--n-l-a3);
}
/* grip 变体（详情面板，2026-07-23 用户定）：折叠区整行 hover 抓手 + 标题/箭头提亮，无底色 */
.section-header.grip .fold-area {
  cursor: grab;
}
.section-header.grip:hover .fold-area .title,
.section-header.grip:hover .fold-area .count,
.section-header.grip:hover .fold-area .chev-slot {
  color: var(--n-l1);
}
</style>
