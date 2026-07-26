<script setup lang="ts">
// 视频播放条（5.2）：放搜索栏位置，样式照搜索框（C3 / inset border-1 / 圆角 8 / 内边距 4 / 间距 6）。
// 播放/暂停、快退/快进（∓5 秒）、音量（VolumeControl）、当前时间/总时长、
// 进度条（3px 轨道 border-1 + 品牌紫填充 + 10px 拖钮，同帧播放条自绘样式）、倍率、循环、
// 笔记（浮层）、当前画面设为预览图（set_video_thumb）、显示器全屏。转码中显示加载提示。
import { computed, ref } from "vue";
import IconButton from "./IconButton.vue";
import VolumeControl from "./VolumeControl.vue";
import { formatDuration } from "../lib/formatDuration";
import { RATES, usePreviewStore } from "../stores/preview";
import { usePreviewMediaStore } from "../stores/previewMedia";
import { usePreviewNotesStore } from "../stores/previewNotes";

const preview = usePreviewStore();
const media = usePreviewMediaStore();
const notes = usePreviewNotesStore();
const trackRef = ref<HTMLElement>();
let seeking = false;

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
      <IconButton name="rewind" title="快退 5 秒" @click="media.seekBy(-5)" />
      <IconButton name="fast-forward" title="快进 5 秒" @click="media.seekBy(5)" />
      <VolumeControl />
      <span class="time-text">{{ formatDuration(media.currentTime) }} / {{ formatDuration(media.duration) }}</span>
      <div
        ref="trackRef"
        class="progress"
        style="touch-action: none"
        @pointerdown="onSeekDown"
        @pointermove="onSeekMove"
        @pointerup="onSeekUp"
      >
        <span class="fill" :style="{ width: `${pct}%` }" />
        <span class="knob" :style="{ left: `${pct}%` }" />
      </div>
      <button class="rate-btn" v-tip="'倍率（点击切换）'" @click="cycleRate">{{ media.rate }}x</button>
      <IconButton name="loop" title="循环播放" :active="media.loop" @click="media.toggleLoop()" />
      <IconButton name="annotate" title="笔记" :active="notes.open" @click="notes.toggle()" />
      <IconButton name="set-as-thumb" title="当前画面设为预览图" @click="media.setVideoThumb()" />
      <IconButton
        name="fullscreen"
        :title="preview.fullscreen ? '退出全屏' : '全屏'"
        @click="preview.setFullscreen(!preview.fullscreen)"
      />
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
/* 进度条：3px 轨道 border-1 底，品牌紫填充，10px 拖钮（SideSlider 语言，同帧播放条） */
.progress {
  position: relative;
  flex: 1;
  min-width: 0;
  height: 16px;
  cursor: pointer;
}
.progress::before {
  content: "";
  position: absolute;
  left: 0;
  right: 0;
  top: 50%;
  height: 3px;
  transform: translateY(-50%);
  border-radius: 2px;
  background: var(--border-1);
}
.fill {
  position: absolute;
  left: 0;
  top: 50%;
  height: 3px;
  transform: translateY(-50%);
  border-radius: 2px;
  background: var(--brand-1);
  pointer-events: none;
}
.knob {
  position: absolute;
  top: 50%;
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--font-icon-1);
  transform: translate(-50%, -50%);
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
