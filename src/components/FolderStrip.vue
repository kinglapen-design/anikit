<script setup lang="ts">
// 文件夹卡片区（03_UI 布局/03_中间面板 03）：换行流式布局，随网格统一滚动（在 AssetGrid canvas 内）。
// 卡片同宽：grid auto-fill 均分列宽（含空轨道），拖动窗口 / 拆分条时整体缩放、阈值换行。
import SectionHeader from "./SectionHeader.vue";
import FolderCard from "./FolderCard.vue";
import { useContentStore } from "../stores/content";

const content = useContentStore();
</script>

<template>
  <section class="folder-block">
    <SectionHeader title="文件夹" :count="content.stripFolders.length" />
    <div class="flow">
      <FolderCard v-for="f in content.stripFolders" :key="f.id" :folder="f" class="cell" />
    </div>
  </section>
</template>

<style scoped>
/* 文件夹卡片区块：间距 6；padding 上 8 / 右 8（右缘与素材卡片对齐），左 0（随素材区块容器左缘） */
.folder-block {
  flex: none;
  display: flex;
  flex-direction: column;
  gap: var(--space-6);
  padding: var(--space-8) var(--space-8) 0 0;
}
/* 卡片列表：基础网格，固定 189 列宽（用户 2026-07-20 定：文件夹卡片固定大小，不自适应），
   间距 6，满了换行 */
.flow {
  display: grid;
  grid-template-columns: repeat(auto-fill, 189px);
  gap: var(--space-6);
}
.cell {
  min-width: 0;
}
</style>
