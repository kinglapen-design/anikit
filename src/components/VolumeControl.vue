<script setup lang="ts">
// 音量控件（5.2，音频/视频播放条共用）：图标点击切静音（volume/mute 图标随状态），
// 悬停出竖向滑块弹层调音量（与倍率弹窗同款容器：C3 / inset 1px border-1 / 圆角 8 /
// --shadow-pop；内边距上下 8 左右 4——细长 item 上下留白为左右两倍，拖钮不出容器；
// 竖向轨道高 96，3px 轨道 border-1 底 + 品牌紫填充 + 10px 拖钮，
// 同播放进度条自绘口径转 90°，SideSlider 为横向缩放滑块专用不复用）。
// 拖/点设音量 0-1：静音时滑块归 0 联动显示，拖离 0 经 setVolume 自动解除静音。
// 弹层底部 4px 透明桥接区（padding）保证指针从按钮滑入弹层不断开。
// 5.3 参数化：状态与行为改 props/emits（音频 = previewMedia，视频 = preview store 帧流会话）。
import { computed, ref } from "vue";
import IconButton from "./IconButton.vue";

const props = defineProps<{ volume: number; muted: boolean }>();
const emit = defineEmits<{ setVolume: [ratio: number]; toggleMute: [] }>();
const open = ref(false);
const trackRef = ref<HTMLElement>();
let dragging = false;

// 静音时滑块显示 0（联动），实际音量保留在 props.volume 供取消静音恢复
const pct = computed(() => (props.muted ? 0 : props.volume) * 100);

function apply(e: PointerEvent): void {
  const rect = trackRef.value!.getBoundingClientRect();
  const ratio = Math.max(0, Math.min(1, 1 - (e.clientY - rect.top) / rect.height));
  emit("setVolume", ratio);
}
function onDown(e: PointerEvent): void {
  dragging = true;
  trackRef.value!.setPointerCapture(e.pointerId);
  apply(e);
  e.preventDefault();
}
function onMove(e: PointerEvent): void {
  if (dragging) apply(e);
}
function onUp(e: PointerEvent): void {
  if (!dragging) return;
  dragging = false;
  trackRef.value!.releasePointerCapture(e.pointerId);
}
</script>

<template>
  <div class="volume" @pointerenter="open = true" @pointerleave="open = false">
    <IconButton
      :name="muted || volume === 0 ? 'mute' : 'volume'"
      :title="muted ? '取消静音' : '静音'"
      @click="emit('toggleMute')"
    />
    <div v-if="open" class="vol-pop-wrap">
      <div class="vol-pop">
        <div
          ref="trackRef"
          class="vol-track"
          style="touch-action: none"
          @pointerdown="onDown"
          @pointermove="onMove"
          @pointerup="onUp"
        >
          <span class="fill" :style="{ height: `${pct}%` }" />
          <span class="knob" :style="{ bottom: `${pct}%` }" />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.volume {
  position: relative;
  display: inline-flex;
  flex: none;
}
/* 浮层定位壳：按钮正上方，padding-bottom 4 作指针桥接 */
.vol-pop-wrap {
  position: absolute;
  bottom: 100%;
  left: 50%;
  transform: translateX(-50%);
  padding-bottom: var(--space-4);
  z-index: 10;
}
/* 浮层卡片：C3 / inset 1px border-1 / 圆角 8 / --shadow-pop；
   内边距上下 8 左右 4（细长 item 上下留白为左右两倍，拖钮不出容器，2026-07-26 用户定） */
.vol-pop {
  display: flex;
  padding: var(--space-8) var(--space-4);
  border-radius: var(--radius-8);
  background: var(--c3);
  box-shadow:
    var(--shadow-pop),
    inset 0 0 0 1px var(--border-1);
}
/* 竖向轨道：高 96 宽 16（点击热区），3px 轨道 border-1 底，品牌紫填充自底向上，10px 拖钮 */
.vol-track {
  position: relative;
  width: 16px;
  height: 96px;
  cursor: pointer;
}
.vol-track::before {
  content: "";
  position: absolute;
  top: 0;
  bottom: 0;
  left: 50%;
  width: 3px;
  transform: translateX(-50%);
  border-radius: 2px;
  background: var(--border-1);
}
.fill {
  position: absolute;
  bottom: 0;
  left: 50%;
  width: 3px;
  transform: translateX(-50%);
  border-radius: 2px;
  background: var(--brand-1);
  pointer-events: none;
}
.knob {
  position: absolute;
  left: 50%;
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--font-icon-1);
  transform: translate(-50%, 50%);
  pointer-events: none;
}
</style>
