<script setup lang="ts">
// 音频波形大图（原 PreviewPanel waveUrl 分支，2026-09-21 R5 抽入 previewSpecs）：
// 优先后端按画面区实际像素重采样的高清图（useHiresWaveform 内部已按 kind==='audio' 门控，
// stage 测量元素经 PREVIEW_STAGE_KEY 由宿主注入），未就绪 / 生成失败回退 info 波形缩略图（5.1）
// 拉伸旧行为；waveUrl 未就绪渲染空 = 原模板链「靠 waveUrl 非 kind」fallthrough 等价。
import { computed, inject } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { useHiresWaveform } from "../lib/useHiresWaveform";
import { PREVIEW_STAGE_KEY } from "../lib/previewSpecs";
import { usePreviewStore } from "../stores/preview";

const preview = usePreviewStore();
const stageRef = inject(PREVIEW_STAGE_KEY)!; // 宿主 PreviewPanel 必提供
const hiresWavePath = useHiresWaveform(stageRef);
// kind==='audio' 由注册表命中结构保证（原宿主 computed 里的 kind 门控随之省略）
const waveUrl = computed(() => {
  const src = hiresWavePath.value ?? preview.current?.thumbPath;
  return src ? convertFileSrc(src) : null;
});
</script>

<template>
  <div v-if="waveUrl" class="wave-large" :style="{ '--wave-url': `url(&quot;${waveUrl}&quot;)` }" />
</template>

<style scoped>
/* 波形大图（重采样图按画面区等比渲染；CSS mask 方案——图柱 alpha 恒 100% 供 mask 取形，
   颜色 = 静态波形实色 --n-l3（原 --wave-pending 就近；2026-08-10 实色化，font-icon-3 观感作废），
   mask-size contain 居中即原 object-fit 行为） */
.wave-large {
  position: absolute;
  inset: 0;
  background: var(--n-l3);
  -webkit-mask-image: var(--wave-url);
  mask-image: var(--wave-url);
  -webkit-mask-size: contain;
  mask-size: contain;
  -webkit-mask-position: center;
  mask-position: center;
  -webkit-mask-repeat: no-repeat;
  mask-repeat: no-repeat;
}
</style>
