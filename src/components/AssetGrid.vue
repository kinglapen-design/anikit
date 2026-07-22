<script setup lang="ts">
// 内容区素材网格：自适应布局 + 自定义 transform 虚拟滚动（踩坑 005 定案）。
// 卡片列表共用一个滚动（03_UI 布局/03_中间面板 03）：文件夹区块与"内容"副标题
// 作为 canvas 头部随网格一起滚动，网格 boxes 按头部高度偏移；
// 滚动条贴 viewport 右缘（不占布局），中间面板 4px 内边距作呼吸（token 15）。
import { computed, onBeforeUnmount, onMounted, ref, shallowRef, watch } from "vue";
import AssetCard from "./AssetCard.vue";
import ListRowCard from "./ListRowCard.vue";
import ListHeader from "./ListHeader.vue";
import FolderStrip from "./FolderStrip.vue";
import SectionHeader from "./SectionHeader.vue";
import {
  CARD_CAPTION_H,
  computeJustified,
  computeWaterfall,
  computeGrid,
  computeList,
  lowerBound,
  type Box,
  type Row,
} from "../lib/layouts";
import { useVirtualScroll } from "../lib/useVirtualScroll";
import { useContentStore } from "../stores/content";
import { useUiStore } from "../stores/ui";
import type { PlacementView } from "../stores/content";

const props = defineProps<{ items: PlacementView[] }>();
const ui = useUiStore();
const content = useContentStore();

// 布局算法只认数值宽高：fla 等无尺寸资源按 400×300 默认比例（阶段 5 插件补图后刷新）
const layoutItems = computed(() => props.items.map((it) => ({ w: it.w ?? 400, h: it.h ?? 300 })));

const BUFFER = 800;

const viewportRef = ref<HTMLElement>();
const trackRef = ref<HTMLElement>();
const thumbRef = ref<HTMLElement>();
const headerRef = ref<HTMLElement>();

const boxes = shallowRef<Box[]>([]);
const rows = shallowRef<Row[]>([]);
const r0 = ref(0);
const r1 = ref(0);
const headerH = ref(0);
const gridTotal = ref(0);
const assetsOpen = ref(true);

// 总高 = 头部（文件夹区块 + 内容副标题 + 间距 6）+ 网格高（折叠"内容"时不计）
// 列表模式另有固定列头 28px + 列头与列表间距 6（不占布局、不随滚动）
const listExtra = computed(() => (ui.layoutMode === "list" ? 34 : 0));

function currentTotal(): number {
  return (assetsOpen.value ? gridTotal.value : 0) + headerH.value + 6 + listExtra.value;
}

function updateRange(scrollPos: number): void {
  const vh = viewportRef.value?.clientHeight ?? 0;
  const off = headerH.value + 6;
  const rs = rows.value;
  r0.value = Math.max(0, lowerBound(rs, scrollPos - off - BUFFER) - 1);
  r1.value = Math.min(rs.length, lowerBound(rs, scrollPos + vh - off + BUFFER) + 1);
}

const { pos, thumbTop, thumbHeight, scrollable, setTotal, attach, detach } = useVirtualScroll(updateRange);

const visibleCards = computed(() => {
  const rs = rows.value;
  const bs = boxes.value;
  const out: { asset: PlacementView; box: Box; odd: boolean }[] = [];
  if (rs.length === 0) return out;
  const from = Math.min(r0.value, rs.length - 1);
  const to = Math.min(r1.value, rs.length);
  for (let r = from; r < to; r++) {
    for (let i = rs[r].start; i < rs[r].end; i++) {
      out.push({ asset: props.items[bs[i].i], box: bs[i], odd: r % 2 === 1 });
    }
  }
  return out;
});

