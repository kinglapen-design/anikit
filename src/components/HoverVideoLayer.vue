<script setup lang="ts">
// 视频卡片播放层（5.3，2026-07-27 用户定案，参考图 1），双模式：
// - 悬停模式（auto=false）：完整播放整段视频（循环），跟随鼠标横向移动快进快退——
//   scrub 走**预生成预览帧切图**（Eagle 同方案，list_video_scrub_frames 全片均匀 40 帧，
//   鼠标移动只换 img 零解码 seek，2026-07-28 用户实测 video seek 卡顿后改）；
//   帧未就绪时退化为 video seek（rAF 合帧 + 66ms 门控）兜底；鼠标停 300ms 后 video 从该位置续播；
//   右下胶囊 = 当前时间 + 音量钮（默认静音），底部 3px 进度条显示全片进度；
// - 自动播放模式（auto=true，查看方式菜单"自动播放视频"开启且未悬停）：无控件、恒静音、不 scrub，
//   范围 = 封面帧（thumbFrame `t:{秒}`，未设置 = 0）起 10 秒封顶时长循环——
//   "设置当前画面为预览图"后自动播放范围自动跟随该帧（无需新设置 UI）。
// 悬停接管 / 移出在同一组件内切换（media 元素不重建）；media 元素组件自管（与 previewMedia 隔离）。
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import AppIcon from "./AppIcon.vue";
import { formatDuration } from "../lib/formatDuration";
import { registerHoverPlayer } from "../stores/hoverPreview";
import { useUiStore } from "../stores/ui";
import type { PlacementView } from "../stores/content";

const props = withDefaults(defineProps<{ asset: PlacementView; auto?: boolean }>(), { auto: false });
const ui = useUiStore();

// 自动播放预览范围上限（秒，2026-07-27 用户定：自动播放最大只支持 10 秒）
const RANGE_SECS = 10;
// 进入延迟（毫秒）：防快速划过 / 快速滚动触发加载与转码
const ENTER_DELAY = 300;
// 鼠标停止移动判定（毫秒）：超过即结束 scrub，video 从该位置续播
const SCRUB_END_DELAY = 300;

const layerRef = ref<HTMLElement>();
const videoRef = ref<HTMLVideoElement>();
const ready = ref(false); // canplay：视频淡入（之前保持底层静态缩略图，转码等待无闪烁）
const currentTime = ref(0);
const duration = ref(0);
const muted = ref(true); // 默认静音；自动播放模式恒静音（多视频同播），悬停可点音量钮开
// scrub 预览帧（Eagle 帧图方案）：null = 未就绪 / 不可用（维持 video seek 兜底）
const scrubFrames = ref<string[] | null>(null);
const scrubbing = ref(false); // 帧图 scrub 中（帧图层显示、video 暂停）
const scrubIdx = ref(0);

// 范围起点：自动播放 = 封面帧秒数（源已变短于封面帧时回退 0）；悬停 = 0（完整播放）
function rangeStart(): number {
  if (!props.auto) return 0;
  const tf = props.asset.thumbFrame;
  let s = tf?.startsWith("t:") ? Number(tf.slice(2)) || 0 : 0;
  const d = duration.value || props.asset.duration || 0;
  if (d > 0 && s >= d) s = 0;
  return s;
}
// 范围终点：自动播放 = 起点 + 10 秒封顶时长；悬停 = 片尾
function rangeEnd(): number {
  const d = duration.value || props.asset.duration || 0;
  if (!props.auto) return d > 0 ? d : Number.MAX_SAFE_INTEGER;
  return d > 0 ? Math.min(rangeStart() + RANGE_SECS, d) : rangeStart() + RANGE_SECS;
}
const pct = computed(() => {
  const span = rangeEnd() - rangeStart();
  return span > 0 ? Math.min(100, Math.max(0, ((currentTime.value - rangeStart()) / span) * 100)) : 0;
});

let timer: ReturnType<typeof setTimeout> | undefined;
let scrubEndTimer: ReturnType<typeof setTimeout> | undefined;
let disposed = false;
let unregister: (() => void) | undefined;

onMounted(() => {
  unregister = registerHoverPlayer(() => videoRef.value?.pause());
  timer = setTimeout(() => {
    void loadSource();
    if (!props.auto) void loadScrubFrames(); // 自动播放模式不 scrub，不触发抽帧（省一次全片扫描）
  }, ENTER_DELAY);
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
    void v.play().catch((e) => console.warn("卡片播放失败：", e));
  } catch (e) {
    console.warn("卡片播放源解析失败：", e);
  }
}

// 加载 scrub 预览帧（后端全片均匀抽 40 帧，会话级缓存；首次 3-10 秒，期间维持 video seek 兜底）
async function loadScrubFrames(): Promise<void> {
  try {
    const r = await invoke<{ frames: string[]; pending: boolean }>("list_video_scrub_frames", {
      assetId: props.asset.assetId,
    });
    if (disposed || r.frames.length < 2) return; // 单帧无 scrub 意义（len-1 插值会 0/0），维持兜底
    scrubFrames.value = r.frames.map((p) => convertFileSrc(p));
  } catch {
    // 组件缺失 / 生成失败：维持 video seek 兜底
  }
}

