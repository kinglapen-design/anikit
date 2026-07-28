<script setup lang="ts">
// 视频播放条（5.2）：放搜索栏位置，样式照搜索框（C3 / inset border-1 / 圆角 8 / 内边距 4 / 间距 6）。
// 播放/暂停、快退/快进（∓5 秒）、上/下一帧（step-frame 图标左右翻转，步进即暂停，快捷键 , . 的实体入口）、
// 音量（VolumeControl）、当前帧/总帧数（5.3 帧化：查看方式"时间显示 = 帧数"且 fps 可用；
// 回退 当前时间/总时长）、
// 进度条（3px 轨道 border-1 + 品牌紫填充 + 10px 拖钮，同帧播放条自绘样式）、
// 倍率（RateControl 悬停竖排弹窗）、循环、
// 笔记（浮层）、当前画面设为预览图（set_video_thumb）、显示器全屏。转码中显示加载提示。
import { computed, ref } from "vue";
import IconButton from "./IconButton.vue";
import VolumeControl from "./VolumeControl.vue";
import RateControl from "./RateControl.vue";
import { formatDuration } from "../lib/formatDuration";
import { formatFrameRange } from "../lib/formatFrame";
import { useUiStore } from "../stores/ui";
import { usePreviewStore } from "../stores/preview";
import { usePreviewMediaStore } from "../stores/previewMedia";
import { usePreviewNotesStore } from "../stores/previewNotes";

const ui = useUiStore();
const preview = usePreviewStore();
const media = usePreviewMediaStore();
const notes = usePreviewNotesStore();
const trackRef = ref<HTMLElement>();
let seeking = false;

const pct = computed(() => (media.duration > 0 ? (media.currentTime / media.duration) * 100 : 0));

// 帧显示（5.3 帧化改造，2026-07-28 用户定：帧数优先）：查看方式"时间显示 = 帧数"且 fps 可用
// → 当前帧/总帧数；fps 缺失或时间码模式回退时间显示（mm:ss）
const useFrames = computed(() => ui.timeDisplay === "frames" && media.fps != null && media.fps > 0);
const posText = computed(() =>
  useFrames.value && media.fps
    ? formatFrameRange(media.currentTime, media.duration, media.fps)
    : `${formatDuration(media.currentTime)} / ${formatDuration(media.duration)}`,
);

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
// 逐帧步进（步进即暂停；按钮 = 快捷键 , . 的实体入口，2026-07-26 用户定）。
// 步长（5.3 帧化改造，2026-07-28 用户定："逐帧"语义与显示模式解耦）——
// fps 可用恒按 1/fps（素材自身帧率，时间码模式也一样），fps 缺失才回退 ±1/24s
function stepFrame(delta: number): void {
  media.pause();
  media.seekBy(delta / (media.fps != null && media.fps > 0 ? media.fps : 24));
}
</script>

<template>
  <div class="play-bar">
    <!-- 转码中保持播放条结构（高度不跳变防闪烁，2026-07-26 用户）：控件禁用，提示放进度条区 -->
    <IconButton
      :name="media.playing ? 'pause' : 'play'"
      :title="media.playing ? '暂停' : '播放'"
      :disabled="media.transcodingSlow"
      @click="media.togglePlay()"
    />
    <IconButton name="rewind" title="快退 5 秒" :disabled="media.transcodingSlow" @click="media.seekBy(-5)" />
    <IconButton name="fast-forward" title="快进 5 秒" :disabled="media.transcodingSlow" @click="media.seekBy(5)" />
    <!-- 上下帧（逐帧步进即暂停；step-frame 图标左右翻转使用，快捷键 , . 的实体入口） -->
    <IconButton class="flip-h" name="step-frame" title="上一帧（,）" :disabled="media.transcodingSlow" @click="stepFrame(-1)" />
    <IconButton name="step-frame" title="下一帧（.）" :disabled="media.transcodingSlow" @click="stepFrame(1)" />
    <VolumeControl />
    <span class="time-text">{{ posText }}</span>
    <div
      ref="trackRef"
      class="progress"
      :class="{ transcoding: media.transcodingSlow }"
      style="touch-action: none"
      @pointerdown="onSeekDown"
      @pointermove="onSeekMove"
      @pointerup="onSeekUp"
    >
      <span v-if="media.transcodingSlow" class="hint">转码中…</span>
      <template v-else>
        <span class="fill" :style="{ width: `${pct}%` }" />
        <span class="knob" :style="{ left: `${pct}%` }" />
      </template>
    </div>
    <RateControl :rate="media.rate" @select="media.setRate" />
    <IconButton name="loop" title="循环播放" :active="media.loop" @click="media.toggleLoop()" />
    <IconButton name="annotate" title="笔记" :active="notes.open" :disabled="media.transcodingSlow" @click="notes.toggle()" />
    <IconButton name="set-as-thumb" title="当前画面设为预览图" :disabled="media.transcodingSlow" @click="media.setVideoThumb()" />
    <IconButton
      name="fullscreen"
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
/* 转码加载提示（说明文字档；进度条区内居中——转码中播放条结构保持，高度不跳变防闪烁） */
.hint {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--font-icon-3);
  pointer-events: none;
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
/* 转码中：轨道隐藏（防与"转码中…"文字交叠，2026-07-26 用户定） */
.progress.transcoding::before {
  display: none;
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
/* 上一帧按钮：step-frame 图标水平翻转（左右翻转使用，2026-07-26 用户定） */
.flip-h :deep(svg) {
  transform: scaleX(-1);
}
</style>
