<script setup lang="ts">
// 内容区素材网格：自适应布局 + 自定义 transform 虚拟滚动（踩坑 005 定案）。
// 卡片列表共用一个滚动（03_UI 布局/03_中间面板 03）：文件夹区块与"内容"副标题
// 作为 canvas 头部随网格一起滚动，网格 boxes 按头部高度偏移；
// 滚动条贴 viewport 右缘（不占布局），中间面板 4px 内边距作呼吸（token 15）。
import { computed, onBeforeUnmount, onMounted, ref, shallowRef, watch } from "vue";
import AssetCard from "./AssetCard.vue";
import FolderStrip from "./FolderStrip.vue";
import SectionHeader from "./SectionHeader.vue";
import { CARD_CAPTION_H, computeJustified, lowerBound, type Box, type Row } from "../lib/justified";
import { useVirtualScroll } from "../lib/useVirtualScroll";
import { useContentStore } from "../stores/content";
import { useUiStore } from "../stores/ui";
import type { FakeAsset } from "../stores/content";

const props = defineProps<{ items: FakeAsset[] }>();
const ui = useUiStore();
const content = useContentStore();

const BUFFER = 800;

const viewportRef = ref<HTMLElement>();
const trackRef = ref<HTMLElement>();
const thumbRef = ref<HTMLElement>();
const headerRef = ref<HTMLElement>();

const boxes = shallowRef<Box[]>([]);
const rows = shallowRef<Row[]>([]);
const r0 = ref(0);
const r1 = ref(0);
let totalHeight = 0;

function updateRange(scrollPos: number): void {
  const vh = viewportRef.value?.clientHeight ?? 0;
  const rs = rows.value;
  r0.value = Math.max(0, lowerBound(rs, scrollPos - BUFFER) - 1);
  r1.value = Math.min(rs.length, lowerBound(rs, scrollPos + vh + BUFFER) + 1);
}

const { pos, thumbTop, thumbHeight, setTotal, attach, detach } = useVirtualScroll(updateRange);

const visibleCards = computed(() => {
  const rs = rows.value;
  const bs = boxes.value;
  const out: { asset: FakeAsset; box: Box }[] = [];
  if (rs.length === 0) return out;
  const from = Math.min(r0.value, rs.length - 1);
  const to = Math.min(r1.value, rs.length);
  for (let r = from; r < to; r++) {
    for (let i = rs[r].start; i < rs[r].end; i++) {
      out.push({ asset: props.items[i], box: bs[i] });
    }
  }
  return out;
});

function relayout(): void {
  const vp = viewportRef.value;
  if (!vp || vp.clientWidth <= 0) return;
  ui.gridWidth = vp.clientWidth;
  // 滑块 max = 内容区宽（第一张卡片刚好撑满容器）；min 80
  const maxH = Math.max(80, vp.clientWidth);
  if (ui.targetRowHeight > maxH) ui.targetRowHeight = maxH;
  // 网格按头部（文件夹区块 + 内容副标题）高度 + 区块间距 6 偏移
  const offsetTop = (headerRef.value?.clientHeight ?? 0) + 6;
  const t0 = performance.now();
  const result = computeJustified(props.items, vp.clientWidth, ui.targetRowHeight, CARD_CAPTION_H, offsetTop);
  boxes.value = result.boxes;
  rows.value = result.rows;
  totalHeight = result.totalHeight;
  layoutWidth.value = vp.clientWidth;
  canvasScale.value = 1;
  setTotal(totalHeight);
  console.debug(`[AssetGrid] 布局 ${props.items.length} 条：${(performance.now() - t0).toFixed(1)}ms`);
}

// ctrl + 滚轮缩放卡片（与滑块联动）
function onCtrlWheel(e: WheelEvent): void {
  if (!e.ctrlKey) return;
  e.preventDefault();
  const max = Math.max(80, viewportRef.value?.clientWidth ?? 800);
  const next = ui.targetRowHeight + (e.deltaY > 0 ? -10 : 10);
  ui.targetRowHeight = Math.max(80, Math.min(max, next));
}

