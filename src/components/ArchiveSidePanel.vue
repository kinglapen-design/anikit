<script setup lang="ts">
// 档案二级侧边栏（2026-08-15 任务 A，PRD 02-03；本轮空壳，数据六阶段接入）：
// 位置 = 左侧边栏与中间面板（dockview 宿主）之间，仅档案视口挂载（App.vue v-if 判定）；
// 宽 250（同右栏）/ C3 底 / 右边框 1px 暗-悬停分隔线；
// 收展宽度 = sidebarShowWidths.archive 单一写点（useSidebarAnim 逐帧驱动，同左右栏机制无闪帧；
// 本组件挂载时按 ui.archivePanelOpen 快照初始宽，不产生开合动画）。
// 结构预留：标题区（档案名，真实数据）+ 内容滚动区（ScrollThumb 口径，空壳无占位文案）。
import { onMounted, ref } from "vue";
import ScrollThumb from "./ScrollThumb.vue";
import { ARCHIVE_PANEL_W, sidebarShowWidths } from "../lib/useSidebarAnim";
import { useUiStore } from "../stores/ui";
import type { FolderNode } from "../stores/content";

defineProps<{ node: FolderNode }>();
const ui = useUiStore();
const bodyRef = ref<HTMLElement>();

// 挂载快照（进档案视口不播动画：开 = 全宽 / 关 = 0，toggle 才走逐帧）
onMounted(() => {
  sidebarShowWidths.archive = ui.archivePanelOpen ? ARCHIVE_PANEL_W : 0;
});
</script>

<template>
  <aside class="archive-side" :style="{ width: `${sidebarShowWidths.archive}px` }">
    <div class="as-inner">
      <!-- 标题区：档案名（真实数据；操作钮等随数据阶段接入） -->
      <header class="as-title">{{ node.name }}</header>
      <!-- 内容滚动区（空壳预留，ScrollThumb 口径：滚动容器 .kit-scroll + relative 包裹层） -->
      <div class="scroll-wrap">
        <div ref="bodyRef" class="as-body kit-scroll" />
        <ScrollThumb :body="bodyRef" />
      </div>
    </div>
  </aside>
</template>

<style scoped>
/* 档案二级侧边栏：250 定宽内层 + align-end（收窄时内容锚右缘、左缘裁剪，同左右栏 v6 口径；
   本文件禁挂 width 的 CSS transition——宽度由 useSidebarAnim 逐帧写） */
.archive-side {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  flex: none;
  min-height: 0;
  overflow: hidden;
  background: var(--c3);
}
.as-inner {
  display: flex;
  flex-direction: column;
  width: 250px;
  min-height: 0;
  flex: 1;
  border-right: 1px solid var(--line-hover);
}
/* 标题区：46 高 / padding 12 16 / 下边框 暗-悬停（主窗口既有面板口径） */
.as-title {
  display: flex;
  align-items: center;
  height: 46px;
  box-sizing: border-box;
  padding: var(--space-12) var(--space-16);
  flex: none;
  border-bottom: 1px solid var(--line-hover);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-highlight);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.scroll-wrap {
  position: relative;
  flex: 1;
  min-height: 0;
}
.as-body {
  height: 100%;
  overflow-y: auto;
  overflow-x: hidden;
}
</style>
