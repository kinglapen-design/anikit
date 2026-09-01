<script setup lang="ts">
// 动图卡片播放层（阶段四·自研解码器）：WS 帧流渲染 canvas——替代 2.3 小动画缓存
// （anim_preview 已退役）。后端解码线程按源帧 delay 节奏推 RGBA，putImageData 直渲；
// 会话失败（非动图 / 上限 / 解码失败）不出帧，下层静态封面自然兜底。
// 出帧尺寸跟随卡片显示短边（2026-08-20 用户定：缩小省资源、放大跟清晰）——
// 开流时按实际尺寸起，ResizeObserver 监听 .thumb 尺寸变化（缩放布局）防抖 150ms setSize 热更新。
// 骨架（进入延迟 / 主窗口隐藏暂停 / disposed 护栏）收编 useHoverPlayer。
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import { AUTOPLAY_ENTER_DELAY, HOVER_ENTER_DELAY, useHoverPlayer } from "../lib/useHoverPlayer";
import { StreamSession, type StreamFrame } from "../lib/streamClient";
import { useUiStore } from "../stores/ui";
import type { PlacementView } from "../stores/content";

// auto = 自动播放触发（视口可见性驱动）：进入延迟走 100ms 短档（2026-08-23 滚回恢复提速）；
// 缺省 false = 悬停触发 300ms 防划过
const props = withDefaults(defineProps<{ asset: PlacementView; auto?: boolean }>(), { auto: false });
const ui = useUiStore();

// 目标短边口径与后端一致（clamp [16,1024]）；布局未就绪兜底 320（缩略图档）
const MIN_EDGE = 16;
const MAX_EDGE = 1024;
const FALLBACK_EDGE = 320;
const RESIZE_DEBOUNCE = 150;

const cv = ref<HTMLCanvasElement>();
const hasFrame = ref(false);
let session: StreamSession | null = null;
let resizeObs: ResizeObserver | null = null;
let resizeTimer = 0;

const { isDisposed } = useHoverPlayer({
  load: start,
  pause: stopSession, // 主窗口隐藏：停会话省 CPU（对齐"隐藏停止"语义）
  resume: () => {
    if (!session) void start(); // 重新显示：会话已被 pause 停掉则重建（组件仍在 = 悬停/自动播放态存活）
  },
  cleanup: stopSession,
  enterDelay: props.auto ? AUTOPLAY_ENTER_DELAY : HOVER_ENTER_DELAY,
});

// 卡片显示短边：canvas 未出帧时 display:none 读不到尺寸，读父元素 .thumb（inset-0 同尺寸）
function shortEdge(): number {
  const r = cv.value?.parentElement?.getBoundingClientRect();
  const e = r ? Math.round(Math.min(r.width, r.height)) : 0;
  return e > 0 ? Math.min(MAX_EDGE, Math.max(MIN_EDGE, e)) : FALLBACK_EDGE;
}

async function start(): Promise<void> {
  // 播放源代理（8.1）：fla 等工程卡片的伴生 WebP 动图，帧流开流目标是 hidden 资产而非自身
  const s = await StreamSession.start(props.asset.playAssetId ?? props.asset.assetId, shortEdge(), draw);
  if (isDisposed()) {
    s?.stop();
    return;
  }
  session = s;
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
}

function stopSession(): void {
  session?.stop();
  session = null;
}

// 播放源热切换（2026-09-02 用户实机报：换绑伴生缩略图后卡片仍播旧动画——start 只读一次
// playAssetId，常驻组件（自动播放）滞留旧会话盖住下层已刷新的静态封面）：源 id 变 = 停旧流重开，
// hasFrame 回落让下层新封面先露出直到新流出帧
watch(
  () => props.asset.playAssetId ?? props.asset.assetId,
  (id, old) => {
    if (!old || id === old) return;
    stopSession();
    hasFrame.value = false;
    void start();
  },
);

onMounted(() => {
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
  <!-- hasFrame 前 canvas 透明（下层静态封面可见）；铺设口径同 .img（grid contain） -->
  <canvas v-show="hasFrame" ref="cv" class="anim-canvas" :class="{ contain: ui.layoutMode === 'grid' }" />
</template>

<style scoped>
.anim-canvas {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
  /* 播放中衬卡片底色（g2）：帧透明像素不透下层静态首帧图——大透明区动图（CEP 伴生 WebP）
     首曝"第一帧永远在后面"（2026-09-01）；GIF 等不透明帧像素全覆盖无视觉变化 */
  background: var(--n-g2);
}
.anim-canvas.contain {
  object-fit: contain;
}
</style>
