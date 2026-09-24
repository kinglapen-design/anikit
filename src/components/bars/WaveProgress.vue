<script setup lang="ts">
// 音频波形进度条（2026-09-25 竖条化换代，用户定：全 app 波形观感统一为文件卡片-音频稿竖条
// 新样式——卡片 WaveContent / 详情 pinned 均已同源，播放条是最后一处旧 5.1 发丝渲染）：
// 渲染 = WaveContent 直接复用（竖线 4px 圆头 99 gap 4 通用/五级，played 同构层品牌色 clip 进度），
// 播放条语境差两笔 = inset 12→0 满幅、played 层祖先态显隐→恒显（下方 :deep 覆盖）；
// 数据 = get_audio_peaks 128 根 cardPeaksFor 懒加载（SectionPinned 同范式），peaks 失败 = 空波形；
// 播放头 1px 品牌竖线保留。旧 hires 重采样图 + CSS mask 方案退役（get_waveform 全链删除）。
// 交互 = pointerdown 跳值 + window pointermove 拖动（BarSlider 同款链路，drag-start/end 透传 scrubbing 挂钩）。
import { inject, ref, watch } from "vue";
import WaveContent from "../cards/contents/WaveContent.vue";
import { cardPeaksFor } from "../../lib/cardPeaks";
import { PREVIEW_SESSION_KEY } from "../../lib/previewSessionKey";

const props = withDefaults(defineProps<{ peaks?: number[] }>(), { peaks: undefined });
const model = defineModel<number>({ default: 0 }); // 0~1
const emit = defineEmits<{ "drag-start": []; "drag-end": [] }>();

const preview = inject(PREVIEW_SESSION_KEY)!;
const rootEl = ref<HTMLElement>();

// peaks 懒加载（SectionPinned 同范式：await 期间切素材防错配）；props.peaks 提供 = 演示注入优先
const fetchedPeaks = ref<number[]>([]);
watch(
  () => preview.current,
  async (cur) => {
    if (!cur) {
      fetchedPeaks.value = [];
      return;
    }
    const id = cur.assetId;
    const p = await cardPeaksFor(cur);
    if (preview.current?.assetId !== id) return;
    fetchedPeaks.value = p;
  },
  { immediate: true },
);

function setFromX(clientX: number): void {
  const el = rootEl.value;
  if (!el) return;
  const r = el.getBoundingClientRect();
  model.value = Math.min(1, Math.max(0, (clientX - r.left) / r.width));
}

function onDown(e: PointerEvent): void {
  setFromX(e.clientX);
  emit("drag-start");
  const move = (ev: PointerEvent) => setFromX(ev.clientX);
  const up = (): void => {
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", up);
    emit("drag-end");
  };
  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", up);
}
</script>

<template>
  <div ref="rootEl" class="wp-root" style="touch-action: none" @pointerdown="onDown">
    <WaveContent :peaks="props.peaks ?? fetchedPeaks" :progress="model" />
    <span class="head" :style="{ left: `${model * 100}%` }" />
  </div>
</template>

<style scoped>
/* 根：满父宽、满播放条内容高（42 - padding 12×2 = 18，父 .fbp-progress align-self stretch 供给，
   §59），圆角 4 裁波形（沿用旧口径） */
.wp-root {
  position: relative;
  width: 100%;
  height: 100%;
  border-radius: var(--n-radius-4);
  overflow: hidden;
  cursor: pointer;
}
/* WaveContent 播放条语境差：卡片 inset 12 → 满幅 0；played 扫过层卡片靠祖先 hover/playing 显隐
   （.fc-root 语境）→ 播放条恒显（进度本色语义，无悬停概念） */
.wp-root :deep(.fc-wave) {
  inset: 0;
}
.wp-root :deep(.fc-wave-row.played) {
  opacity: 1;
}
.head {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 1px;
  background: var(--n-brand);
  z-index: 2;
  pointer-events: none;
}
</style>
