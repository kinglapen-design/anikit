<script setup lang="ts">
// 视频播放条（5.2 样式 / 5.3 帧流化）：样式 = 13_播放条稿（2026-08-20 用户定，与搜索栏同位同尺寸）——
// 46 高 / padding 8 16 / C3 / border-top 暗-悬停 / 直角通栏；左组 = 播放/快退/快进/上帧/下帧/音量（稿全量件）。
// 5.3 全面接入自研解码器：数据源 = preview store 视频帧流态（WS 帧流渲染 + PCM 音频），无 <video> 元素、
// 无转码（原"转码中…"加载态随 <video> 链退役）。播放/暂停、快退/快进（∓5 秒；seek 原样 ◀◀ = 快退、
// flip-h = 快进）、上/下一帧（step-frame-vertical 原样 ◀| = 上一帧、flip-h |▶ = 下一帧；
// 真逐帧 = 暂停 + idx±1 定位停住，快捷键 , . 的实体入口）、音量（VolumeControl，有音轨才出）、
// 当前帧/总帧数（TimeDisplayControl：查看方式"时间显示 = 帧数"且 fps 可用，回退 当前时间/总时长；
// 悬停出帧数/时间码就地切换弹窗）、进度条（3px 轨道 暗-悬停 + 品牌紫填充 + 10px 常态色拖钮）、
// 倍率（RateControl 悬停竖排弹窗）、循环（关 = 自然到底停末帧）、笔记（浮层）、
// 当前画面设为预览图（set_video_thumb）、显示器全屏。
// 帧流会话未建立（启动失败）时控件禁用（vStreamLive 闸），画面保持静态缩略图 + 报错文案。
import { computed, ref } from "vue";
import IconButton from "./IconButton.vue";
import VolumeControl from "./VolumeControl.vue";
import RateControl from "./RateControl.vue";
import TimeDisplayControl from "./TimeDisplayControl.vue";
import { formatDuration } from "../lib/formatDuration";
import { totalFrames } from "../lib/formatFrame";
import { useUiStore } from "../stores/ui";
import { usePreviewStore } from "../stores/preview";
import { usePreviewNotesStore } from "../stores/previewNotes";

const ui = useUiStore();
const preview = usePreviewStore();
const notes = usePreviewNotesStore();
const trackRef = ref<HTMLElement>();
let seeking = false;

// 控件禁用闸：帧流会话未建立（启动失败 / 加载中）
const off = computed(() => !preview.vStreamLive);

const pct = computed(() => (preview.vDuration > 0 ? (preview.vSecs / preview.vDuration) * 100 : 0));

// 帧显示（5.3 帧化，2026-07-28 用户定：帧数优先）：查看方式"时间显示 = 帧数"且 fps 可用 →
// 当前帧/总帧数（帧流 idx 0 起 → 显示 1 起）；fps 缺失或时间码模式回退时间显示（mm:ss）
const useFrames = computed(() => ui.timeDisplay === "frames" && preview.vFps != null && preview.vFps > 0);
const posText = computed(() => {
  if (useFrames.value && preview.vFps) {
    const total = totalFrames(preview.vDuration, preview.vFps);
    const cur = Math.min(total, Math.max(1, preview.frame + 1));
    return `${cur} / ${total}`;
  }
  return `${formatDuration(preview.vSecs)} / ${formatDuration(preview.vDuration)}`;
});