// 模式切换（悬停接管 / 移出回自动播放）：元素不重建——切回自动播放强制静音，
// 位置超出范围由 timeupdate 自然跳回起点
watch(
  () => props.auto,
  (auto) => {
    const v = videoRef.value;
    if (auto && v) {
      v.muted = true;
      muted.value = true;
    }
  },
);

function onTimeUpdate(): void {
  const v = videoRef.value;
  if (!v) return;
  // 范围循环：到终点跳回起点（悬停模式 = 片尾跳回开头整段循环；不足 10 秒的视频自然整段循环）
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
// 跟随鼠标快进快退（仅悬停模式）：横向位置映射到 [start, end]。
// pointermove 只记目标比例，rAF 合帧应用（img 换 src 也有解码开销，合帧降载）——
// 预览帧就绪 = 帧图模式（切图零解码 seek，跟手）；未就绪 = video seek 兜底（66ms 门控 15fps 上限）
let scrubRatio: number | null = null;
let scrubDirty = false;
let scrubRaf = 0;
let lastSeekAt = 0;
let lastSeekSec = -1;

function onScrub(e: PointerEvent): void {
  if (props.auto) return;
  const rect = layerRef.value!.getBoundingClientRect();
  scrubRatio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
  scrubDirty = true;
  if (!scrubRaf) scrubRaf = requestAnimationFrame(applyScrub);
}
function applyScrub(): void {
  scrubRaf = 0;
  if (scrubRatio == null) {
    scrubDirty = false;
    return;
  }
  const ratio = scrubRatio;
  const frames = scrubFrames.value;
  if (frames && frames.length > 0) {
    // 帧图模式：切图 + video 暂停；鼠标停 SCRUB_END_DELAY 后由 endScrub 续播
    scrubDirty = false;
    const v = videoRef.value;
    // 再次 scrub：取消上次 endScrub 的 seeked 等待（防旧回调误收帧图层）
    if (pendingSeeked) {
      if (v) v.removeEventListener("seeked", pendingSeeked);
      pendingSeeked = null;
    }
    const idx = Math.round(ratio * (frames.length - 1));
    scrubIdx.value = idx;
    scrubbing.value = true;
    if (v && !v.paused) v.pause();
    const d = duration.value || props.asset.duration || 0;
    currentTime.value = (idx / (frames.length - 1)) * d;
    lastSeekSec = currentTime.value;
    clearTimeout(scrubEndTimer);
    scrubEndTimer = setTimeout(endScrub, SCRUB_END_DELAY);
    return;
  }
  // 兜底：video seek（预览帧未就绪）
  const v = videoRef.value;
  if (!v || !ready.value) {
    scrubDirty = false;
    return;
  }
  const now = performance.now();
  if (now - lastSeekAt < 66) {
    if (scrubDirty) scrubRaf = requestAnimationFrame(applyScrub); // 门控等待，下帧再试
    return;
  }
  const target = rangeStart() + ratio * (rangeEnd() - rangeStart());
  scrubDirty = false;
  if (Math.abs(target - lastSeekSec) < 0.05) return;
  lastSeekAt = now;
  lastSeekSec = target;
  v.currentTime = target;
}
// 鼠标停 300ms：video seek 到 scrub 位置——帧图层保持到 seek 到位（seeked）再收起续播，
// 防"帧图 → scrub 前旧画面 → 新画面"两次跳变（2026-07-28 用户实测）；再次 scrub 取消等待
let pendingSeeked: (() => void) | null = null;
function endScrub(): void {
  const v = videoRef.value;
  if (!v || !v.src) {
    scrubbing.value = false;
    return;
  }
  if (pendingSeeked) v.removeEventListener("seeked", pendingSeeked);
  pendingSeeked = () => {
    if (pendingSeeked) v.removeEventListener("seeked", pendingSeeked);
    pendingSeeked = null;
    scrubbing.value = false;
    void v.play().catch(() => {});
  };
  v.addEventListener("seeked", pendingSeeked);
  v.currentTime = lastSeekSec;
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
  clearTimeout(scrubEndTimer);
  cancelAnimationFrame(scrubRaf);
  const v0 = videoRef.value;
  if (v0 && pendingSeeked) v0.removeEventListener("seeked", pendingSeeked);
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
    <!-- scrub 帧图层（Eagle 帧图方案）：鼠标移动时盖住 video 切帧图，停下后收起续播 -->
    <img
      v-if="scrubbing && scrubFrames"
      class="v scrub-img"
      :class="{ contain: ui.layoutMode === 'grid' }"
      :src="scrubFrames[scrubIdx]"
      :alt="asset.name"
      decoding="async"
      draggable="false"
    />
    <!-- 底部控件区（仅悬停模式）：不触发 scrub -->
    <div v-if="!auto" class="hud" @pointermove.stop>
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
/* scrub 帧图层：盖 video（不参与 ready 淡入，直接不透明；DOM 序在 video 后自然在上）；
   C1 底——带 alpha 的 mov 帧图透明区会透出底下 video 画面（同序列帧修复，2026-07-28 用户实测） */
.scrub-img {
  position: absolute;
  inset: 0;
  opacity: 1;
  background: var(--c1);
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
