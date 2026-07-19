<script setup lang="ts">
// 文件夹卡片区（03_UI 布局/03_中间面板 03）：换行流式布局（一行满了自动换行，不横向滚动），
// 卡片基准 189 宽、弹性缩放跟随容器（拖动窗口 / 拆分条时整体缩放、阈值换行）。
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
/* 文件夹卡片区块：内边距 8 / 间距 6 */
.folder-block {
  flex: none;
  display: flex;
  flex-direction: column;
  gap: var(--space-6);
  padding: var(--space-8);
}
/* 文件夹卡片列表：间距 6，满了换行 */
.flow {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-6);
}
.cell {
  flex: 1 0 189px;
  max-width: 260px;
  min-width: 0;
}
</style>