function seek(e: PointerEvent): void {
  if (preview.vDuration <= 0) return;
  const rect = trackRef.value!.getBoundingClientRect();
  const ratio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
  preview.videoSeek(ratio * preview.vDuration);
}
function onSeekDown(e: PointerEvent): void {
  if (off.value) return;
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
    <div class="btn-group">
      <IconButton
        :name="preview.playing ? 'pause' : 'play'"
        :title="preview.playing ? '暂停' : '播放'"
        :disabled="off"
        @click="preview.togglePlay()"
      />
      <IconButton name="seek" title="快退 5 秒" :disabled="off" @click="preview.videoSeekBy(-5)" />
      <IconButton class="flip-h" name="seek" title="快进 5 秒" :disabled="off" @click="preview.videoSeekBy(5)" />
      <!-- 上下帧（真逐帧，步进即暂停；step-frame-vertical 原样 ◀| = 上一帧、flip-h |▶ = 下一帧；
           快捷键 , . 的实体入口，tip tag 化同"新增"口径） -->
      <IconButton name="step-frame-vertical" :title="{ text: '上一帧', keys: ',' }" :disabled="off" @click="preview.stepFrame(-1)" />
      <IconButton class="flip-h" name="step-frame-vertical" :title="{ text: '下一帧', keys: '.' }" :disabled="off" @click="preview.stepFrame(1)" />
      <VolumeControl
        v-if="preview.vHasAudio"
        :volume="preview.vVolume"
        :muted="preview.vMuted"
        @set-volume="preview.videoSetVolume"
        @toggle-mute="preview.videoToggleMuted()"
      />
    </div>
    <TimeDisplayControl :text="posText" />
    <div
      ref="trackRef"
      class="progress"
      :class="{ off }"
      style="touch-action: none"
      @pointerdown="onSeekDown"
      @pointermove="onSeekMove"
      @pointerup="onSeekUp"
    >
      <span class="fill" :style="{ width: `${pct}%` }" />
      <span class="knob" :style="{ left: `${pct}%` }" />
    </div>
    <div class="btn-group">
      <RateControl :rate="preview.rate" @select="preview.setRate" />
      <IconButton name="loop" title="循环播放" :active="preview.vLoop" :disabled="off" @click="preview.videoToggleLoop()" />
      <IconButton name="add-note" title="笔记" :active="notes.open" :disabled="off" @click="notes.toggle()" />
      <IconButton name="set-thumbnail" title="当前画面设为预览图" :disabled="off" @click="preview.setVideoThumb()" />
      <IconButton
        name="fullscreen"
        :title="preview.fullscreen ? '退出全屏' : '全屏'"
        @click="preview.setFullscreen(!preview.fullscreen)"
      />
    </div>
  </div>
</template>

<style scoped>
/* 总容器（13_播放条稿，与搜索栏同口径）：46 高 / padding 8 16 / gap 8 / C3 / border-top 暗-悬停 /
   直角通栏（2026-08-20 用户定——原"照搜索框圆角 8 inset 边框"口径作废） */
.play-bar {
  display: flex;
  align-items: center;
  gap: var(--space-8);
  padding: var(--space-8) var(--space-16);
  height: 46px;
  box-sizing: border-box;
  background: var(--c3);
  border-top: 1px solid var(--line-hover);
}
/* 左 / 右按钮组：组内 gap 4（13 稿） */
.btn-group {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  flex: none;
}
/* 播放条图标 = 标签档（13 稿 Vector 文字/标签） */
.play-bar :deep(.icon-btn) {
  color: var(--text-label);
}
/* hover 提亮到常态（2026-08-20 用户定）；激活（品牌紫）/ 禁用态不提亮 */
.play-bar :deep(.icon-btn:hover:not(.active):not(:disabled)) {
  color: var(--text-normal);
}
.play-bar :deep(.icon-btn.active) {
  color: var(--brand-1);
}
/* 进度条（13 稿）：3px 轨道 暗-悬停底，品牌紫填充（已播放表达保留），10px 常态色拖钮 */
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
  background: var(--line-hover);
}
.progress.off {
  pointer-events: none;
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
  background: var(--text-normal);
  transform: translate(-50%, -50%);
  pointer-events: none;
}
/* 上一帧按钮：step-frame 图标水平翻转（左右翻转使用，2026-07-26 用户定） */
.flip-h :deep(svg) {
  transform: scaleX(-1);
}
</style>
