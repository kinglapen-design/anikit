<script setup lang="ts">
// 音频播放条（5.2）：放搜索栏位置，样式照搜索框（C3 / inset border-1 / 圆角 8 / 内边距 4 / 间距 6）。
// 播放/暂停、音量（VolumeControl）、当前时间/总时长（mm:ss 同 formatDuration 口径）、
// 波形进度条（波形缩略图铺满整条：未播放部分压 C1-40 罩层、已播放部分全亮，两层同图 clip-path 裁切，
// 播放头 1px 品牌紫竖线；点击/拖动 seek）、倍率、循环。转码中显示加载提示。音频无笔记/全屏。
import { computed, ref } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import IconButton from "./IconButton.vue";
import VolumeControl from "./VolumeControl.vue";
import { formatDuration } from "../lib/formatDuration";
import { RATES, usePreviewStore } from "../stores/preview";
import { usePreviewMediaStore } from "../stores/previewMedia";

const preview = usePreviewStore();
const media = usePreviewMediaStore();
const trackRef = ref<HTMLElement>();
let seeking = false;

// 波形缩略图（5.1 info 波形图）作进度条底图
const waveUrl = computed(() => {
  const p = preview.current?.thumbPath;
  return p ? convertFileSrc(p) : null;
});
const pct = computed(() => (media.duration > 0 ? (media.currentTime / media.duration) * 100 : 0));

function cycleRate(): void {
  const i = RATES.indexOf(media.rate);
  media.setRate(RATES[(i + 1) % RATES.length]);
}

function seek(e: PointerEvent): void {
  if (media.duration <= 0) return;
  const rect = trackRef.value!.getBoundingClientRect();
  const ratio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
  media.seek(ratio * media.duration);
}
function onSeekDown(e: PointerEvent): void {
  seeking = true;
  trackRef.value!.setPointerCapture(e.pointerId);
  seek(e);
  e.preventDefault();
}
function onSeekMove(e: PointerEvent): void {
  if (seeking) seek(e);
}
function onSeekUp(e: PointerEvent): void {
  if (!seeking) return;
  seeking = false;
  trackRef.value!.releasePointerCapture(e.pointerId);
}
</script>

<template>
  <div class="play-bar">
    <template v-if="media.transcoding">
      <span class="hint">转码中…</span>
    </template>
    <template v-else>
      <IconButton
        :name="media.playing ? 'pause' : 'play'"
        :title="media.playing ? '暂停' : '播放'"
        @click="media.togglePlay()"
      />
      <VolumeControl />
      <span class="time-text">{{ formatDuration(media.currentTime) }} / {{ formatDuration(media.duration) }}</span>
      <div
        ref="trackRef"
        class="wave-progress"
        style="touch-action: none"
        @pointerdown="onSeekDown"
        @pointermove="onSeekMove"
        @pointerup="onSeekUp"
      >
        <img v-if="waveUrl" class="wave" :src="waveUrl" draggable="false" />
        <span class="wave-dim" />
        <img
          v-if="waveUrl"
          class="wave lit"
          :src="waveUrl"
          :style="{ clipPath: `inset(0 ${100 - pct}% 0 0)` }"
          draggable="false"
        />
        <span class="head" :style="{ left: `${pct}%` }" />
      </div>
      <button class="rate-btn" v-tip="'倍率（点击切换）'" @click="cycleRate">{{ media.rate }}x</button>
      <IconButton name="loop" title="循环播放" :active="media.loop" @click="media.toggleLoop()" />
    </template>
  </div>
</template>

<style scoped>
/* 总容器：照搜索框（C3 底 / inset 1px border-1 / 圆角 8 / 内边距 4 / 间距 6） */
.play-bar {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  padding: var(--space-4);
  border-radius: var(--radius-8);
  background: var(--c3);
  box-shadow: inset 0 0 0 1px var(--border-1);
}
.play-bar :deep(.icon-btn) {
  color: var(--font-icon-2);
}
.play-bar :deep(.icon-btn.active) {
  color: var(--brand-1);
}
.time-text {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--font-icon-1);
  white-space: nowrap;
  flex: none;
}
/* 转码加载提示（说明文字档） */
.hint {
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--font-icon-3);
  padding: 0 var(--space-4);
}
/* 波形进度条：高 24，圆角 4；两层同图 + 未播放区 C1-40 罩层，播放头 1px 品牌紫 */
.wave-progress {
  position: relative;
  flex: 1;
  min-width: 0;
  height: 24px;
  border-radius: var(--radius-4);
  overflow: hidden;
  cursor: pointer;
}
.wave {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: fill;
  pointer-events: none;
}
.wave-dim {
  position: absolute;
  inset: 0;
  background: var(--c1-40);
  pointer-events: none;
}
.wave.lit {
  z-index: 1;
}
.head {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 1px;
  background: var(--brand-1);
  z-index: 2;
  pointer-events: none;
}
.rate-btn {
  padding: 0 var(--space-4);
  height: 24px;
  border-radius: var(--radius-4);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--font-icon-1);
  white-space: nowrap;
  flex: none;
  transition: background 0.12s ease;
}
.rate-btn:hover {
  background: var(--font-icon-6);
}
</style>
