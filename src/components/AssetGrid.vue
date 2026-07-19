<script setup lang="ts">
// 内容区素材网格：自适应布局 + 自定义 transform 虚拟滚动（踩坑 005 定案）。
// 布局为自写贪心算法（lib/justified.ts，10 万条 <100ms）；
// 窗口 / 拆分条 resize 经 debounce 后才重排，拖动过程保持旧布局。
import { computed, onBeforeUnmount, onMounted, ref, shallowRef, watch } from "vue";
import AssetCard from "./AssetCard.vue";
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
  const t0 = performance.now();
  const result = computeJustified(props.items, vp.clientWidth, ui.targetRowHeight, CARD_CAPTION_H);
  boxes.value = result.boxes;
  rows.value = result.rows;
  totalHeight = result.totalHeight;
  setTotal(totalHeight);
  console.debug(`[AssetGrid] 布局 ${props.items.length} 条：${(performance.now() - t0).toFixed(1)}ms`);
}

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
  // 拖动窗口 / 拆分条期间 debounce：过程不重排，停顿 200ms 后才重排
  clearTimeout(resizeTimer);
  resizeTimer = setTimeout(() => {
    if (widthChanged) relayout();
    else setTotal(totalHeight);
  }, 200);
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
  lastW = viewportRef.value!.clientWidth;
  lastH = viewportRef.value!.clientHeight;
  ro.observe(viewportRef.value!);
  relayout();
});

onBeforeUnmount(() => {
  clearTimeout(resizeTimer);
  detach();
  ro.disconnect();
});
</script>

<template>
  <div ref="viewportRef" class="grid-viewport" tabindex="0">
    <div class="grid-canvas" :style="{ transform: `translateY(${-pos}px)` }">
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
}
/* token 15：滚动条 4px 胶囊 C4-15、距右 4px、不影响布局；命中区放宽至 12px */
.grid-track {
  position: absolute;
  top: 0;
  bottom: 0;
  right: 4px;
  width: 12px;
  z-index: 10;
}
.grid-thumb {
  position: absolute;
  left: 4px;
  width: 4px;
  border-radius: 2px;
  background: var(--c4-15);
  cursor: grab;
}
</style>
