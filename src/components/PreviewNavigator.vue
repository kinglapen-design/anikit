<script setup lang="ts">
// 视图导航器（4.3，Eagle 式）：图片放大超出可视范围时右下角出现，还原后消失。
// 小图 = info 缩略图（跟随旋转/翻转）；视口框 = 当前可见区在图上的映射，
// 拖视口框移动画面、点击小图跳转该点居中。
import { computed, inject, ref } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { PREVIEW_SESSION_KEY } from "../lib/previewSessionKey";

const preview = inject(PREVIEW_SESSION_KEY)!;
const boxRef = ref<HTMLElement>();

// 小图用 info 缩略图（320 短边，比原图省内存）；无缩略图回退显示图
const thumbUrl = computed(() => {
  const p = preview.current;
  const path = p?.thumbPath ?? preview.filePath;
  return path ? convertFileSrc(path) : null;
});

// 图片视觉区域（旋转后宽高互换）的显示尺寸
const dispW = computed(() => preview.visualW * preview.zoom);
const dispH = computed(() => preview.visualH * preview.zoom);

// 小图盒尺寸：按视觉比例缩进 160×120 内
const boxSize = computed(() => {
  const vw = preview.visualW || 1;
  const vh = preview.visualH || 1;
  const k = Math.min(160 / vw, 120 / vh);
  return { w: Math.max(24, vw * k), h: Math.max(24, vh * k) };
});

// 视口框（相对图片视觉区域的 0-1 比例）：
// 容器坐标（中心原点）下图片占 [-W'/2+pan, W'/2+pan]，可见区 = [-view/2, view/2]
function viewportRect(): { left: number; top: number; width: number; height: number } {
  const W = dispW.value;
  const H = dispH.value;
  if (!W || !H) return { left: 0, top: 0, width: 1, height: 1 };
  const vLeft = Math.max(-W / 2 + preview.panX, -preview.viewW / 2);
  const vRight = Math.min(W / 2 + preview.panX, preview.viewW / 2);
  const vTop = Math.max(-H / 2 + preview.panY, -preview.viewH / 2);
  const vBottom = Math.min(H / 2 + preview.panY, preview.viewH / 2);
  return {
    left: (vLeft + W / 2 - preview.panX) / W,
    top: (vTop + H / 2 - preview.panY) / H,
    width: Math.max(0.02, (vRight - vLeft) / W),
    height: Math.max(0.02, (vBottom - vTop) / H),
  };
}
const rect = computed(viewportRect);

// 小图上位置 (ratioX, ratioY) → pan 使该点居中
function panTo(ratioX: number, ratioY: number): void {
  preview.panX = (0.5 - ratioX) * dispW.value;
  preview.panY = (0.5 - ratioY) * dispH.value;
}

let dragging = false;
function ratioOf(e: PointerEvent): { x: number; y: number } {
  const r = boxRef.value!.getBoundingClientRect();
  return {
    x: Math.min(1, Math.max(0, (e.clientX - r.left) / r.width)),
    y: Math.min(1, Math.max(0, (e.clientY - r.top) / r.height)),
  };
}
function onDown(e: PointerEvent): void {
  dragging = true;
  boxRef.value!.setPointerCapture(e.pointerId);
  const p = ratioOf(e);
  panTo(p.x, p.y);
  e.stopPropagation(); // 不触发 stage 平移
}
function onMove(e: PointerEvent): void {
  if (!dragging) return;
  const p = ratioOf(e);
  panTo(p.x, p.y);
}
function onUp(e: PointerEvent): void {
  if (!dragging) return;
  dragging = false;
  boxRef.value!.releasePointerCapture(e.pointerId);
}

const imgStyle = computed(() => ({
  transform: `${preview.flipH ? "scaleX(-1) " : ""}rotate(${preview.rotate}deg)`,
}));
</script>

<template>
  <div class="navigator" @pointerdown.stop>
    <div
      ref="boxRef"
      class="nav-box"
      :style="{ width: `${boxSize.w}px`, height: `${boxSize.h}px` }"
      @pointerdown="onDown"
      @pointermove="onMove"
      @pointerup="onUp"
    >
      <img v-if="thumbUrl" :src="thumbUrl" :style="imgStyle" draggable="false" />
      <span
        class="viewport"
        :style="{
          left: `${rect.left * 100}%`,
          top: `${rect.top * 100}%`,
          width: `${rect.width * 100}%`,
          height: `${rect.height * 100}%`,
        }"
      />
    </div>
  </div>
</template>

<style scoped>
/* 右下角悬浮：距右下各 16；n-g3 底 / 1px n-l-a2 / 圆角 8 / 浮层阴影 */
.navigator {
  position: absolute;
  right: 16px;
  bottom: 16px;
  padding: var(--n-space-4);
  background: var(--n-g3);
  border-radius: var(--n-radius-8);
  box-shadow:
    inset 0 0 0 1px var(--n-l-a2),
    var(--n-shadow-lg);
  cursor: crosshair;
  user-select: none;
}
.nav-box {
  position: relative;
  overflow: hidden;
  border-radius: var(--n-radius-4);
}
.nav-box img {
  display: block;
  width: 100%;
  height: 100%;
}
/* 视口框：1px 亮线（Eagle 式白框） */
.viewport {
  position: absolute;
  box-shadow: inset 0 0 0 1px var(--n-l1);
  pointer-events: none;
}
</style>
