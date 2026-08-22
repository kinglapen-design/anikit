<script setup lang="ts">
// 视频卡片播放层（5.3 全面接入自研解码器，替代旧 <video> 元素的 HoverVideoLayer）：双模式——
// - 悬停模式（auto=false）：完整播放整段视频（循环，后端 loop），跟随鼠标横向移动快进快退
//   （seekVideo 秒定位：pointermove 只记目标比例，rAF 合帧 + 66ms 门控应用，沿用 2026-07-27 定稿参数）；
//   右下胶囊 = 当前帧号（查看方式"时间显示 = 帧数"且 fps 可用；回退当前时间 mm:ss）+ 音量钮
//   （默认静音，有音轨才出——GainNode 纯前端），底部 3px 进度条显示全片进度；
// - 自动播放模式（auto=true，查看方式菜单"自动播放视频"开启且未悬停）：无控件、恒静音、不 scrub，
//   范围 = 封面帧（thumbFrame `t:{秒}`，未设置 = 0）起 10 秒封顶时长循环（帧 pts 越界即 seekVideo 回起点，
//   前端驱动；"设置当前画面为预览图"后范围自动跟随该帧）。
// 播放源 = WS 帧流 canvas（后端 ffmpeg-next 对 origin 运行时解码 + 音轨 PCM 推送），转码链已退役；
// 会话失败不出帧，下层静态缩略图自然兜底。出帧尺寸跟随卡片短边（同 AnimCanvasLayer 机制）；
// 骨架（300ms 进入延迟 / 主窗口隐藏暂停 / disposed 护栏 / 卸载清理）收编 useHoverPlayer。
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import AppIcon from "./AppIcon.vue";
import { formatDuration } from "../lib/formatDuration";
import { frameOfTime, totalFrames } from "../lib/formatFrame";
import { StreamSession, type StreamFrame } from "../lib/streamClient";
import { useHoverPlayer } from "../lib/useHoverPlayer";
import { useUiStore } from "../stores/ui";
import type { PlacementView } from "../stores/content";

const props = withDefaults(defineProps<{ asset: PlacementView; auto?: boolean }>(), { auto: false });
const ui = useUiStore();

// 自动播放预览范围上限（秒，2026-07-27 用户定：自动播放最大只支持 10 秒）
const RANGE_SECS = 10;
// 出帧短边口径与后端 / AnimCanvasLayer 一致（clamp [16,1024]；布局未就绪兜底 320）
const MIN_EDGE = 16;
const MAX_EDGE = 1024;
const FALLBACK_EDGE = 320;
const RESIZE_DEBOUNCE = 150;

const layerRef = ref<HTMLElement>();
const cv = ref<HTMLCanvasElement>();
const hasFrame = ref(false);
const curSecs = ref(0);
const muted = ref(true); // 默认静音；自动播放模式恒静音（多视频同播），悬停可点音量钮开
const hasAudio = ref(false);

let session: StreamSession | null = null;
let resizeObs: ResizeObserver | null = null;
let resizeTimer = 0;
let lastRangeSeek = 0; // 范围循环 seek 冷却标记（防在途/失败洪泛）

// 素材时长（秒）：会话响应优先（元数据准确值），PlacementView 快照兜底
const duration = computed(() => session?.durationSec ?? props.asset.duration ?? 0);
// 素材帧率：会话响应优先（自愈后的准确值），PlacementView 快照兜底
const fps = computed(() => session?.fps ?? props.asset.fps ?? null);

// 范围起点：自动播放 = 封面帧秒数（源已变短于封面帧时回退 0）；悬停 = 0（完整播放）
function rangeStart(): number {
  if (!props.auto) return 0;
  const tf = props.asset.thumbFrame;
  let s = tf?.startsWith("t:") ? Number(tf.slice(2)) || 0 : 0;
  const d = duration.value;
  if (d > 0 && s >= d) s = 0;
  return s;
}
// 范围终点：自动播放 = 起点 + 10 秒封顶时长；悬停 = 片尾
function rangeEnd(): number {
  const d = duration.value;
  if (!props.auto) return d > 0 ? d : Number.MAX_SAFE_INTEGER;
  return d > 0 ? Math.min(rangeStart() + RANGE_SECS, d) : rangeStart() + RANGE_SECS;
}
const pct = computed(() => {
  const span = rangeEnd() - rangeStart();
  return span > 0 ? Math.min(100, Math.max(0, ((curSecs.value - rangeStart()) / span) * 100)) : 0;
});
// 胶囊位置标识（帧数优先，2026-07-28 用户定）：帧数模式且 fps 可用 → 当前帧号；否则当前时间
const posText = computed(() => {
  const f = fps.value;
  if (ui.timeDisplay === "frames" && f != null && f > 0) {
    return String(frameOfTime(curSecs.value, f, totalFrames(duration.value, f)));
  }
  return formatDuration(curSecs.value);
});

// 悬停播放骨架（300ms 进入延迟 / 活跃实例注册 / disposed 护栏 / 卸载清理）收编 useHoverPlayer；
// 本组件只管帧流会话、scrub 与范围循环
const { isDisposed } = useHoverPlayer({
  load: start,
  pause: stopSession, // 主窗口隐藏：停会话省 CPU（对齐"隐藏停止"语义，重悬停重建）
  cleanup: () => {
    cancelAnimationFrame(scrubRaf);
    stopSession();
  },
});

