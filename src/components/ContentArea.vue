<script setup lang="ts">
// 中间内容展示区（C1）：标题栏 + 卡片列表（统一滚动：文件夹区块 + 素材网格）+ 搜索框。
// 预览模式（4.3）：preview.assetKey 非空时整栏切换——预览标题栏 / 预览画面 / 序列帧播放条+帧条
// （播放条占搜索栏位置，2026-07-25 定案）；全屏时 preview-root fixed 铺满窗口、隐藏标题栏。
import { computed } from "vue";
import ContentTitleBar from "./ContentTitleBar.vue";
import AssetGrid from "./AssetGrid.vue";
import SearchBar from "./SearchBar.vue";
import ImportProgressBar from "./ImportProgressBar.vue";
import PreviewTitleBar from "./PreviewTitleBar.vue";
import PreviewPanel from "./PreviewPanel.vue";
import PreviewPlayBar from "./PreviewPlayBar.vue";
import PreviewFrameStrip from "./PreviewFrameStrip.vue";
import { useContentStore } from "../stores/content";
import { usePreviewStore } from "../stores/preview";

const content = useContentStore();
const preview = usePreviewStore();

// 当前视口资源（"全部"= 全部 Placement；文件夹 = 该文件夹直接 Placement）。
// PRD 04-1"显示子文件夹内容"开关：递归展开子文件夹内容阶段 4 接（当前两种状态同列表）。
const items = computed(() => content.visiblePlacements);
</script>

<template>
  <main class="content-area">
    <template v-if="preview.assetKey">
      <div class="preview-root" :class="{ fullscreen: preview.fullscreen }">
        <PreviewTitleBar v-if="!preview.fullscreen" />
        <section class="preview-section">
          <PreviewPanel />
        </section>
        <footer v-if="preview.kind === 'sequence'" class="preview-footer">
          <PreviewPlayBar />
          <PreviewFrameStrip />
        </footer>
      </div>
    </template>
    <template v-else>
      <ContentTitleBar />
      <ImportProgressBar />
      <section class="asset-section">
        <AssetGrid :items="items" />
      </section>
      <footer class="search-wrap">
        <SearchBar />
      </footer>
    </template>
  </main>
</template>

<style scoped>
.content-area {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  background: var(--c1);
  /* 三栏面板无圆角无边框（主窗口结构调整）；padding 左右 4——标题栏 / 搜索框
     （自身 padding 12）左右缘均为 16；滚动条可见条右缘距中栏外边框 4（2026-07-23 用户定） */
  padding: var(--space-4);
}
/* 素材卡片区块：内边距 0 0 0 20（左 20 + 中栏 4 = 卡片距中栏外边框左 24；
   右 0——滚动条可见条右缘距中栏外边框 4；卡片右缘 24 = 中栏右 4 + 里层网格预留 20） */
.asset-section {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  padding: 0 0 0 var(--space-20);
}
/* 搜索框内容容器：内边距 4 12 8 12（中栏外层 4 + 12 = 左右缘均 16，2026-07-23 用户定） */
.search-wrap {
  flex: none;
  padding: var(--space-4) var(--space-12) var(--space-12) var(--space-12);
}
/* 预览模式根：占满中栏；全屏时 fixed 铺满窗口（2026-07-25 定案：显示器全屏） */
.preview-root {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}
.preview-root.fullscreen {
  position: fixed;
  inset: 0;
  z-index: 1500;
  background: var(--c1);
}
.preview-section {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}
/* 播放条 + 帧条容器：与搜索框容器同位置同内边距 */
.preview-footer {
  flex: none;
  padding: var(--space-4) var(--space-12) var(--space-12) var(--space-12);
}
</style>
