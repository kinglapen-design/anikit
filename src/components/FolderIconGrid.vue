<script setup lang="ts">
// 图标网格（05 稿 §3 共用体，2026-08-18 自 FolderIconPanel 抽出）：20 格 = 默认（folder-closed）
// + CUSTOM_ICONS 19 个，5 列（item 30×30 padding 6 圆角 6，图标 18 常态色，列 gap 8 行 gap 12）；
// 当前项选中 = 亮-选中背景；hover 叠暗-悬停，选中项 hover 不再变色（速查表 §3）。
// 两处复用：树菜单「更改图标」面板（FolderIconPanel，带应用至子级 + 执行命令）/
// 区块管理行「默认图标」下拉（openPanelMenu 整体面板菜单，纯选择器）。
import AppIcon from "./AppIcon.vue";
import { CUSTOM_ICONS } from "../lib/customIcons";

defineProps<{ current: string | null }>(); // 当前图标（null = 默认，首格选中）
const emit = defineEmits<{ select: [name: string | null] }>();
</script>

<template>
  <div class="icon-grid">
    <button class="cell" :class="{ cur: current === null }" @click="emit('select', null)">
      <AppIcon name="folder-closed" :size="18" />
    </button>
    <button v-for="def in CUSTOM_ICONS" :key="def.name" class="cell" :class="{ cur: current === def.name }" @click="emit('select', def.name)">
      <AppIcon :name="def.name" :size="18" />
    </button>
  </div>
</template>

<style scoped>
/* 5 列网格（05 稿 §3：item 30×30、列 gap 8、行 gap 12） */
.icon-grid {
  display: grid;
  grid-template-columns: repeat(5, 30px);
  gap: var(--space-12) var(--space-8);
}
.cell {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  padding: 6px;
  border-radius: var(--radius-6);
  color: var(--text-normal);
}
/* hover 叠 暗-悬停；选中 = 亮-选中背景（稿 §3 首格样例），选中项 hover 不再变色 */
.cell:hover:not(.cur) {
  background: linear-gradient(var(--line-hover), var(--line-hover));
}
.cell.cur {
  background: var(--line-active);
}
</style>
