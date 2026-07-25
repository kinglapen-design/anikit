<script setup lang="ts">
// 预览画面区（4.3）：图片/源文件/序列帧共用渲染（filePath 由 store 维护，序列帧切帧即换 src）。
// 缩放/平移作用于外层 wrap（left/top 计算式定位，无 transform-origin 坑）；
// 旋转/翻转只作用于 <img> 自身，pan/zoom 数学不受影响。
// 交互：滚轮缩放（瞄点跟鼠标）、左键拖动平移、双击 适合窗口⇄原图大小、
// ←/→ 图片=上/下一个素材 序列帧=上/下帧、空格播放暂停、Esc 退出全屏/预览。
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import PreviewNavigator from "./PreviewNavigator.vue";
import { usePreviewStore } from "../stores/preview";

const preview = usePreviewStore();
const stageRef = ref<HTMLElement>();
let ro: ResizeObserver | undefined;

const url = computed(() => (preview.filePath ? convertFileSrc(preview.filePath) : null));
// other 保底：格式图标大图
const iconUrl = computed(() => {
  const p = preview.current;
  return preview.kind === "other" && p?.thumbPath ? convertFileSrc(p.thumbPath) : null;
});

// wrap 定位：图中心 = 容器中心 + pan，尺寸 = 自然尺寸 × zoom（left/top 直算，无 origin 问题）
const wrapStyle = computed(() => ({
  left: `${preview.viewW / 2 - (preview.naturalW * preview.zoom) / 2 + preview.panX}px`,
  top: `${preview.viewH / 2 - (preview.naturalH * preview.zoom) / 2 + preview.panY}px`,
  width: `${preview.naturalW * preview.zoom}px`,
  height: `${preview.naturalH * preview.zoom}px`,
}));
const imgStyle = computed(() => ({
  transform: `${preview.flipH ? "scaleX(-1) " : ""}rotate(${preview.rotate}deg)`,
}));

function onLoad(e: Event): void {
  const img = e.target as HTMLImageElement;
  preview.onImageLoad(img.naturalWidth, img.naturalHeight);
}

// 滚轮缩放：瞄点跟鼠标位置（2026-07-25 定案）
function onWheel(e: WheelEvent): void {
  if (preview.kind === "other") return;
  const rect = stageRef.value!.getBoundingClientRect();
  const factor = e.deltaY < 0 ? 1.15 : 1 / 1.15;
  preview.zoomAt(preview.zoom * factor, e.clientX - rect.left, e.clientY - rect.top);
  e.preventDefault();
}

// 左键拖动平移
let dragging = false;
function onPointerDown(e: PointerEvent): void {
  if (e.button !== 0 || preview.kind === "other") return;
  dragging = true;
  stageRef.value!.setPointerCapture(e.pointerId);
}
function onPointerMove(e: PointerEvent): void {
  if (!dragging) return;
  preview.panX += e.movementX;
  preview.panY += e.movementY;
}
function onPointerUp(e: PointerEvent): void {
  if (!dragging) return;
  dragging = false;
  stageRef.value!.releasePointerCapture(e.pointerId);
}

// 双击：适合窗口 ⇄ 原图大小
function onDblClick(): void {
  if (preview.kind === "other") return;
  if (Math.abs(preview.zoom - preview.fitZoom) < 0.01) preview.actual();
  else preview.fit();
}

function onKeydown(e: KeyboardEvent): void {
  switch (e.key) {
    case "Escape":
      if (preview.fullscreen) void preview.setFullscreen(false);
      else preview.close();
      break;
    case "ArrowLeft":
      if (preview.kind === "sequence") preview.stepFrame(-1);
      else void preview.step(-1);
      break;
    case "ArrowRight":
      if (preview.kind === "sequence") preview.stepFrame(1);
      else void preview.step(1);
      break;
    case " ":
      if (preview.kind === "sequence") {
        preview.togglePlay();
        e.preventDefault();
      }
      break;
  }
}

onMounted(() => {
  ro = new ResizeObserver(() => {
    const el = stageRef.value;
    if (!el) return;
    const r = el.getBoundingClientRect();
    preview.viewW = r.width;
    preview.viewH = r.height;
  });
  ro.observe(stageRef.value!);
  window.addEventListener("keydown", onKeydown);
});
onBeforeUnmount(() => {
  ro?.disconnect();
  window.removeEventListener("keydown", onKeydown);
});

// 全屏时禁止标题栏拖拽区域误触（预览区无拖拽区，无需处理）；切素材时结束拖动
watch(
  () => preview.assetKey,
  () => (dragging = false),
);
</script>

<template>
  <div
    ref="stageRef"
    class="preview-stage"
    :class="{ dragging: dragging || undefined, pannable: preview.kind !== 'other' }"
    @wheel="onWheel"
    @pointerdown="onPointerDown"
    @pointermove="onPointerMove"
    @pointerup="onPointerUp"
    @dblclick="onDblClick"
  >
    <div v-if="url" class="img-wrap" :style="wrapStyle">
      <img :src="url" :style="imgStyle" draggable="false" @load="onLoad" />
    </div>
    <img v-else-if="iconUrl" class="format-icon" :src="iconUrl" draggable="false" />
    <PreviewNavigator v-if="preview.overflow && preview.kind !== 'other'" />
  </div>
</template>

<style scoped>
.preview-stage {
  position: relative;
  flex: 1;
  min-height: 0;
  overflow: hidden;
  touch-action: none;
  user-select: none;
}
.preview-stage.pannable {
  cursor: grab;
}
.preview-stage.dragging {
  cursor: grabbing;
}
.img-wrap {
  position: absolute;
  /* 尺寸由 style 绑定（naturalW/H × zoom） */
}
.img-wrap img {
  display: block;
  width: 100%;
  height: 100%;
  /* 旋转/翻转绕图中心，视觉可溢出 wrap（stage overflow hidden 兜底） */
}
.format-icon {
  position: absolute;
  left: 50%;
  top: 50%;
  transform: translate(-50%, -50%);
  width: 256px;
  height: 256px;
  object-fit: contain;
}
</style>