// 卡片显示短边：canvas 未出帧时读不到尺寸，读父元素 .thumb（inset-0 同尺寸）
function shortEdge(): number {
  const r = cv.value?.parentElement?.getBoundingClientRect();
  const e = r ? Math.round(Math.min(r.width, r.height)) : 0;
  return e > 0 ? Math.min(MAX_EDGE, Math.max(MIN_EDGE, e)) : FALLBACK_EDGE;
}

async function start(): Promise<void> {
  const s = await StreamSession.startVideo(
    props.asset.assetId,
    { shortEdge: shortEdge(), startSecs: rangeStart(), loop: true }, // 卡片恒循环（范围循环前端驱动）
    {
      onFrame: draw,
      onControl: (c) => {
        if (c === "error") stopSession(); // 会话死亡：降级静态封面（canvas 隐藏，下层缩略图自然兜底）
      },
    },
  );
  if (isDisposed()) {
    s?.stop();
    return;
  }
  session = s;
  hasAudio.value = s?.hasAudio ?? false;
  session?.setMuted(true); // 恒静音起；悬停音量钮解开
}

function draw(f: StreamFrame): void {
  const c = cv.value;
  if (!c) return;
  if (c.width !== f.w || c.height !== f.h) {
    c.width = f.w;
    c.height = f.h;
  }
  c.getContext("2d")?.putImageData(new ImageData(f.rgba, f.w, f.h), 0, 0);
  hasFrame.value = true;
  curSecs.value = f.ptsMs / 1000;
  // 自动播放范围循环：越终点回起点（自然到底回绕 0 也被 pts 回退兜住——片长短于范围时）。
  // 冷却 300ms：seek 在途 / 失败时防逐帧 invoke 洪泛
  if (
    props.auto &&
    (f.ptsMs / 1000 >= rangeEnd() || f.ptsMs / 1000 < rangeStart() - 0.5) &&
    performance.now() - lastRangeSeek > 300
  ) {
    lastRangeSeek = performance.now();
    session?.seekVideo(rangeStart());
  }
}

function stopSession(): void {
  session?.stop();
  session = null;
  hasFrame.value = false;
  curSecs.value = 0;
  lastRangeSeek = 0;
}

// 模式切换（悬停接管 / 移出回自动播放）：会话不重建——切回自动播放强制静音，
// 位置超出范围由 draw 内范围循环自然跳回起点
watch(
  () => props.auto,
  (auto) => {
    if (auto) {
      muted.value = true;
      session?.setMuted(true);
    }
  },
);

// 跟随鼠标快进快退（仅悬停模式）：横向位置映射到 [start, end]。
// pointermove 只记目标比例，rAF 合帧应用（2026-07-27 用户实测：每次 move 直接 seek 卡顿不跟手）——
// seek 上限 15fps（66ms 门控，解码器喘得过气）；目标与上次差 <0.05s 不 seek（手停微抖不触发）
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
  if (scrubRatio == null || !hasFrame.value || !session) {
    scrubDirty = false;
    return;
  }
  const now = performance.now();
  if (now - lastSeekAt < 66) {
    if (scrubDirty) scrubRaf = requestAnimationFrame(applyScrub); // 门控等待，下帧再试
    return;
  }
  const target = rangeStart() + scrubRatio * (rangeEnd() - rangeStart());
  scrubDirty = false;
  if (Math.abs(target - lastSeekSec) < 0.05) return;
  lastSeekAt = now;
  lastSeekSec = target;
  session.seekVideo(target);
}
function toggleMute(): void {
  muted.value = !muted.value;
  session?.setMuted(muted.value);
}

onMounted(() => {
  // 出帧尺寸跟随卡片显示短边（缩放布局；防抖 150ms setSize 热更新，同 AnimCanvasLayer）
  const box = cv.value?.parentElement;
  if (!box) return;
  resizeObs = new ResizeObserver(() => {
    window.clearTimeout(resizeTimer);
    resizeTimer = window.setTimeout(() => session?.setSize(shortEdge()), RESIZE_DEBOUNCE);
  });
  resizeObs.observe(box);
});

onBeforeUnmount(() => {
  resizeObs?.disconnect();
  window.clearTimeout(resizeTimer);
});
</script>

<template>
  <div ref="layerRef" class="hover-video" @pointermove="onScrub">
    <!-- hasFrame 前 canvas 透明（下层静态封面可见）；铺设口径同 .img（grid contain） -->
    <canvas v-show="hasFrame" ref="cv" class="v" :class="{ contain: ui.layoutMode === 'grid' }" />
    <!-- 底部控件区（仅悬停模式）：不触发 scrub -->
    <div v-if="!auto" class="hud" @pointermove.stop>
      <div class="capsule">
        <span class="time">{{ posText }}</span>
        <button v-if="hasAudio" class="hud-btn" v-tip="muted ? '取消静音' : '静音'" @click.stop="toggleMute">
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
}
.v.contain {
  object-fit: contain;
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
