<script setup lang="ts">
// 音频波形进度条（音乐预览稿 prose：「只在播放条上放波形图即可」）——播放条进度条的音频变体，
// 供 FloatBarPlay #progress 插槽替换 BarSlider（2026-09-09 预览悬浮栏接入轮，自旧 PlayBarAudio 移植）：
// 两层 div 以波形图作 CSS mask（alpha 通道铺满）：底层 = 未播放 亮色/三级实色（旧 wave-pending
// 「标签档不透明版」2026-08-10 用户定的语义档映射——mask 层不受 alpha 影响须实色）；
// 已播放层 = 品牌底 clip-path 裁到进度；播放头 1px 品牌竖线。
// 波形图 = 后端按轨道实际像素重采样（useHiresWaveform 返回磁盘路径，本组件 convertFileSrc 转
// asset URL——裸路径塞 CSS url() 必加载失败，踩坑 088），未就绪/失败回退 info 波形缩略图（fallbackUrl，
// 调用方已转换）。
// 交互 = pointerdown 跳值 + window pointermove 拖动（BarSlider 同款链路，drag-start/end 透传 scrubbing 挂钩）。
import { computed, ref } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { useHiresWaveform } from "../../lib/useHiresWaveform";

const props = withDefaults(defineProps<{ fallbackUrl?: string | null }>(), { fallbackUrl: null });
const model = defineModel<number>({ default: 0 }); // 0~1
const emit = defineEmits<{ "drag-start": []; "drag-end": [] }>();

const rootEl = ref<HTMLElement>();
const hiresPath = useHiresWaveform(rootEl);
// hiresPath 是磁盘绝对路径——必须 convertFileSrc 转 asset URL（裸路径塞 CSS url() 按相对
// 地址解析必然加载失败：mask 空 = 层不可见只剩播放头，2026-09-25 实机报，踩坑 088）；
// fallbackUrl 已由调用方（barSpecs audio waveFallback）转换好，不再二转
const waveUrl = computed(() => (hiresPath.value ? convertFileSrc(hiresPath.value) : (props.fallbackUrl ?? null)));
// 两层 mask 共用的图地址（CSS 变量下发，mask 走 alpha 通道）
const waveMask = computed(() => (waveUrl.value ? { "--wave-url": `url("${waveUrl.value}")` } : {}));

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
  <div ref="rootEl" class="wp-root" :style="waveMask" style="touch-action: none" @pointerdown="onDown">
    <template v-if="waveUrl">
      <div class="wave-layer base" />
      <div class="wave-layer played" :style="{ clipPath: `inset(0 ${(1 - model) * 100}% 0 0)` }" />
    </template>
    <span class="head" :style="{ left: `${model * 100}%` }" />
  </div>
</template>

<style scoped>
/* 根：满父宽、满播放条内容高（42 - padding 12×2 = 18），圆角 4 裁波形（沿用旧口径） */
.wp-root {
  position: relative;
  width: 100%;
  height: 100%;
  border-radius: var(--n-radius-4);
  overflow: hidden;
  cursor: pointer;
}
.wave-layer {
  position: absolute;
  inset: 0;
  -webkit-mask-image: var(--wave-url);
  mask-image: var(--wave-url);
  -webkit-mask-size: 100% 100%;
  mask-size: 100% 100%;
  -webkit-mask-repeat: no-repeat;
  mask-repeat: no-repeat;
  pointer-events: none;
}
.wave-layer.base {
  background: var(--n-l3); /* 未播放 = 亮色/三级实色（旧 wave-pending 语义档映射） */
}
.wave-layer.played {
  background: var(--n-brand);
  z-index: 1;
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
