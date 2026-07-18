<script setup lang="ts">
// 内容区素材网格：自适应布局 + 自定义 transform 虚拟滚动（踩坑 005 定案）。
import { computed, nextTick, onBeforeUnmount, onMounted, ref, shallowRef, watch } from "vue";
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
const layingOut = ref(true);
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

let relayoutSeq = 0;
async function relayout(): Promise<void> {
  const vp = viewportRef.value;
  if (!vp || vp.clientWidth <= 0) return;
  const seq = ++relayoutSeq;
  layingOut.value = true;
  await nextTick();
  // 10 万条约 2.6s（05 待办：分片 / 缓存 / 移 Rust），先让"布局计算中…"绘制出来
  await new Promise((r) => setTimeout(r, 30));
  if (seq !== relayoutSeq) return;
  const result = computeJustified(props.items, vp.clientWidth, ui.targetRowHeight, CARD_CAPTION_H);
  boxes.value = result.boxes;
  rows.value = result.rows;
  totalHeight = result.totalHeight;
  layingOut.value = false;
  setTotal(totalHeight);
}

let lastW = 0;
let lastH = 0;
const ro = new ResizeObserver(() => {
  const vp = viewportRef.value;
  if (!vp) return;
  const w = vp.clientWidth;
  const h = vp.clientHeight;
  if (w !== lastW) {
    lastW = w;
    lastH = h;
    void relayout();
  } else if (h !== lastH) {
    lastH = h;
    setTotal(totalHeight);
  }
});

watch(
  () => props.items,
  () => void relayout(),
);
watch(
  () => ui.targetRowHeight,
  () => void relayout(),
);

onMounted(() => {
  attach(viewportRef.value!, trackRef.value!, thumbRef.value!);
  lastW = viewportRef.value!.clientWidth;
  lastH = viewportRef.value!.clientHeight;
  ro.observe(viewportRef.value!);
  void relayout();
});

onBeforeUnmount(() => {
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
    <div v-if="layingOut" class="laying-out">布局计算中…</div>
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
.laying-out {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: var(--text-caption-size);
  color: var(--c4-40);
  pointer-events: none;
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
