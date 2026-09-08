<script setup lang="ts">
// 独立预览窗口（4.3 新窗口打开）：标题栏（文件名 + 置顶 + 窗口控制）+ 预览画面 + 悬浮播放条/帧条/笔记弹层
//（2026-09-09 预览悬浮栏接入轮换 PreviewFloatBar，旧通栏 footer 三件套作废）。
// 经 URL ?preview={assetKey} 进入；数据走同一 Rust AppState，本窗口独立 pinia，拉全量后 open。
import { onMounted } from "vue";
import PreviewTitleBar from "./PreviewTitleBar.vue";
import PreviewPanel from "./PreviewPanel.vue";
import PreviewFloatBar from "./PreviewFloatBar.vue";
import { useContentStore } from "../stores/content";
import { usePreviewStore } from "../stores/preview";

const props = defineProps<{ assetKey: string }>();
const content = useContentStore();
const preview = usePreviewStore();

onMounted(async () => {
  await content.refresh();
  await preview.open(props.assetKey);
});
</script>

<template>
  <div class="preview-window" :class="{ fullscreen: preview.fullscreen }">
    <PreviewTitleBar v-if="!preview.fullscreen" window />
    <section class="pw-body">
      <PreviewPanel />
    </section>
    <PreviewFloatBar v-if="preview.hasPlayBar" />
  </div>
</template>

<style scoped>
/* 独立预览窗口：占满窗口（App.vue 的 .window 是横向 flex——必须 flex:1，否则宽度由内容撑开、右侧留白）；
   relative = 悬浮播放条（PreviewFloatBar anchor absolute 底部居中距底 12）定位基准，全屏两态同位 */
.preview-window {
  position: relative;
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  height: 100%;
  padding: var(--space-4);
}
.pw-body {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}
/* 全屏：去窗口内边距，画面按整个窗口铺满（2026-07-26 用户定） */
.preview-window.fullscreen {
  padding: 0;
}
</style>
