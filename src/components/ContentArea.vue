<script setup lang="ts">
// 中间内容展示区（C2）：标题栏 + 卡片列表（统一滚动：文件夹区块 + 素材网格）+ 搜索框。
import { computed } from "vue";
import ContentTitleBar from "./ContentTitleBar.vue";
import AssetGrid from "./AssetGrid.vue";
import SearchBar from "./SearchBar.vue";
import { useContentStore } from "../stores/content";
import { useUiStore } from "../stores/ui";

const content = useContentStore();
const ui = useUiStore();

// PRD 04-1：开启"显示子文件夹内容"= 递归显示全部子项（"全部"视图即此模式）；
// 关闭则只显示当前目录（假数据取前 1231 条演示）。
const items = computed(() => (ui.showSubfolders ? content.assets : content.assets.slice(0, 1231)));
</script>

<template>
  <main class="content-area">
    <ContentTitleBar />
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
  background: var(--c2);
  /* 三栏面板无圆角无边框（主窗口结构调整） */
  padding: var(--space-4);
}
/* 素材卡片区块：内边距 8 0 0 8（右 0——滚动条贴区块右缘，中栏 4px 内边距作呼吸） */
.asset-section {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  padding: var(--space-8) 0 0 var(--space-8);
}
/* 搜索框内容容器：内边距 4 8 8 8 */
.search-wrap {
  flex: none;
  padding: var(--space-4) var(--space-8) var(--space-8) var(--space-8);
}
</style>
