<script setup lang="ts">
// 附加件 · 视频底部播放条（video）——R5 扩展点注册表化（2026-09-21）自 FileCard 抽出，
// 注册见 src/lib/cardSpecs/video.ts；样式块原样随迁、值未动（hover 才现，150ms）。
defineOptions({ inheritAttrs: false });
withDefaults(defineProps<{ progress?: number }>(), { progress: 0 });
</script>

<template>
  <div class="fc-vbar">
    <span class="fc-vbar-fill" :style="{ width: `${progress * 100}%` }" />
  </div>
</template>

<style scoped>
/* 视频播放条：4px 高、左右 12、贴底 4；轨道 亮色/半透明-2、填充 亮色/一级；hover 才现 */
.fc-vbar {
  position: absolute;
  left: var(--n-space-12);
  right: var(--n-space-12);
  bottom: var(--n-space-4);
  z-index: 5;
  height: 4px;
  border-radius: 99px; /* 稿值 99 */
  background: var(--n-l-a2);
  opacity: 0;
  transition: opacity var(--n-dur-150) var(--n-ease-in-out);
}
.fc-root:hover .fc-vbar,
.fc-root.force-hover .fc-vbar {
  opacity: 1;
}
.fc-vbar-fill {
  display: block;
  height: 100%;
  border-radius: 99px; /* 稿值 99 */
  background: var(--n-l1);
}
</style>
