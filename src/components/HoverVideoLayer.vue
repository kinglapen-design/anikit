<script setup lang="ts">
// 视频卡片悬停播放层（5.3，2026-07-27 用户定案，参考图 1）：无悬停按钮，悬停即自动播放。
// 预览范围 = 封面帧（thumbFrame `t:{秒}`，未设置 = 0）起 10 秒封顶时长，循环播放；
// 跟随鼠标横向移动快进快退（scrub 到范围内对应位置）；右下胶囊 = 当前时间 + 音量钮（默认静音）；
// 底部 3px 进度条显示范围内进度。media 元素组件自管（与 previewMedia 预览单例隔离）。
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import AppIcon from "./AppIcon.vue";
import { formatDuration } from "../lib/formatDuration";
import { registerHoverPlayer } from "../stores/hoverPreview";
import { useUiStore } from "../stores/ui";
import type { PlacementView } from "../stores/content";

const props = defineProps<{ asset: PlacementView }>();
const ui = useUiStore();

// 自动播放预览范围上限（秒，2026-07-27 用户定：最大只支持 10 秒）
const RANGE_SECS = 10;
// 悬停进入延迟（毫秒）：防快速划过触发加载/转码
const ENTER_DELAY = 300;

const layerRef = ref<HTMLElement>();
const videoRef = ref<HTMLVideoElement>();
const ready = ref(false); // canplay：视频淡入（之前保持底层静态缩略图，转码等待无闪烁）
const currentTime = ref(0);
const duration = ref(0);
const muted = ref(true); // 默认静音，音量钮点击切换（图 1"可开音量"）

// 范围起点：封面帧秒数；源已变短于封面帧时回退 0
function rangeStart(): number {
  const tf = props.asset.thumbFrame;
  let s = tf?.startsWith("t:") ? Number(tf.slice(2)) || 0 : 0;
  const d = duration.value || props.asset.duration || 0;
  if (d > 0 && s >= d) s = 0;
  return s;
}
function rangeEnd(): number {
  const d = duration.value || props.asset.duration || 0;
  return d > 0 ? Math.min(rangeStart() + RANGE_SECS, d) : rangeStart() + RANGE_SECS;
}
const pct = computed(() => {
  const span = rangeEnd() - rangeStart();
  return span > 0 ? Math.min(100, Math.max(0, ((currentTime.value - rangeStart()) / span) * 100)) : 0;
});

let timer: ReturnType<typeof setTimeout> | undefined;
let disposed = false;
let unregister: (() => void) | undefined;

onMounted(() => {
  unregister = registerHoverPlayer(() => videoRef.value?.pause());
  timer = setTimeout(() => void loadSource(), ENTER_DELAY);
});

async function loadSource(): Promise<void> {
  try {
    const r = await invoke<{ path: string; transcoded: boolean }>("get_media_source", {
      assetId: props.asset.assetId,
    });
    if (disposed) return;
    const v = videoRef.value;
    if (!v) return;
    v.src = convertFileSrc(r.path);
    v.currentTime = rangeStart();
    void v.play().catch((e) => console.warn("悬停播放失败：", e));
  } catch (e) {
    console.warn("悬停播放源解析失败：", e);
  }
}

function onTimeUpdate(): void {
  const v = videoRef.value;
  if (!v) return;
  // 范围循环：到终点跳回起点（不足 10 秒的视频 end = 片尾，自然整段循环）
  if (v.currentTime >= rangeEnd()) v.currentTime = rangeStart();
  currentTime.value = v.currentTime;
}
function onLoadedMetadata(): void {
  duration.value = videoRef.value?.duration || 0;
}
function onEnded(): void {
  const v = videoRef.value;
  if (!v) return;
  v.currentTime = rangeStart();
  void v.play().catch(() => {});
}
// 跟随鼠标快进快退：横向位置映射到 [start, end]
function onScrub(e: PointerEvent): void {
  const v = videoRef.value;
  if (!v || !ready.value) return;
  const rect = layerRef.value!.getBoundingClientRect();
  const ratio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
  v.currentTime = rangeStart() + ratio * (rangeEnd() - rangeStart());
}
function toggleMute(): void {
  const v = videoRef.value;
  if (!v) return;
  v.muted = !v.muted;
  muted.value = v.muted;
}

onBeforeUnmount(() => {
  disposed = true;
  clearTimeout(timer);
  unregister?.();
  const v = videoRef.value;
  if (v) {
    v.pause();
    v.removeAttribute("src");
    v.load();
  }
});
</script>

<template>
  <div ref="layerRef" class="hover-video" @pointermove="onScrub">
    <video
      ref="videoRef"
      class="v"
      :class="{ ready, contain: ui.layoutMode === 'grid' }"
      :muted="muted"
      preload="auto"
      playsinline
      @canplay="ready = true"
      @timeupdate="onTimeUpdate"
      @loadedmetadata="onLoadedMetadata"
      @ended="onEnded"
    />
    <!-- 底部控件区：不触发 scrub -->
    <div class="hud" @pointermove.stop>
      <div class="capsule">
        <span class="time">{{ formatDuration(currentTime) }}</span>
        <button class="hud-btn" v-tip="muted ? '取消静音' : '静音'" @click.stop="toggleMute">
          <AppIcon :name="muted ? 'mute' : 'volume'" />
        </button>
      </div>
      <div class="progress"><div class="fill" :style="{ width: `${pct}%` }" /></div>
    </div>
  </div>
</template>

<style scoped>
.hover-video {
  position: absolute;
  inset: 0;
}
.v {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: cover;
  opacity: 0;
  transition: opacity 0.15s ease;
}
.v.contain {
  object-fit: contain;
}
.v.ready {
  opacity: 1;
}
/* 控件区贴底部：进度条在最下沿横贯，胶囊在其上方右下（图 1） */
.hud {
  position: absolute;
  left: 0;
  right: 0;
  bottom: 0;
}
.capsule {
  position: absolute;
  right: 4px;
  bottom: 7px; /* 进度条 3px + 间距 4 */
  display: flex;
  align-items: center;
  gap: var(--space-4);
  padding: 2px 6px;
  border-radius: var(--radius-4);
  background: var(--c1-40);
}
.time {
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--font-icon-1);
  white-space: nowrap;
}
.hud-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  color: var(--font-icon-1);
}
.hud-btn :deep(svg) {
  width: 12px;
  height: 12px;
}
/* 进度条：3px 轨道 border-2（暗，压图不抢眼）+ fill 品牌紫（与 5.2 播放条同口径） */
.progress {
  height: 3px;
  background: var(--border-2);
}
.fill {
  height: 100%;
  background: var(--brand-1);
}
</style>
