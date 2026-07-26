<script setup lang="ts">
// 播放条分发壳（5.2）：sequence/anim 共用帧播放条（PlayBarFrames），audio/video 各自媒体播放条。
// image/other 不出播放条（ContentArea 按 preview.hasPlayBar 决定是否渲染本组件）。
import PlayBarFrames from "./PlayBarFrames.vue";
import PlayBarAudio from "./PlayBarAudio.vue";
import PlayBarVideo from "./PlayBarVideo.vue";
import { usePreviewStore } from "../stores/preview";

const preview = usePreviewStore();
</script>

<template>
  <PlayBarFrames v-if="preview.kind === 'sequence' || preview.kind === 'anim'" />
  <PlayBarAudio v-else-if="preview.kind === 'audio'" />
  <PlayBarVideo v-else-if="preview.kind === 'video'" />
</template>
