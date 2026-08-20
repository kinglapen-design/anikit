<script setup lang="ts">
// 帧播放条（4.3 序列帧；5.2 动图复用 → 阶段四动图改帧流驱动，2026-08-20）：
// 放搜索栏位置，样式照搜索框（C3 / inset border-1 / 圆角 8）。
// 播放/暂停、当前帧/总帧数、进度条（拖动定位帧）、倍率（RateControl 悬停竖排弹窗）、
// 笔记（5.2 浮层，开着时品牌紫激活态）、当前帧设为预览图、显示器全屏（Esc 退出）。
// 动图播放 = 帧流（不依赖 frames/ 解包）；未解包时帧清单缺失——笔记 / 设封面禁用（播放不受影响）；
// 帧流失败回退态（animTotal=0）不出播放条（原生 img 兜底播放，无法控制）。
import { computed, ref } from "vue";
import IconButton from "./IconButton.vue";
import RateControl from "./RateControl.vue";
import { usePreviewStore } from "../stores/preview";
import { usePreviewNotesStore } from "../stores/previewNotes";

const preview = usePreviewStore();
const notes = usePreviewNotesStore();
const trackRef = ref<HTMLElement>();
let seeking = false;

// 总帧数：序列帧 = 帧清单长度；动图 = 帧流总帧数（metadata frameCount 与已见帧索引取大）
const frameCount = computed(() =>
  preview.kind === "anim" ? preview.animTotal : (preview.seq?.frames.length ?? 0),
);
// 播放条出现条件：序列帧有帧清单；动图帧流活着（animTotal>0。回退态 0 不显示）
const barReady = computed(() => frameCount.value > 0);
// 无帧清单（动图未解包）：笔记 / 设封面禁用（后端需要 frames/ 帧名，会报错）
const noFrameList = computed(() => preview.kind === "anim" && !preview.seq);

function seek(e: PointerEvent): void {
  const rect = trackRef.value!.getBoundingClientRect();
  const ratio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
  preview.setFrame(Math.round(ratio * (frameCount.value - 1)));
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
  <div v-if="barReady" class="play-bar">
    <IconButton
      :name="preview.playing ? 'pause' : 'play'"
      :title="preview.playing ? '暂停' : '播放'"
      @click="preview.togglePlay()"
    />
    <span class="frame-text">{{ preview.frame + 1 }} / {{ frameCount }}</span>
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
        :style="{ width: `${(preview.frame / Math.max(1, frameCount - 1)) * 100}%` }"
      />
      <span
        class="knob"
        :style="{ left: `${(preview.frame / Math.max(1, frameCount - 1)) * 100}%` }"
      />
    </div>
    <RateControl :rate="preview.rate" @select="preview.setRate" />
    <IconButton
      name="add-note"
      title="笔记"
      :active="notes.open"
      :disabled="noFrameList"
      @click="notes.toggle()"
    />
    <IconButton
      name="set-thumbnail"
      title="当前画面设为预览图"
      :disabled="noFrameList"
      @click="preview.setAsThumb()"
    />
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
/* 激活态优先于 80% 档（笔记开着 = 品牌紫） */
.play-bar :deep(.icon-btn.active) {
  color: var(--brand-1);
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
</style>
