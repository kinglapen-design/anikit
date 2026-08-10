<script setup lang="ts">
// 中间内容展示区（C1）：1.2 起 = dockview panel 组件，每个标签页一个实例，按 params.viewKey
// 渲染各自视口（标题栏 + 卡片列表（统一滚动：文件夹区块 + 素材网格）+ 搜索框）。
// 预览模式（4.3/5.2）：preview.assetKey 非空且本 panel 激活时整栏切换——预览是全局单例状态，
// 只在激活标签页内发生，非激活 panel 保持原视口；全屏时 preview-root fixed 铺满窗口、隐藏标题栏。
// 通知条（ToastStack）已上移到 DockHost 单例（多 panel 下每 panel 一份会重复弹出）。
import { computed, onBeforeUnmount, ref } from "vue";
import type { DockviewPanelApi, IDockviewPanelProps } from "dockview-vue";
import ContentTitleBar from "./ContentTitleBar.vue";
import AssetGrid from "./AssetGrid.vue";
import SearchBar from "./SearchBar.vue";
import ImportProgressBar from "./ImportProgressBar.vue";
import PreviewTitleBar from "./PreviewTitleBar.vue";
import PreviewPanel from "./PreviewPanel.vue";
import PreviewPlayBar from "./PreviewPlayBar.vue";
import PreviewFrameStrip from "./PreviewFrameStrip.vue";
import PreviewNotes from "./PreviewNotes.vue";
import { useContentStore } from "../stores/content";
import { usePreviewStore } from "../stores/preview";

// props 运行时声明（rolldown 环境 SFC 编译器无法解析 node_modules 类型，defineProps 泛型须字面量）。
// dockview-vue 传参约定（v7 源码 VueRenderer.init）：framework props 整组包在一个 params prop 里——
// 组件实际收到 params = { params: 用户参数, api, containerApi, tabLocation }，updateParameters 后整体换新（响应式）。
const raw = defineProps<{ params: Record<string, unknown> }>();
const frame = computed(() => raw.params as unknown as IDockviewPanelProps);
const panelApi = frame.value.api as DockviewPanelApi; // api 引用 init 时确定、不变
const content = useContentStore();
const preview = usePreviewStore();

// 本标签页视口（frame.params = addPanel/updateParameters 的用户参数）
const viewKey = computed(() => (frame.value.params?.viewKey as string) ?? "quick:all");

// 本 panel 激活态：预览整栏切换只对激活标签页生效
const active = ref(panelApi.isActive);
const disposeActive = panelApi.onDidActiveChange(() => {
  active.value = panelApi.isActive;
});
onBeforeUnmount(() => disposeActive.dispose());

// 当前视口资源（"全部"= 全部 Placement；文件夹 = 该文件夹直接 Placement，
// 递归显示开关 ui.showSubfolders 打开时含全部后代文件夹，2026-08-10 接入）。
const items = computed(() => content.placementsFor(viewKey.value));
</script>

<template>
  <main class="content-area">
    <template v-if="preview.assetKey && active">
      <div class="preview-root" :class="{ fullscreen: preview.fullscreen }">
        <PreviewTitleBar v-if="!preview.fullscreen" />
        <section class="preview-section">
          <PreviewPanel />
        </section>
        <footer v-if="preview.hasPlayBar" class="preview-footer">
          <PreviewPlayBar />
          <PreviewFrameStrip />
          <PreviewNotes />
        </footer>
      </div>
    </template>
    <template v-else>
      <ContentTitleBar />
      <ImportProgressBar />
      <section class="asset-section">
        <AssetGrid :items="items" :view-key="viewKey" :panel-active="active" />
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
  height: 100%; /* 1.2：dockview panel 内容容器由 dock 管理尺寸，铺满即可 */
  min-width: 0;
  background: var(--c1);
  /* 三栏面板无圆角无边框、外层无 padding（figma-css 02_布局/42_中间面板，2026-08-10）：
     工具栏 / 搜索框自带 padding 12 16、列表容器 padding 16 16 0，各自承担 */
  box-sizing: border-box;
}
/* 卡片列表（figma 42 列表 padding 16px 16px 0px）：上 16 / 左 16 / 下 0；
   右 0——滚动条紧贴右边框（2026-08-10 用户定：4px 间距让通栏分隔线视觉截断；
   卡片右缘距边框 16 由布局 PAD.right 16 承担） */
.asset-section {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  padding: var(--space-16) 0 0 var(--space-16);
}
/* 搜索框（1.2 新稿）：通栏行（C3 + 上边框横贯中栏）——外层无 padding，自然通栏 */
.search-wrap {
  flex: none;
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
/* 全屏时播放条/帧条脱离文档流浮在画面底部（2026-07-26 用户：画面按整个窗口铺满，挡住就挡住） */
.preview-root.fullscreen .preview-footer {
  position: absolute;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 1;
}
.preview-section {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}
/* 播放条 + 帧条容器：relative 供笔记浮层（bottom:100%）锚定；内边距 8 16 16——
   内容距中栏左右缘 16、上 8、下 16（2026-08-10 随 42 稿外层 padding 归零后保持原视觉） */
.preview-footer {
  position: relative;
  flex: none;
  padding: var(--space-8) var(--space-16) var(--space-16);
}
</style>
