<script setup lang="ts">
// 视频卡片播放层（5.3，2026-07-27 用户定案，参考图 1），双模式：
// - 悬停模式（auto=false）：完整播放整段视频（循环），跟随鼠标横向移动快进快退
//   （video seek：pointermove 直接设 currentTime——2026-08-22 实机数据：WebView2 单点 seek
//   5-40ms、66ms 间隔连发全部完成，浏览器自身合并跟不上的 seek，无需 rAF 合帧 / 门控；
//   原"rAF 合帧 + 66ms 门控 + 0.05s 死区"（2026-07-27 定稿）作废；
//   预生成帧图方案已于 2026-07-28 用户决策废弃——CPU 长期高负载 / 磁盘占用大 / 载入慢）；
//   右下胶囊 = 当前帧号（5.3 帧化：查看方式"时间显示 = 帧数"且 fps 可用；回退当前时间）+ 音量钮
//   （默认静音），底部 3px 进度条显示全片进度；
// - 自动播放模式（auto=true，查看方式菜单"自动播放视频"开启且未悬停）：无控件、恒静音、不 scrub，
//   范围 = 封面帧（thumbFrame `t:{秒}`，未设置 = 0）起 10 秒封顶时长循环——
//   "设置当前画面为预览图"后自动播放范围自动跟随该帧（无需新设置 UI）。
// 悬停接管 / 移出在同一组件内切换（media 元素不重建）；media 元素组件自管（与 previewMedia 隔离）。
import { computed, ref, watch } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import AppIcon from "./AppIcon.vue";
import { formatDuration } from "../lib/formatDuration";
import { frameOfTime, totalFrames } from "../lib/formatFrame";
import { getMediaSource } from "../lib/mediaOps";
import { useHoverPlayer } from "../lib/useHoverPlayer";
import { useUiStore } from "../stores/ui";
import type { PlacementView } from "../stores/content";

const props = withDefaults(defineProps<{ asset: PlacementView; auto?: boolean }>(), { auto: false });
const ui = useUiStore();

// 自动播放预览范围上限（秒，2026-07-27 用户定：自动播放最大只支持 10 秒）
const RANGE_SECS = 10;

const layerRef = ref<HTMLElement>();
const videoRef = ref<HTMLVideoElement>();
const ready = ref(false); // canplay：视频淡入（之前保持底层静态缩略图，转码等待无闪烁）
const currentTime = ref(0);
const duration = ref(0);
const muted = ref(true); // 默认静音；自动播放模式恒静音（多视频同播），悬停可点音量钮开

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
// 胶囊位置标识（5.3 帧化改造，2026-07-28 用户定：帧数优先）：查看方式"时间显示 = 帧数"且
// 素材 fps 可用 → 当前帧号；fps 缺失或时间码模式回退当前时间（mm:ss）
const posText = computed(() => {
  const fps = props.asset.fps;
  if (ui.timeDisplay === "frames" && fps != null && fps > 0) {
    const total = totalFrames(duration.value || props.asset.duration || 0, fps);
    return String(frameOfTime(currentTime.value, fps, total));
  }
  return formatDuration(currentTime.value);
});

// 悬停播放骨架（300ms 进入延迟 / 活跃实例注册 / disposed 护栏 / 卸载清理）收编 useHoverPlayer；
// 本组件只管 media 元素、scrub 与范围循环
const { isDisposed } = useHoverPlayer({
  load: loadSource,
  pause: () => videoRef.value?.pause(),
  cleanup: () => {
    const v = videoRef.value;
    if (v) {
      v.pause();
      v.removeAttribute("src");
      v.load();
    }
  },
});

async function loadSource(): Promise<void> {
  try {
    const r = await getMediaSource(props.asset.assetId);
    if (isDisposed()) return;
    const v = videoRef.value;
    if (!v) return;
    v.src = convertFileSrc(r.path);
    v.currentTime = rangeStart();
    void v.play().catch((e) => console.warn("卡片播放失败：", e));
  } catch (e) {
    console.warn("卡片播放源解析失败：", e);
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
// 跟随鼠标快进快退（仅悬停模式）：横向位置映射到 [start, end]，pointermove 直接 seek
//（实测每次 seek 5-40ms，无需合帧 / 门控 / 死区；元素未 ready 时不 seek，保持底层缩略图）；
// UI 状态（进度条 / 帧号）同步写目标值立即跟手——2026-08-22 修：原只走 timeupdate（250ms 粒度），肉眼滞后
function onScrub(e: PointerEvent): void {
  if (props.auto) return;
  const v = videoRef.value;
  if (!v || !ready.value) return;
  const rect = layerRef.value!.getBoundingClientRect();
  const ratio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
  v.currentTime = rangeStart() + ratio * (rangeEnd() - rangeStart());
  currentTime.value = v.currentTime;
}
function toggleMute(): void {
  const v = videoRef.value;
  if (!v) return;
  v.muted = !v.muted;
  muted.value = v.muted;
}
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
    <!-- 底部控件区（仅悬停模式）：不触发 scrub -->
    <div v-if="!auto" class="hud" @pointermove.stop>
      <div class="capsule">
        <span class="time">{{ posText }}</span>
        <button class="hud-btn" v-tip="muted ? '取消静音' : '静音'" @click.stop="toggleMute">
          <AppIcon :name="muted ? 'mute' : 'volume'" :size="12" />
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
/* 胶囊共享样式在 base.css（.capsule / .time / .hud-btn）；本地差异：bottom 7 给底部 3px
   进度条让位（进度条 3 + 间距 4） */
.capsule {
  bottom: 7px;
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
