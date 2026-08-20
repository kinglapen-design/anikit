<script setup lang="ts">
// 帧播放条（4.3 序列帧；5.2 动图复用 → 阶段四动图改帧流驱动，2026-08-20）：
// 样式 = 13_播放条稿（2026-08-20 用户定，与搜索栏同位同尺寸）——46 高 / padding 8 16 / C3 /
// border-top 暗-悬停 / 直角通栏；图标按实际删减（动图无音量/快进快退，上·下帧不补——帧条两端已有）。
// 播放/暂停、当前帧/总帧数、进度条（拖动定位帧）、倍率（RateControl 悬停竖排弹窗）、
// 笔记（5.2 浮层，开着时品牌紫激活态）、当前帧设为预览图、显示器全屏（Esc 退出）。
// 动图播放 = 帧流，设封面 / 笔记 / 帧条 = 后端 `f:{索引}` 抽帧——全链路不依赖磁盘帧图
// （2026-08-20 解包废除）；帧流失败回退态（animTotal=0）不出播放条（原生 img 兜底播放，无法控制）。
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
    <div class="btn-group">
      <IconButton
        :name="preview.playing ? 'pause' : 'play'"
        :title="preview.playing ? '暂停' : '播放'"
        @click="preview.togglePlay()"
      />
    </div>
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
    <div class="btn-group">
      <RateControl :rate="preview.rate" @select="preview.setRate" />
      <IconButton name="add-note" title="笔记" :active="notes.open" @click="notes.toggle()" />
      <IconButton name="set-thumbnail" title="当前画面设为预览图" @click="preview.setAsThumb()" />
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
/* 时间码位（13 稿：72 宽 14/20 文字/常态——min-width 防长帧号溢出） */
.frame-text {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-normal);
  min-width: 72px;
  white-space: nowrap;
  flex: none;
}
/* 播放条图标 = 标签档（13 稿 Vector 文字/标签） */
.play-bar :deep(.icon-btn) {
  color: var(--text-label);
}
/* 激活态优先于标签档（笔记开着 = 品牌紫） */
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
</style>
