<script setup lang="ts">
// 预览画面区（4.3/5.2）：image/sequence/anim 走 <img>（filePath 由 store 维护，切帧即换 src）；
// video 把 previewMedia 的 <video> 元素挂进画面区（缩放/平移/旋转/翻转复用图片那套数学，
// 尺寸经 loadedmetadata → onImageLoad 喂入）；audio 显示波形大图（按画面区实际像素后端重采样，
// 未就绪回退 info 波形缩略图 contain），无缩放平移。
// 交互：滚轮缩放（瞄点跟鼠标）、左键拖动平移、双击 适合窗口⇄原图大小；
// ←/→ sequence/anim=上/下帧 video=逐帧步进(±1/24s) 其余=上/下一个素材；
// 空格 sequence/anim/audio/video=播放暂停；Esc 退出全屏/预览。
import { computed, onBeforeUnmount, onMounted, ref, watch, watchEffect } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import PreviewNavigator from "./PreviewNavigator.vue";
import { useHiresWaveform } from "../lib/useHiresWaveform";
import { usePreviewStore } from "../stores/preview";
import { usePreviewMediaStore } from "../stores/previewMedia";

const preview = usePreviewStore();
const media = usePreviewMediaStore();
const stageRef = ref<HTMLElement>();
const videoWrapRef = ref<HTMLElement>();
let ro: ResizeObserver | undefined;

const url = computed(() => (preview.filePath ? convertFileSrc(preview.filePath) : null));
// <img> 渲染的 kind（video 走元素挂载，audio 走波形大图）
const showImg = computed(
  () => url.value && preview.kind !== "video" && preview.kind !== "audio" && preview.kind !== "other",
);
// other 保底：格式图标大图
const iconUrl = computed(() => {
  const p = preview.current;
  return preview.kind === "other" && p?.thumbPath ? convertFileSrc(p.thumbPath) : null;
});
// audio：波形大图——优先后端按画面区实际像素重采样的高清图（useHiresWaveform 内部已按
// kind==='audio' 门控），未就绪 / 生成失败回退 info 波形缩略图（5.1）拉伸旧行为
const hiresWavePath = useHiresWaveform(stageRef);
const waveUrl = computed(() => {
  const p = preview.current;
  if (preview.kind !== "audio") return null;
  const src = hiresWavePath.value ?? p?.thumbPath;
  return src ? convertFileSrc(src) : null;
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

// video：元素挂进画面区（模块级单例，kind 切入时 append；切走随 v-if 脱离 DOM，store 仍持有）
watch(
  [() => preview.kind, videoWrapRef],
  ([k]) => {
    if (k === "video" && videoWrapRef.value) {
      const v = media.getEl();
      if (v.parentElement !== videoWrapRef.value) videoWrapRef.value.appendChild(v);
    }
  },
  { immediate: true, flush: "post" },
);
// video 旋转/翻转同步到元素（wrap 定位/pan/zoom 数学不受影响）
watchEffect(() => {
  if (preview.kind !== "video") return;
  media.getEl().style.transform = imgStyle.value.transform;
});

function onLoad(e: Event): void {
  const img = e.target as HTMLImageElement;
  preview.onImageLoad(img.naturalWidth, img.naturalHeight);
}

// 滚轮缩放：瞄点跟鼠标位置（2026-07-25 定案）；audio/other 无查看器交互
function onWheel(e: WheelEvent): void {
  if (!preview.viewable) return;
  const rect = stageRef.value!.getBoundingClientRect();
  const factor = e.deltaY < 0 ? 1.15 : 1 / 1.15;
  preview.zoomAt(preview.zoom * factor, e.clientX - rect.left, e.clientY - rect.top);
  e.preventDefault();
}

// 左键拖动平移
let dragging = false;
function onPointerDown(e: PointerEvent): void {
  if (e.button !== 0 || !preview.viewable) return;
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
  if (!preview.viewable) return;
  if (Math.abs(preview.zoom - preview.fitZoom) < 0.01) preview.actual();
  else preview.fit();
}

// 逐帧步进（, 和 . = YouTube / mpv / Premiere / Animate 同约定，2026-07-26 用户定）：
// 序列帧/动图步一帧（stepFrame 内含步进即暂停）；视频暂停后 ±1/24s；其他类型无逐帧
function stepOneFrame(delta: number): void {
  if (preview.kind === "sequence" || preview.kind === "anim") preview.stepFrame(delta);
  else if (preview.kind === "video") {
    media.pause();
    media.seekBy(delta / 24);
  }
}

function onKeydown(e: KeyboardEvent): void {
  const t = e.target as HTMLElement;
  if (t.tagName === "INPUT" || t.tagName === "TEXTAREA") return; // 笔记编辑中不抢键（5.2）
  switch (e.key) {
    case "Escape":
      if (preview.fullscreen) void preview.setFullscreen(false);
      else preview.close();
      break;
    case "ArrowLeft": // ←/→ 全类型统一 = 上/下一个素材（2026-07-26 用户定，逐帧让位给 , .）
      void preview.step(-1);
      break;
    case "ArrowRight":
      void preview.step(1);
      break;
    case ",":
      stepOneFrame(-1);
      break;
    case ".":
      stepOneFrame(1);
      break;
    case " ":
      if (preview.kind === "sequence" || preview.kind === "anim") preview.togglePlay();
      else if (preview.kind === "audio" || preview.kind === "video") media.togglePlay();
      else break;
      e.preventDefault();
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

// 切素材时结束拖动
watch(
  () => preview.assetKey,
  () => (dragging = false),
);
</script>

<template>
  <div
    ref="stageRef"
    class="preview-stage"
    :class="{ dragging: dragging || undefined, pannable: preview.viewable }"
    @wheel="onWheel"
    @pointerdown="onPointerDown"
    @pointermove="onPointerMove"
    @pointerup="onPointerUp"
    @dblclick="onDblClick"
  >
    <div v-if="showImg" class="img-wrap" :style="wrapStyle">
      <img :src="url!" :style="imgStyle" draggable="false" @load="onLoad" />
    </div>
    <div v-else-if="preview.kind === 'video'" ref="videoWrapRef" class="img-wrap" :style="wrapStyle" />
    <div v-else-if="waveUrl" class="wave-large" :style="{ '--wave-url': `url(&quot;${waveUrl}&quot;)` }" />
    <img v-else-if="iconUrl" class="format-icon" :src="iconUrl" draggable="false" />
    <span v-if="media.transcoding" class="hint">转码中…</span>
    <span v-if="preview.error" class="error-text">{{ preview.error }}</span>
    <PreviewNavigator v-if="preview.overflow" />
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
/* audio：波形大图（重采样图按画面区等比渲染；CSS mask 方案——图柱 alpha 恒 100% 供 mask 取形，
   颜色走 token font-icon-3 与原 60% 白观感一致，mask-size contain 居中即原 object-fit 行为） */
.wave-large {
  position: absolute;
  inset: 0;
  background: var(--font-icon-3);
  -webkit-mask-image: var(--wave-url);
  mask-image: var(--wave-url);
  -webkit-mask-size: contain;
  mask-size: contain;
  -webkit-mask-position: center;
  mask-position: center;
  -webkit-mask-repeat: no-repeat;
  mask-repeat: no-repeat;
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
/* 转码加载态 / 媒体源报错文案（说明文字档，居中底部） */
.hint,
.error-text {
  position: absolute;
  left: 50%;
  bottom: var(--space-16);
  transform: translateX(-50%);
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--font-icon-3);
  white-space: nowrap;
}
.error-text {
  color: var(--error);
}
</style>
