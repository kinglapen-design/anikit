<script setup lang="ts">
// 中间内容展示区（C2）：标题栏 + 文件夹区 + 素材网格（自适应 + 虚拟滚动）+ 搜索框。
import { computed } from "vue";
import ContentTitleBar from "./ContentTitleBar.vue";
import FolderStrip from "./FolderStrip.vue";
import SectionHeader from "./SectionHeader.vue";
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
    <div class="content-body">
      <FolderStrip />
      <section class="asset-section">
        <SectionHeader title="内容" :count="items.length" />
        <AssetGrid :items="items" />
      </section>
    </div>
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
  border: 1px solid var(--c4-7);
  border-radius: var(--radius-6);
  padding: var(--space-4);
}
.content-body {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  /* 文件夹卡片列表与素材卡片列表间距 16（token 间距） */
  gap: var(--space-16);
}
.asset-section {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}
/* 组件布局 05.3：搜索框内容容器内边距 4 8 8 8 */
.search-wrap {
  flex: none;
  padding: var(--space-4) var(--space-8) var(--space-8) var(--space-8);
}
</style>
