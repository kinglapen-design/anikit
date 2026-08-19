<script setup lang="ts">
// 动图卡片播放层（阶段四·自研解码器）：WS 帧流渲染 canvas——替代 2.3 小动画缓存
// （anim_preview 已退役）。后端解码线程按源帧 delay 节奏推 320p RGBA，putImageData 直渲；
// 会话失败（非动图 / 上限 / 解码失败）不出帧，下层静态封面自然兜底。
// 骨架（300ms 进入延迟 / 主窗口隐藏暂停 / disposed 护栏）收编 useHoverPlayer。
import { ref } from "vue";
import { useHoverPlayer } from "../lib/useHoverPlayer";
import { AnimStreamSession, type AnimFrame } from "../lib/animStream";
import { useUiStore } from "../stores/ui";
import type { PlacementView } from "../stores/content";

const props = defineProps<{ asset: PlacementView }>();
const ui = useUiStore();

const cv = ref<HTMLCanvasElement>();
const hasFrame = ref(false);
let session: AnimStreamSession | null = null;

const { isDisposed } = useHoverPlayer({
  load: start,
  pause: stopSession, // 主窗口隐藏：停会话省 CPU（对齐"隐藏停止"语义，重悬停重建）
  cleanup: stopSession,
});

async function start(): Promise<void> {
  const s = await AnimStreamSession.start(props.asset.assetId, draw);
  if (isDisposed()) {
    s?.stop();
    return;
  }
  session = s;
}

function draw(f: AnimFrame): void {
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
}
.anim-canvas.contain {
  object-fit: contain;
}
</style>
