<script setup lang="ts">
// 文件夹卡片区（03_UI 布局/03_中间面板 03）：换行流式布局，随网格统一滚动（在 AssetGrid canvas 内）。
// 卡片同宽：grid auto-fill 均分列宽（含空轨道）；列宽跟随滑块（与素材行高同比例缩放，
// 滚轮 / 滑块可同时缩放文件夹卡片，窗口拉伸后仍有缩小手段）。
import { computed, ref } from "vue";
import SectionHeader from "./SectionHeader.vue";
import FolderCard from "./FolderCard.vue";
import { useContentStore } from "../stores/content";
import { useUiStore } from "../stores/ui";

const content = useContentStore();
const ui = useUiStore();
const folderOpen = ref(true);

// 列宽 = 189 × 滑块行高 / 220（基准行高 220 时列宽 189）
const colWidth = computed(() => Math.max(80, Math.round((189 * ui.targetRowHeight) / 220)));
</script>

<template>
  <section class="folder-block">
    <SectionHeader title="文件夹" :count="content.stripFolders.length" v-model:open="folderOpen" />
    <div v-if="folderOpen" class="flow" :style="{ gridTemplateColumns: `repeat(auto-fill, minmax(${colWidth}px, 1fr))` }">
      <FolderCard
        v-for="f in content.stripFolders"
        :key="f.id"
        :folder="f"
        class="cell"
        :selected="content.selectedFolderId === f.id"
        @select="
          content.selectedFolderId = f.id;
          content.selectedAssetKey = '';
        "
      />
    </div>
  </section>
</template>

<style scoped>
/* 文件夹卡片区块：间距 8；padding 上 8 / 右 16 / 下 8（左 0 由素材区块容器 16 承担；
   右 16 与网格预留对齐——文件夹卡片右缘距中栏外边框同为 24，2026-07-23 用户定） */
.folder-block {
  flex: none;
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
  padding: var(--space-8) var(--space-16) var(--space-8) 0;
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