// 拖动窗口 / 拆分条期间：canvas 按宽度差等比缩放（卡片实时跟随，GPU 合成零成本），
// 宽度差超过 12% 立即重排，否则停顿 240ms 后精确重排（Eagle 同款交互）。
const layoutWidth = ref(0);
const canvasScale = ref(1);
const SCALE_THRESHOLD = 0.12;

let lastW = 0;
let lastH = 0;
let resizeTimer: ReturnType<typeof setTimeout> | undefined;
const ro = new ResizeObserver(() => {
  const vp = viewportRef.value;
  if (!vp) return;
  const w = vp.clientWidth;
  const h = vp.clientHeight;
  if (w === lastW && h === lastH) return;
  const widthChanged = w !== lastW;
  lastW = w;
  lastH = h;
  if (!widthChanged) {
    setTotal(totalHeight);
    return;
  }
  if (layoutWidth.value > 0) {
    canvasScale.value = w / layoutWidth.value;
  }
  clearTimeout(resizeTimer);
  if (Math.abs(canvasScale.value - 1) > SCALE_THRESHOLD) {
    relayout();
  } else {
    resizeTimer = setTimeout(relayout, 240);
  }
});

watch(
  () => props.items,
  () => relayout(),
);
watch(
  () => ui.targetRowHeight,
  () => relayout(),
);

onMounted(() => {
  attach(viewportRef.value!, trackRef.value!, thumbRef.value!);
  viewportRef.value!.addEventListener("wheel", onCtrlWheel, { passive: false });
  lastW = viewportRef.value!.clientWidth;
  lastH = viewportRef.value!.clientHeight;
  ro.observe(viewportRef.value!);
  relayout();
});

onBeforeUnmount(() => {
  clearTimeout(resizeTimer);
  viewportRef.value?.removeEventListener("wheel", onCtrlWheel);
  detach();
  ro.disconnect();
});
</script>

<template>
  <div ref="viewportRef" class="grid-viewport" tabindex="0">
    <div
      class="grid-canvas"
      :style="{ transform: `scale(${canvasScale}) translateY(${-pos}px)` }"
    >
      <div ref="headerRef" class="grid-header">
        <FolderStrip />
        <SectionHeader title="内容" :count="items.length" />
      </div>
      <AssetCard
        v-for="c in visibleCards"
        :key="c.asset.id"
        :asset="c.asset"
        :box="c.box"
        :selected="content.selectedAssetId === c.asset.id"
        @click="content.selectedAssetId = c.asset.id"
      />
    </div>
    <div ref="trackRef" class="grid-track">
      <div
        ref="thumbRef"
        class="grid-thumb"
        :style="{ top: `${thumbTop}px`, height: `${thumbHeight}px` }"
      />
    </div>
  </div>
</template>

<style scoped>
.grid-viewport {
  position: relative;
  flex: 1;
  min-height: 0;
  overflow: hidden;
  outline: none;
}
.grid-canvas {
  position: absolute;
  left: 0;
  top: 0;
  right: 0;
  will-change: transform;
  transform-origin: 0 0;
}
/* 头部：文件夹区块与"内容"副标题（间距 16），随 canvas 一起滚动 */
.grid-header {
  display: flex;
  flex-direction: column;
  gap: var(--space-16);
}
/* token 15：滚动条 4px 胶囊 C4-15、贴 viewport 右缘、不影响布局；命中区 12px */
.grid-track {
  position: absolute;
  top: 0;
  bottom: 0;
  right: 0;
  width: 12px;
  z-index: 10;
}
.grid-thumb {
  position: absolute;
  right: 0;
  width: 4px;
  border-radius: 2px;
  background: var(--c4-15);
  cursor: grab;
}
</style>
