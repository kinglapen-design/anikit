<script setup lang="ts">
// 序列帧播放条（4.3；动图 5.2 完全复用）：放搜索栏位置，样式照搜索框（C3 / inset border-1 / 圆角 8）。
// 播放/暂停、当前帧/总帧数、进度条（拖动定位帧）、倍率（点击循环切换）、
// 笔记（占位 disabled，5.2 统一实现）、当前帧设为预览图、显示器全屏（Esc 退出）。
import { ref } from "vue";
import AppIcon from "./AppIcon.vue";
import IconButton from "./IconButton.vue";
import { RATES, usePreviewStore } from "../stores/preview";

const preview = usePreviewStore();
const trackRef = ref<HTMLElement>();
let seeking = false;

function cycleRate(): void {
  const i = RATES.indexOf(preview.rate);
  preview.setRate(RATES[(i + 1) % RATES.length]);
}

function seek(e: PointerEvent): void {
  const rect = trackRef.value!.getBoundingClientRect();
  const ratio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
  const n = preview.seq?.frames.length ?? 1;
  preview.setFrame(Math.round(ratio * (n - 1)));
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
  <div v-if="preview.seq" class="play-bar">
    <IconButton
      :name="preview.playing ? 'pause' : 'play'"
      :title="preview.playing ? '暂停' : '播放'"
      @click="preview.togglePlay()"
    />
    <span class="frame-text">{{ preview.frame + 1 }} / {{ preview.seq.frames.length }}</span>
    <div
      ref="trackRef"
      class="progress"
      style="touch-action: none"
      @pointerdown="onSeekDown"
      @pointermove="onSeekMove"
      @pointerup="onSeekUp"
    >
      <span
        class="fill"
        :style="{ width: `${(preview.frame / Math.max(1, preview.seq.frames.length - 1)) * 100}%` }"
      />
      <span
        class="knob"
        :style="{ left: `${(preview.frame / Math.max(1, preview.seq.frames.length - 1)) * 100}%` }"
      />
    </div>
    <button class="rate-btn" v-tip="'倍率（点击切换）'" @click="cycleRate">{{ preview.rate }}x</button>
    <IconButton name="annotate" title="笔记（随 5.2 动图/音视频统一实现）" disabled />
    <IconButton name="set-as-thumb" title="当前画面设为预览图" @click="preview.setAsThumb()" />
    <IconButton
      :name="preview.fullscreen ? 'fullscreen' : 'fullscreen'"
      :title="preview.fullscreen ? '退出全屏' : '全屏'"
      @click="preview.setFullscreen(!preview.fullscreen)"
    />
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
.frame-text {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--font-icon-1);
  white-space: nowrap;
  flex: none;
}
/* 中栏工具栏图标按钮 80%（2026-07-23 用户定，同预览标题栏） */
.play-bar :deep(.icon-btn) {
  color: var(--font-icon-2);
}
/* 进度条：3px 轨道 border-1 底，品牌紫填充，10px 拖钮（SideSlider 语言） */
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
