<script setup lang="ts">
// 图标网格（05 稿 §3 共用体，2026-08-18 自 FolderIconPanel 抽出）：30 格 = 默认（folder-closed）
// + CUSTOM_ICONS 29 个（02_自定义 新双色集 2026-09-01 二十五轮接入；archive 档案专属同日出清单），
// 5 列（item 30×30 padding 6 圆角 6，图标 18 双色原值灰，gap 6 行列统一——2026-09-01 用户定，废列 8/行 12）；
// 网格容器 padding 6x6 = AppMenu 面板形态统一 pad 6（废面板 24 覆盖），宽度随网格内容自适应（5×30+4×6+12=186）。
// 当前项选中 = 亮-选中背景；hover 叠暗-悬停，选中项 hover 不再变色（速查表 §3）。
// 两处复用：树菜单「更改图标」面板（FolderIconPanel，查当前值 + 执行命令）/
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
/* 5 列网格（item 30×30；gap 6 行列统一，2026-09-01 用户定） */
.icon-grid {
  display: grid;
  grid-template-columns: repeat(5, 30px);
  gap: var(--n-space-6); /* 行列统一 6（2026-09-01 用户定，废列 8/行 12） */
}
.cell {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  padding: 6px;
  border-radius: var(--n-radius-6);
  color: var(--n-l2);
}
/* hover 叠 暗-悬停；选中 = 亮-选中背景（稿 §3 首格样例），选中项 hover 不再变色 */
.cell:hover:not(.cur) {
  background: linear-gradient(var(--n-l-a3), var(--n-l-a3));
}
.cell.cur {
  background: var(--n-l-a1);
}
</style>
