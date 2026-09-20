<script setup lang="ts">
// 预览画面区（4.3/5.2）：image/sequence 走 <img>（filePath 由 store 维护，切帧即换 src）；
// anim = 阶段四帧流（PreviewStreamCanvas 渲染 canvas，占位缩略图 / 回退 img 在下层）；
// video 把 previewMedia 的 <video> 元素挂进画面区（缩放/平移/旋转/翻转复用图片那套数学，
// 尺寸经 loadedmetadata → onImageLoad 喂入）；audio 显示波形大图（按画面区实际像素后端重采样，
// 未就绪回退 info 波形缩略图 contain），无缩放平移；
// font / text（5.6）= PreviewFont / PreviewText 组件自管（无缩放平移）；
// other（不支持预览态，R4 按稿 2026-09-21 落地：布局-中间面板-内容容器-此文件不支持预览）=
// 居中立姿——格式图标 200 + 文件名（标题档二级）+ 提示「暂不支持此文件格式预览，请用其他应用打开」
// （正文三级）+ 按钮组（ActionButton solid md：默认应用打开 / 其他应用打开；
// ws 合成项按钮组隐藏——assetId 假 id 禁发命令，同 PreviewTitleBar isWsCurrent 口径）。
// 交互：滚轮缩放（瞄点跟鼠标）、左键拖动平移、双击 适合窗口⇄原图大小；
// ←/→ 上/下一个素材；,/ . sequence/anim=上/下帧 video=逐帧步进；
// 空格 sequence/anim/audio/video=播放暂停；Esc 退出全屏/预览。
import { computed, onBeforeUnmount, onMounted, ref, watch, watchEffect } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import PreviewNavigator from "./PreviewNavigator.vue";
import PreviewStreamCanvas from "./PreviewStreamCanvas.vue";
import PreviewFont from "./PreviewFont.vue";
import PreviewText from "./PreviewText.vue";
import ActionButton from "./buttons/ActionButton.vue";
import { useHiresWaveform } from "../lib/useHiresWaveform";
import { openWithDialog } from "../lib/placementOps";
import { usePreviewStore } from "../stores/preview";
import { usePreviewMediaStore } from "../stores/previewMedia";

const preview = usePreviewStore();
const media = usePreviewMediaStore();
const stageRef = ref<HTMLElement>();
const videoWrapRef = ref<HTMLElement>();
let ro: ResizeObserver | undefined;

const url = computed(() => (preview.filePath ? convertFileSrc(preview.filePath) : null));
// <img> 渲染的 kind（video 走元素挂载，audio 走波形大图，font/text 走各自组件）
const showImg = computed(
  () =>
    url.value &&
    preview.kind !== "video" &&
    preview.kind !== "audio" &&
    preview.kind !== "font" &&
    preview.kind !== "text" &&
    preview.kind !== "other",
);
// other 不支持预览态：格式图标（current.thumbPath，200 实寸稿值）
const iconUrl = computed(() => {
  const p = preview.current;
  return preview.kind === "other" && p?.thumbPath ? convertFileSrc(p.thumbPath) : null;
});
// ws 合成假 id 禁发 assetId 类命令（PRD 02 步骤 1），打开按钮组隐藏——同 PreviewTitleBar isWsCurrent 口径
const isWsCurrent = computed(() => !!preview.current?.assetId.startsWith("ws:"));
// 其他应用打开 = 系统"打开方式"对话框（失败 toast 在 openWithDialog 内）；默认应用打开直走 store
function onOpenOther(): void {
  const p = preview.current;
  if (!p || isWsCurrent.value) return;
  void openWithDialog(p.assetId);
}
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
    <PreviewFont v-else-if="preview.kind === 'font'" />
    <PreviewText v-else-if="preview.kind === 'text'" />
    <div v-else-if="waveUrl" class="wave-large" :style="{ '--wave-url': `url(&quot;${waveUrl}&quot;)` }" />
    <div v-else-if="preview.kind === 'other'" class="unsupported">
      <div class="us-body">
        <img v-if="iconUrl" class="us-icon" :src="iconUrl" draggable="false" />
        <div class="us-texts">
          <span class="us-title">{{ preview.current?.name ?? "" }}</span>
          <span class="us-hint">暂不支持此文件格式预览，请用其他应用打开</span>
        </div>
        <div v-if="!isWsCurrent" class="us-actions">
          <ActionButton label="默认应用打开" icon="open-default" @click="preview.openDefaultApp()" />
          <ActionButton label="其他应用打开" icon="open-in-other-app" @click="onOpenOther" />
        </div>
      </div>
    </div>
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
   颜色 = 静态波形实色 --n-l3（原 --wave-pending 就近；2026-08-10 实色化，font-icon-3 观感作废），
   mask-size contain 居中即原 object-fit 行为） */
.wave-large {
  position: absolute;
  inset: 0;
  background: var(--n-l3);
  -webkit-mask-image: var(--wave-url);
  mask-image: var(--wave-url);
  -webkit-mask-size: contain;
  mask-size: contain;
  -webkit-mask-position: center;
  mask-position: center;
  -webkit-mask-repeat: no-repeat;
  mask-repeat: no-repeat;
}
/* other 不支持预览态（稿：布局-中间面板-内容容器-此文件不支持预览）：
   根层铺满画面区内容居中；块 = column 居中 gap 24 + padding-bottom 24（稿「图标+提示+按钮」） */
.unsupported {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}
.us-body {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--n-space-24);
  padding-bottom: var(--n-space-24);
}
/* shell-icon：200×200 稿值（格式图标方形缩略图 contain） */
.us-icon {
  width: 200px;
  height: 200px;
  object-fit: contain;
}
/* 标题+提示文字：column 居中 gap 8（稿） */
.us-texts {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--n-space-8);
}
/* 文件名 = 标题档 18/26 Bold 二级；超长截断稿未画——同 PreviewTitleBar .index-name 口径
   nowrap 不截断（stage overflow hidden 兜底） */
.us-title {
  font-size: var(--n-fs-title);
  line-height: var(--n-lh-title);
  font-weight: var(--n-fw-bold);
  color: var(--n-l2);
  white-space: nowrap;
}
.us-hint {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l3);
}
/* 按钮组：稿值两钮各 150 + gap 8 = 308——ActionButton md/lg 宽 100% 由父容器定格，
   grid 双列均分贴稿（同 EmptyFolderState 居中根因口径，速查 §22） */
.us-actions {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--n-space-8);
  width: 308px;
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
