<script setup lang="ts">
// 独立预览窗口（4.3 新窗口打开）：标题栏（文件名 + 置顶 + 窗口控制）+ 预览画面 + 悬浮播放条/帧条/弹层
//（FloatBarHost 预览态 play 模式；不传 panelId/viewKey = 搜索分支禁用，2026-09-09 二轮宿主合并）。
// 经 URL ?preview={assetKey} 进入；数据走同一 Rust AppState，本窗口独立 pinia，拉全量后 open。
import { onMounted, provide } from "vue";
import PreviewTitleBar from "./PreviewTitleBar.vue";
import PreviewPanel from "./PreviewPanel.vue";
import FloatBarHost from "./FloatBarHost.vue";
import { useContentStore } from "../stores/content";
import { previewSession, WINDOW_PANEL_ID } from "../stores/preview";
import { PREVIEW_SESSION_KEY } from "../lib/previewSessionKey";

const props = defineProps<{ assetKey: string }>();
const content = useContentStore();
// 独立子窗口会话（属主 = WINDOW_PANEL_ID；provide 给本窗口预览族组件 inject）
const preview = previewSession(WINDOW_PANEL_ID);
provide(PREVIEW_SESSION_KEY, preview);

// 启动竞态（2026-11-02 踩坑 121 附带，用户实机截图「请先打开素材库」坐实）：子窗口 onMounted 即
// content.refresh()，启动早期 AppState 库未就绪时吃「请先打开素材库」——主窗口有库就绪等待
//（deep-link pending 同口径），子窗口补同款：仅拦库未就绪错 200ms×50 重试，其余错误原样抛
async function refreshWhenLibraryReady(): Promise<void> {
  for (let i = 0; i < 50; i++) {
    try {
      await content.refresh();
      return;
    } catch (e) {
      if (!String(e).includes("请先打开素材库") || i === 49) throw e;
      await new Promise((r) => setTimeout(r, 200));
    }
  }
}

onMounted(async () => {
  await refreshWhenLibraryReady();
  await preview.open(props.assetKey);
});
</script>

<template>
  <div class="preview-window" :class="{ fullscreen: preview.fullscreen }">
    <PreviewTitleBar v-if="!preview.fullscreen" window />
    <section class="pw-body">
      <PreviewPanel />
    </section>
    <FloatBarHost v-if="preview.hasPlayBar" preview-active />
  </div>
</template>

<style scoped>
/* 独立预览窗口：占满窗口（App.vue 的 .window 是横向 flex——必须 flex:1，否则宽度由内容撑开、右侧留白）；
   relative = 悬浮栏宿主（FloatBarHost anchor absolute 底部居中距底 12、两侧 12）定位基准，全屏两态同位 */
.preview-window {
  position: relative;
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  height: 100%;
  padding: var(--n-space-4);
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
