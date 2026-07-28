<script setup lang="ts">
// 文件夹卡片区（03_UI 布局/03_中间面板 03）：换行流式布局，随网格统一滚动（在 AssetGrid canvas 内）。
// 卡片同宽：grid auto-fill 均分列宽（含空轨道）；列宽跟随滑块（与素材行高同比例缩放，
// 滚轮 / 滑块可同时缩放文件夹卡片，窗口拉伸后仍有缩小手段）。
import { computed, ref } from "vue";
import SectionHeader from "./SectionHeader.vue";
import FolderCard from "./FolderCard.vue";
import { dragView } from "../lib/dragOperation";
import { useContentStore } from "../stores/content";
import { useSelectionStore } from "../stores/selection";
import { useUiStore } from "../stores/ui";

const content = useContentStore();
const selection = useSelectionStore();
const ui = useUiStore();
const folderOpen = ref(true);

// 文件夹卡片选中（2026-07-28 复验反馈 5：迁 selection store 文件夹选择域——单击 = 单选 /
// Ctrl+单击 = 加选切换 / Shift+单击 = 按条带顺序锚点范围选；与素材 / 收件箱条目互斥，
// 互斥清理由 clickFolder 内部统一做）；有效拖拽结束后的紧随 click 不触发（C4 引擎置位）
function onSelect(id: string, e: MouseEvent): void {
  if (dragView.suppressClick) return;
  selection.clickFolder(
    id,
    { ctrl: e.ctrlKey || e.metaKey, shift: e.shiftKey },
    content.stripFolders.map((f) => f.id),
  );
}

// 双击卡片（非名称区域）= 进入该文件夹（2026-07-29 复验三轮）；
// sidebarSelected watch 会清选择，无需手动；名称双击在卡片内 stop 不会到此
function onEnter(id: string): void {
  content.sidebarSelected = `folder:${id}`;
}

// 列宽 = 181 × 滑块行高 / 220（基准行高 220 时列宽 181——gap 16 + 左右间距后
// 189 一排只放得下 3 列，181 一排正好 4 列；181 也是文件夹 SVG 原生宽度，1:1 显示）
const colWidth = computed(() => Math.max(80, Math.round((181 * ui.targetRowHeight) / 220)));
</script>

<template>
  <section class="folder-block">
    <SectionHeader title="文件夹" :count="content.stripFolders.length" v-model:open="folderOpen" />
    <div v-if="folderOpen" class="flow" :style="{ gridTemplateColumns: `repeat(auto-fill, minmax(${colWidth}px, 1fr))` }">
      <FolderCard
        v-for="f in content.stripFolders"
        :key="f.id"
        :folder="f"
        :thumb-path="content.folderThumbMap.get(f.id) ?? null"
        class="cell"
        :selected="selection.isFolderSelected(f.id)"
        :dimmed="dragView.folderIds?.has(f.id) ?? false"
        @select="onSelect(f.id, $event)"
        @dblclick="onEnter(f.id)"
      />
    </div>
  </section>
</template>

<style scoped>
/* 文件夹卡片区块：间距 8；padding 上 8 / 右 20 / 下 8（左 0 由素材区块容器 20 承担；
   右 20 与网格预留对齐——文件夹卡片右缘距中栏外边框同为 24，2026-07-23 用户定） */
.folder-block {
  flex: none;
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
  padding: var(--space-8) var(--space-20) var(--space-8) 0;
}
/* 卡片列表：grid 均分同宽（auto-fill 空轨道同宽，末行不被拉宽）——
   卡片等比例跟随拖拽缩放（SVG 天然等比，无布局计算），满了换行；
   列间距 16 / 行间距 8（2026-07-23 用户定，与素材卡片同步）；
   查看方式的自适应 / 瀑布 / 网格只影响素材卡片，与文件夹卡片无关 */
.flow {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(189px, 1fr));
  column-gap: var(--space-16);
  row-gap: var(--space-16);
}
.cell {
  min-width: 0;
}
</style>
