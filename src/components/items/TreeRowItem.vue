<script setup lang="ts">
// 树容器 item 族（文件夹 / 快速访问 / 档案）——docs/UI 重构/Design Token/组件/item/树容器 item/
// 壳：230 宽 × 26 高 / padding 4 6 / gap 6 / 圆角 6；hover = 亮色/半透明-3；
// 选中 = 半透明-2 + 立体感-通用 + 展开钮/提示装饰/名称/数量提亮 亮色/一级。
import AppIcon from "../AppIcon.vue";
import TwoToneIcon from "./TwoToneIcon.vue";

withDefaults(
  defineProps<{
    kind: "folder" | "archive" | "quick"; // 文件夹 / 档案 / 快速访问
    name: string;
    count?: number | string; // 数量（数字档；不传则不渲染）
    icon?: string; // quick 型图标名（01_常用 线条图标）
    expanded?: boolean; // folder 型：展开钮朝向（∨ / >）
    selected?: boolean;
    tint?: string; // folder/archive 型图标自定义九色（red/blue/...；空 = 常态双色）
    forceState?: "" | "hover" | "selected"; // 预览页强制态
  }>(),
  { expanded: false, selected: false, count: undefined, icon: "", tint: "", forceState: "" },
);

const emit = defineEmits<{ toggle: []; click: [] }>();
</script>

<template>
  <div
    class="tree-item"
    :class="[{ selected }, forceState ? `force-${forceState}` : '']"
    @click="emit('click')"
  >
    <span class="lead">
      <button
        v-if="kind === 'folder'"
        class="exp"
        :class="{ open: expanded }"
        @click.stop="emit('toggle')"
      >
        <AppIcon name="expand" :size="14" />
      </button>
      <span v-else-if="kind === 'archive'" class="dot" aria-hidden="true" />
      <TwoToneIcon v-if="kind === 'folder'" name="folder-closed" :size="18" :tint="tint" />
      <TwoToneIcon v-else-if="kind === 'archive'" name="archive" :size="18" :tint="tint" />
      <span v-else-if="kind === 'quick' && icon" class="qico"><AppIcon :name="icon" :size="18" /></span>
      <span class="nm">{{ name }}</span>
    </span>
    <span v-if="count !== undefined" class="cnt">{{ count }}</span>
  </div>
</template>

<style scoped>
.tree-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--n-space-6);
  width: 230px;
  height: 26px;
  padding: var(--n-space-4) var(--n-space-6);
  border-radius: var(--n-radius-6);
  cursor: pointer;
  color: var(--n-l2);
  transition: background var(--n-dur-100) var(--n-ease-in-out);
}
.tree-item:hover,
.tree-item.force-hover {
  background: var(--n-l-a3);
}
.tree-item.selected,
.tree-item.force-selected {
  background: var(--n-l-a2);
  box-shadow: var(--n-effect-raised);
  color: var(--n-l1);
}
.lead {
  display: flex;
  align-items: center;
  gap: var(--n-space-6);
  min-width: 0;
}
.exp {
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  flex: none;
  padding: 0;
  color: var(--n-l4);
  transition: transform var(--n-dur-100) var(--n-ease-in-out);
}
/* 点击热区外扩（2026-08-28 走查通用问题，旧 TreeItem 同款机制）：
   上/下 -4 吃满 item padding、左 -6 吃到 item 左缘、右 -3 吃箭头与图标间距一半 */
.exp::after {
  content: "";
  position: absolute;
  inset: -4px -3px -4px -6px;
}
.exp:not(.open) {
  transform: rotate(-90deg); /* 收起 = >，展开 = ∨（expand.svg 源图形朝下） */
}
.selected .exp,
.force-selected .exp {
  color: var(--n-l1);
}
.dot {
  width: 18px;
  height: 18px;
  flex: none;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
.dot::before {
  content: "";
  width: 8px;
  height: 8px;
  border-radius: 50%; /* 提示装饰：稿 CSS 为 8×8 方块，图实为圆点（以图为准） */
  background: var(--n-l4);
}
.selected .dot::before,
.force-selected .dot::before {
  background: var(--n-l1);
}
.qico {
  display: inline-flex;
  width: 18px;
  height: 18px;
  flex: none;
  color: var(--n-l3);
}
.selected .qico,
.force-selected .qico {
  color: var(--n-l1);
}
.nm {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
}
.cnt {
  flex: none;
  font-family: var(--n-font-mono);
  font-size: var(--n-fs-num);
  line-height: var(--n-lh-num);
  font-weight: var(--n-fw-medium);
  color: var(--n-l3);
}
.selected .cnt,
.force-selected .cnt {
  color: var(--n-l1);
}
</style>
