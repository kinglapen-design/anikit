<script setup lang="ts">
// 预览画面区（4.3/5.2）：image/sequence 走 <img>（filePath 由 store 维护，切帧即换 src）；
// anim = 阶段四帧流（PreviewStreamCanvas 渲染 canvas，占位缩略图 / 回退 img 在下层）；
// video 把 previewMedia 的 <video> 元素挂进画面区（缩放/平移/旋转/翻转复用图片那套数学，
// 尺寸经 loadedmetadata → onImageLoad 喂入）——以上为宿主级基建分支（缩放 / 元素挂载 / 流画布）；
// 组件型分支（font / text / audio 波形大图 / docx / other 不支持预览立姿）= previewSpecs 注册表按
// preview.kind 查表分发（2026-09-21 R5 扩展点注册表化：新组件型格式 = spec 文件 + 一行注册，
// 本文件零改动——R6 docx 验证落地；分支数据条件如 audio 靠 waveUrl 非 kind 由组件内 v-if 承接；
// 均无缩放平移）。
// 交互：滚轮缩放（瞄点跟鼠标）、左键拖动平移、双击 适合窗口⇄原图大小；
// ←/→ 上/下一个素材；,/ . sequence/anim=上/下帧 video=逐帧步进；
// 空格 sequence/anim/audio/video=播放暂停；Esc 退出全屏/预览。
import { computed, onBeforeUnmount, onMounted, provide, ref, watch, watchEffect } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import PreviewNavigator from "./PreviewNavigator.vue";
import PreviewStreamCanvas from "./PreviewStreamCanvas.vue";
import { PREVIEW_STAGE_KEY, previewPaneSpecFor } from "../lib/previewSpecs";
import { usePreviewStore } from "../stores/preview";
import { usePreviewMediaStore } from "../stores/previewMedia";

const preview = usePreviewStore();
const media = usePreviewMediaStore();
const stageRef = ref<HTMLElement>();
const videoWrapRef = ref<HTMLElement>();
let ro: ResizeObserver | undefined;

// 视口上下文注入：组件型分支按 stage 实寸取数（audio 波形高清重采样测量，见 previewSpecs/types.ts）
provide(PREVIEW_STAGE_KEY, stageRef);

const url = computed(() => (preview.filePath ? convertFileSrc(preview.filePath) : null));
// <img> 渲染的 kind：白名单 = 宿主基建三分支 image / sequence / anim（R6 由排除式否定清单改白名单——
// 新 kind（docx 及未来组件型格式）不再因漏加排除而错误走 <img> 分支；video 走元素挂载，
// 其余组件型分支经 previewSpecs 查表）
const IMG_KINDS = ["image", "sequence", "anim"];
const showImg = computed(() => url.value && IMG_KINDS.includes(preview.kind));
// 组件型分支查表（font / text / audio 波形 / other……；未注册 kind = null 不渲染）
const paneComponent = computed(() => previewPaneSpecFor(preview.kind)?.component ?? null);

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

// 动图帧流出帧尺寸跟随显示尺寸（视口变化 / 滚轮缩放 → 帧短边 × zoom，防抖由后端 clamp 幂等兜底）
watchEffect(() => {
  void preview.viewW;
  void preview.viewH;
  void preview.zoom;
  preview.syncAnimSize();
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
  preview.fitMode = false; // 手动拖动退出"适合窗口"跟随
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
// 序列帧/动图步一帧（stepFrame 内含步进即暂停）；视频暂停后按素材帧率步进（fps 缺失回退 ±1/24s）；
// 其他类型无逐帧
function stepOneFrame(delta: number): void {
  if (preview.kind === "sequence" || preview.kind === "anim") preview.stepFrame(delta);
  else if (preview.kind === "video") {
    media.pause();
    media.seekBy(delta / (media.fps != null && media.fps > 0 ? media.fps : 24));
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
    // 处于"适合窗口"状态时容器变化自动跟随（全屏切换 / 窗口拉伸画面随容器撑满）
    if (preview.fitMode) preview.fit();
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
      <!-- 动图 = 帧流 canvas（叠于占位缩略图 / 回退 img 之上，旋转翻转转 imgStyle 同款 transform） -->
      <PreviewStreamCanvas v-if="preview.kind === 'anim'" :style="imgStyle" />
    </div>
    <div v-else-if="preview.kind === 'video'" ref="videoWrapRef" class="img-wrap" :style="wrapStyle" />
    <!-- 组件型分支 = previewSpecs 查表（font / text / audio 波形 / other；数据空态由组件内部自理） -->
    <component :is="paneComponent" v-else-if="paneComponent" />
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
/* 媒体源报错文案（说明文字档，居中底部；转码提示已挪进播放条进度区，2026-07-26 用户定） */
.error-text {
  position: absolute;
  left: 50%;
  bottom: var(--n-space-16);
  transform: translateX(-50%);
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  white-space: nowrap;
  color: var(--n-danger);
}
</style>
