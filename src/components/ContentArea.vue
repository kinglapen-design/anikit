<script setup lang="ts">
// 中间内容展示区（C1）：标题栏 + 卡片列表（统一滚动：文件夹区块 + 素材网格）+ 搜索框。
import { computed } from "vue";
import ContentTitleBar from "./ContentTitleBar.vue";
import AssetGrid from "./AssetGrid.vue";
import SearchBar from "./SearchBar.vue";
import ImportProgressBar from "./ImportProgressBar.vue";
import { useContentStore } from "../stores/content";

const content = useContentStore();

// 当前视口资源（"全部"= 全部 Placement；文件夹 = 该文件夹直接 Placement）。
// PRD 04-1"显示子文件夹内容"开关：递归展开子文件夹内容阶段 4 接（当前两种状态同列表）。
const items = computed(() => content.visiblePlacements);
</script>

<template>
  <main class="content-area">
    <ContentTitleBar />
    <ImportProgressBar />
    <section class="asset-section">
      <AssetGrid :items="items" />
    </section>
    <footer class="search-wrap">
      <SearchBar />
    </footer>
  </main>
</template>

<style scoped>
.content-area {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  background: var(--c1);
  /* 三栏面板无圆角无边框（主窗口结构调整） */
  padding: var(--space-8);
}
/* 素材卡片区块：内边距 0 16（左右 16 统一承担文件夹/素材两区块的左右 16，2026-07-23 用户定；
   上 8 由"内容"副标题 pad 承担，与文件夹区块底 8、总容器 gap 16 构成间距 32） */
.asset-section {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  padding: 0 var(--space-16);
}
/* 搜索框内容容器：内边距 4 8 8 8 */
.search-wrap {
  flex: none;
  padding: var(--space-4) var(--space-8) var(--space-8) var(--space-8);
}
</style>
