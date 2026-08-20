<script setup lang="ts">
// 独立预览窗口（4.3 新窗口打开）：标题栏（文件名 + 置顶 + 窗口控制）+ 预览画面 + 播放条/帧条/笔记浮层。
// 经 URL ?preview={assetKey} 进入；数据走同一 Rust AppState，本窗口独立 pinia，拉全量后 open。
import { onMounted } from "vue";
import PreviewTitleBar from "./PreviewTitleBar.vue";
import PreviewPanel from "./PreviewPanel.vue";
import PreviewPlayBar from "./PreviewPlayBar.vue";
import PreviewFrameStrip from "./PreviewFrameStrip.vue";
import PreviewNotes from "./PreviewNotes.vue";
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
    <footer v-if="preview.hasPlayBar" class="pw-footer">
      <PreviewPlayBar />
      <PreviewFrameStrip />
      <PreviewNotes />
    </footer>
  </div>
</template>

<style scoped>
/* 独立预览窗口：占满窗口（App.vue 的 .window 是横向 flex——必须 flex:1，否则宽度由内容撑开、右侧留白） */
.preview-window {
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
/* 播放条 + 帧条：通栏贴底（13_播放条稿 2026-08-20 用户定）——负 margin 拉出窗口 4px 内边距；
   relative 供笔记浮层（bottom:100%）锚定 */
.pw-footer {
  position: relative;
  flex: none;
  margin: 0 calc(-1 * var(--space-4)) calc(-1 * var(--space-4));
}
/* 全屏：去窗口内边距，播放条/帧条脱离文档流浮在画面底部（画面按整个窗口铺满，2026-07-26 用户定） */
.preview-window.fullscreen {
  padding: 0;
}
.preview-window.fullscreen .pw-footer {
  position: absolute;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 1;
  margin: 0; /* 全屏窗口 padding 已归零，负 margin 拉通取消（防拉出窗口外） */
}
</style>
