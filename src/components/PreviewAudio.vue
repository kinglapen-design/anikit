<script setup lang="ts">
// 音乐预览（稿：布局/主窗口/中间面板/中间面板-内容容器-音乐预览，2026-09-25 落地，
// 换代 R5 的 PreviewAudioWave 波形大图——稿 prose「只在播放条上放波形图即可，预览界面无需波形图」，
// 播放条侧 = FloatBarPlay #progress 的 WaveProgress，速查 §36）：
// 居中立姿 = 唱片-icon 200（vinyl 装饰图标，渐变底 + 转盘 + 指针、opacity 0.6 与小阴影均烘焙在 svg）
// + 文件名（标题档 18/26 Bold 二级，含扩展名）；块 = column 居中 gap 24 + padding-bottom 24（稿「图标+提示」）。
import { computed } from "vue";
import AppIcon from "./AppIcon.vue";
import { usePreviewStore } from "../stores/preview";

const preview = usePreviewStore();
// kind==='audio' 由注册表命中结构保证；文件名同 PreviewTitleBar .index-name 口径（nowrap 不截断）
const fileName = computed(() => preview.current?.name ?? "");
</script>

<template>
  <div class="audio-preview">
    <div class="ap-body">
      <AppIcon name="vinyl" :size="200" />
      <span class="ap-title">{{ fileName }}</span>
    </div>
  </div>
</template>

<style scoped>
/* 根层铺满画面区内容居中（同 PreviewUnsupported / EmptyFolderState 居中立姿口径） */
.audio-preview {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}
/* 块 = column 居中 gap 24 + padding-bottom 24（稿「图标+提示」200×274） */
.ap-body {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--n-space-24);
  padding-bottom: var(--n-space-24);
}
/* 文件名 = 标题档 18/26 Bold 二级（稿值） */
.ap-title {
  font-size: var(--n-fs-title);
  line-height: var(--n-lh-title);
  font-weight: var(--n-fw-bold);
  color: var(--n-l2);
  white-space: nowrap;
}
</style>
