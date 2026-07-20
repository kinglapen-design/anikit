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
      <FolderCard
        v-for="f in content.stripFolders"
        :key="f.id"
        :folder="f"
        class="cell"
        :selected="content.selectedFolderId === f.id"
        @select="
          content.selectedFolderId = f.id;
          content.selectedAssetId = -1;
        "
      />
    </div>
  </section>
</template>

<style scoped>
/* 文件夹卡片区块：间距 6；padding 上 8 / 右 8 / 下 8（与素材区块顶 padding 8、
   总容器 gap 16 共同构成两区块间距 32 = 8 + 16 + 8），左 0（随素材区块容器左缘） */
.folder-block {
  flex: none;
  display: flex;
  flex-direction: column;
  gap: var(--space-6);
  padding: var(--space-8) var(--space-8) var(--space-8) 0;
}
/* 卡片列表：grid 均分同宽（auto-fill 空轨道同宽，末行不被拉宽）——
   卡片等比例跟随拖拽缩放（SVG 天然等比，无布局计算），满了换行；
   查看方式的自适应 / 瀑布 / 网格只影响素材卡片，与文件夹卡片无关 */
.flow {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(189px, 1fr));
  gap: var(--space-6);
}
.cell {
  min-width: 0;
}
</style>
