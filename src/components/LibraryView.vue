<script setup lang="ts">
// 资源库视图（PRD 04-1）：快速访问区块 + 文件树区块（03_UI 布局/02_左侧边栏 04 树形容器）。
import SectionHeader from "./SectionHeader.vue";
import TreeItem from "./TreeItem.vue";
import FolderTree from "./FolderTree.vue";
import { useContentStore } from "../stores/content";

const content = useContentStore();

const quickItems = [
  { key: "all", icon: "library-all", label: "全部" },
  { key: "uncategorized", icon: "uncategorized", label: "未分类" },
  { key: "recent", icon: "recent", label: "最近使用" },
  { key: "trash", icon: "trash", label: "回收站" },
];
</script>

<template>
  <div class="library-view">
    <section class="block quick">
      <SectionHeader title="快速访问" pad="0 8px 0 14px" />
      <div class="items">
        <TreeItem
          v-for="q in quickItems"
          :key="q.key"
          :icon="q.icon"
          :label="q.label"
          :count="1235"
          :selected="content.sidebarSelected === `quick:${q.key}`"
          @select="content.sidebarSelected = `quick:${q.key}`"
        />
      </div>
    </section>
    <section class="block tree">
      <SectionHeader title="文件夹" :count="1231" addable pad="0 8px 0 14px" />
      <div class="items">
        <FolderTree :nodes="content.folders" />
      </div>
    </section>
  </div>
</template>

<style scoped>
.library-view {
  display: flex;
  flex-direction: column;
}
/* 快速访问区块：内边距 8 0 / 间距 6 */
.block.quick {
  display: flex;
  flex-direction: column;
  gap: var(--space-6);
  padding: var(--space-8) 0;
}
/* 文件树区块：内边距 8 0 0 / 间距 6，填满剩余 */
.block.tree {
  display: flex;
  flex-direction: column;
  gap: var(--space-6);
  padding: var(--space-8) 0 0;
  flex: 1;
}
/* item 组：内边距 0 8 0 4 / 间距 2（4 + 展开容器 8 + gap 2 = 14，与标题栏按钮左缘对齐） */
.items {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  padding: 0 var(--space-8) 0 4px;
}
</style>