function relayout(): void {
  const vp = viewportRef.value;
  if (!vp || vp.clientWidth <= 0) return;
  ui.gridWidth = vp.clientWidth;
  // 行高只在布局时按容器宽 clamp（滑块 max = 内容区宽、min 80），
  // 不回写 targetRowHeight——否则窗口收窄到最小再拉出，卡片会保持被压的小尺寸
  const maxH = Math.max(80, vp.clientWidth);
  const rowH = Math.max(80, Math.min(maxH, ui.targetRowHeight));
  const t0 = performance.now();
  // 四种布局（PRD 04）：自适应 / 瀑布流 / 网格 / 列表
  const result =
    ui.layoutMode === "waterfall"
      ? computeWaterfall(layoutItems.value, vp.clientWidth, rowH, CARD_CAPTION_H)
      : ui.layoutMode === "grid"
        ? computeGrid(layoutItems.value, vp.clientWidth, rowH, CARD_CAPTION_H)
        : ui.layoutMode === "list"
          ? computeList(layoutItems.value, vp.clientWidth, rowH, CARD_CAPTION_H)
          : computeJustified(layoutItems.value, vp.clientWidth, rowH, CARD_CAPTION_H);
  boxes.value = result.boxes;
  rows.value = result.rows;
  gridTotal.value = result.totalHeight;
  layoutWidth.value = vp.clientWidth;
  canvasScale.value = 1;
  setTotal(currentTotal());
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

// 拖动窗口 / 拆分条期间：卡片层按宽度差等比缩放作重排前的视觉过渡——
// 用户拖窗口是要更多展示空间（更多列），不是放大卡片：scale 限制 ±4%（轻微补充），
// 超出立即重排（justified 列数随宽度变化），否则停顿 240ms 精确重排。
const layoutWidth = ref(0);
const canvasScale = ref(1);
const SCALE_THRESHOLD = 0.04;

let lastW = 0;
let lastH = 0;
let resizeTimer: ReturnType<typeof setTimeout> | undefined;
let headerRo: ResizeObserver | undefined;
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
    setTotal(currentTotal());
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
// 折叠 / 展开"内容"：网格高度不计入总高
watch(assetsOpen, () => setTotal(currentTotal()));
// 布局方式切换（查看方式菜单）
watch(
  () => ui.layoutMode,
  () => relayout(),
);

onMounted(() => {
  attach(viewportRef.value!, trackRef.value!, thumbRef.value!);
  viewportRef.value!.addEventListener("wheel", onCtrlWheel, { passive: false });
  lastW = viewportRef.value!.clientWidth;
  lastH = viewportRef.value!.clientHeight;
  ro.observe(viewportRef.value!);
  // 头部高度实时跟踪（grid 换行 / 宽度变化时卡片层偏移即时跟随，避免重叠拉远）
  headerH.value = headerRef.value!.clientHeight;
  headerRo = new ResizeObserver(() => {
    headerH.value = headerRef.value!.clientHeight;
    setTotal(currentTotal());
  });
  headerRo.observe(headerRef.value!);
  relayout();
});

onBeforeUnmount(() => {
  clearTimeout(resizeTimer);
  viewportRef.value?.removeEventListener("wheel", onCtrlWheel);
  detach();
  ro.disconnect();
  headerRo?.disconnect();
});
</script>

<template>
  <div ref="viewportRef" class="grid-viewport" tabindex="0">
    <div class="grid-canvas" :style="{ transform: `translateY(${-pos}px)` }">
      <div ref="headerRef" class="grid-header">
        <!-- 当前视口无子文件夹时整个文件夹区块隐藏（阶段3修改定案） -->
        <FolderStrip v-if="content.stripFolders.length > 0" />
        <!-- 素材区块顶 padding 8（与文件夹区块底 8、总容器 gap 16 构成间距 32 = 8 + 16 + 8） -->
        <SectionHeader title="内容" :count="items.length" pad="8px 0 0 0" v-model:open="assetsOpen" />
      </div>
      <div
        v-show="assetsOpen"
        class="cards-layer"
        :style="{ transform: `translateY(${headerH + 6 + listExtra}px) scale(${canvasScale})` }"
      >
        <template v-if="ui.layoutMode === 'list'">
          <ListRowCard
            v-for="c in visibleCards"
            :key="`${c.asset.assetId}|${c.asset.folderId}`"
            :asset="c.asset"
            :box="c.box"
            :odd="c.odd"
            :selected="content.selectedAssetKey === `${c.asset.assetId}|${c.asset.folderId}`"
            @select="
              content.selectedAssetKey = `${c.asset.assetId}|${c.asset.folderId}`;
              content.selectedFolderId = '';
            "
          />
        </template>
        <template v-else>
          <AssetCard
            v-for="c in visibleCards"
            :key="`${c.asset.assetId}|${c.asset.folderId}`"
            :asset="c.asset"
            :box="c.box"
            :selected="content.selectedAssetKey === `${c.asset.assetId}|${c.asset.folderId}`"
            @click="
              content.selectedAssetKey = `${c.asset.assetId}|${c.asset.folderId}`;
              content.selectedFolderId = '';
            "
          />
        </template>
      </div>
    </div>
    <div v-show="scrollable" ref="trackRef" class="grid-track">
      <div
        ref="thumbRef"
        class="grid-thumb"
        :style="{ top: `${thumbTop}px`, height: `${thumbHeight}px` }"
      />
    </div>
    <!-- 列头：随内容滚动至顶部后钉住（自定义 transform 滚动无 CSS sticky，手动计算） -->
    <ListHeader
      v-if="ui.layoutMode === 'list' && assetsOpen"
      :style="{ top: `${Math.max(0, headerH + 6 - pos)}px` }"
    />
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
/* 头部：文件夹区块与"内容"副标题（间距 16），grid 自然流即时精确，不参与缩放 */
.grid-header {
  display: flex;
  flex-direction: column;
  gap: var(--space-16);
}
/* 卡片层：translateY 实时跟随头部高度 + scale 跟随宽度差（重排前的视觉过渡） */
.cards-layer {
  position: absolute;
  left: 0;
  top: 0;
  right: 0;
  transform-origin: 0 0;
  will-change: transform;
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
