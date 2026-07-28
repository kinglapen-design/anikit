<script setup lang="ts">
// 内容区素材网格：自适应布局 + 自定义 transform 虚拟滚动（踩坑 005 定案）。
// 卡片列表共用一个滚动（03_UI 布局/03_中间面板 03）：文件夹区块与"内容"副标题
// 作为 canvas 头部随网格一起滚动，网格 boxes 按头部高度偏移；
// 滚动条贴 viewport 右缘（不占布局），中间面板 4px 内边距作呼吸。
import { computed, onBeforeUnmount, onMounted, ref, shallowRef, watch } from "vue";
import AssetCard from "./AssetCard.vue";
import HoverZoomOverlay from "./HoverZoomOverlay.vue";
import ListRowCard from "./ListRowCard.vue";
import ListHeader from "./ListHeader.vue";
import FolderStrip from "./FolderStrip.vue";
import InboxDiskSection from "./InboxDiskSection.vue";
import SectionHeader from "./SectionHeader.vue";
import AssetCardMenu from "./AssetCardMenu.vue";
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
import { useMarquee, makeBoxHitTest, headerBlankClear } from "../lib/useMarquee";
import {
  dragView,
  gridPointerDown,
  makeGridResolver,
  registerGridResolver,
  unregisterGridResolver,
  assetCardFade,
} from "../lib/dragOperation";
import { useContentStore } from "../stores/content";
import { useSelectionStore } from "../stores/selection";
import { useUiStore } from "../stores/ui";
import { usePreviewStore } from "../stores/preview";
import type { PlacementView } from "../stores/content";

const props = defineProps<{ items: PlacementView[] }>();
const ui = useUiStore();
const content = useContentStore();
const selection = useSelectionStore();
const preview = usePreviewStore();

// 进预览（4.3）：双击卡片
function openPreview(p: PlacementView): void {
  void preview.open(`${p.assetId}|${p.folderId}`);
}

// 卡片点击选择（6.1，2026-07-28 用户定案）：单击 = 单选 / Ctrl+单击 = 加选切换 /
// Shift+单击 = 锚点范围选（selection store 三态）；素材卡片选中与文件夹 / 收件箱条目互斥
//（文件夹选择域在 selection store，clickItem 内部互斥清理，2026-07-28 复验反馈 5）
function onCardClick(p: PlacementView, e: MouseEvent): void {
  if (dragView.suppressClick) return; // 有效拖拽结束后紧随的 click 不当选择（拖拽引擎置位）
  // 信息文字区（.card-info）左键单击不选中卡片（2026-07-29 复验二轮 4，对齐 Eagle——
  // 视为空白：框选未过阈值已清空选择；右键菜单 / 双击预览不受影响）
  if ((e.target as HTMLElement).closest(".card-info")) return;
  if (e.ctrlKey || e.metaKey || e.shiftKey) {
    selection.clickItem(`${p.assetId}|${p.folderId}`, { ctrl: e.ctrlKey || e.metaKey, shift: e.shiftKey });
    content.selectedInboxName = "";
  } else {
    selection.selectOnly(`${p.assetId}|${p.folderId}`);
  }
}

// 悬停放大浮层（5.3）：卡片放大镜按钮触发，同时最多一个
const zoomAsset = ref<PlacementView | null>(null);

// 布局算法只认数值宽高：fla 等无尺寸资源按 1:1 方形（4.2 用户定——格式图标为方形，容器随方不裁切）
const layoutItems = computed(() => props.items.map((it) => ({ w: it.w ?? 300, h: it.h ?? 300 })));

const BUFFER = 800;

const viewportRef = ref<HTMLElement>();
const trackRef = ref<HTMLElement>();
const thumbRef = ref<HTMLElement>();
const headerRef = ref<HTMLElement>();
const cardsLayerRef = ref<HTMLElement>(); // 拖拽间隙落点坐标换算（C4 resolver）

const boxes = shallowRef<Box[]>([]);
const rows = shallowRef<Row[]>([]);
const r0 = ref(0);
const r1 = ref(0);
const headerH = ref(0);
const gridTotal = ref(0);
const assetsOpen = ref(true);

// 收件箱视口（quick:inbox，4.4）：头部出"工程文件"磁盘实况区块，"内容"副标题改题"已导入"
const isInbox = computed(() => content.sidebarSelected === "quick:inbox");

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

const { pos, thumbTop, thumbHeight, scrollable, scrollTo, setTotal, attach, detach } =
  useVirtualScroll(updateRange);

// 滚动位置持久化（2026-07-25 用户定）：按视口 key 保存，重排完成后恢复（null = 不恢复）
let restorePos: number | null = null;

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
  // 滚动位置恢复（进出预览 / 切换目录后的重排；超出新列表由 scrollTo 自夹）
  if (restorePos !== null) {
    scrollTo(restorePos);
    restorePos = null;
  }
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

// —— 框选（6.1，2026-07-28 用户定案；手势与命中算法在 lib/useMarquee）——
// 命中口径 = 矩形（viewport 坐标）与 Box 换算后的可视矩形相交（相交口径，非全包）
const {
  marquee,
  onPointerDown: onMarqueeDown,
  onPointerMove: onMarqueeMove,
  onPointerUp: onMarqueeUp,
  onPointerCancel: onMarqueeCancel,
} = useMarquee({
  viewport: viewportRef,
  // 头部（文件夹区块、"内容"副标题、工程文件、列表列头）/ 滚动条 / 按钮上不启动框选；
  // 卡片本体由 gridPointerDown 先行分发去拖拽不会到此，卡片信息区 .card-info 刻意放行
  //（2026-07-29 复验二轮 4：信息区视为空白起框选；卡片行 / 列间距点在 cards-layer 上同落入）
  ignoreSelector: ".grid-header, .list-header, .grid-track, button",
  hitTest: makeBoxHitTest({
    boxes: () => boxes.value,
    scale: () => canvasScale.value,
    offsetY: () => headerH.value + 6 + listExtra.value - pos.value,
    keyAt: (i) => `${props.items[i].assetId}|${props.items[i].folderId}`,
  }),
});

// —— 卡片右键菜单（6.1 C6）：右键未选中项 = 先单选它；菜单本体在 AssetCardMenu（Teleport body）
const cardMenu = ref<{ x: number; y: number } | null>(null);
function onCardContextMenu(e: MouseEvent): void {
  const card = (e.target as HTMLElement).closest(".asset-card, .list-row");
  const key = card?.getAttribute("data-key");
  if (!key) return;
  e.preventDefault();
  if (!selection.isSelected(key)) selection.selectOnly(key);
  cardMenu.value = { x: e.clientX, y: e.clientY };
}

// —— 卡片间距排序落点解析器（C4a）：注册给拖拽引擎，算法主体在 lib/dragOperation makeGridResolver
const resolveGap = makeGridResolver({
  isFolderView: () => content.sidebarSelected.startsWith("folder:"),
  layer: () => cardsLayerRef.value,
  scale: () => canvasScale.value,
  boxes: () => boxes.value,
  keyAt: (i) => `${props.items[i].assetId}|${props.items[i].folderId}`,
});

// 就地重命名（C2）：滚出虚拟滚动销毁 = 取消——重命名卡片离开可视渲染集时清 renamingKey
//（卡片组件 blur 晚于本清理到达时按 key 失配判取消，见 AssetCard.commitRename）
watch(visibleCards, (cards) => {
  const key = selection.renamingKey;
  if (key && !cards.some((c) => `${c.asset.assetId}|${c.asset.folderId}` === key)) {
    selection.renamingKey = null;
  }
});

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

// 切换目录 / 视图：保存旧视口位置、登记新视口待恢复（随后 items watch 重排完成时恢复。
// 注册顺序必须在 items watch 之前——同 tick 内按注册顺序触发，restorePos 要先就位）
watch(
  () => content.sidebarSelected,
  (newKey, oldKey) => {
    content.saveScrollPos(oldKey, pos.value);
    restorePos = content.getScrollPos(newKey) ?? 0;
  },
);
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
  registerGridResolver(resolveGap); // 拖拽卡片间距落点解析（C4）
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
  // 预览返回 / 视口初次挂载：恢复上次滚动位置（2026-07-25 用户定）
  restorePos = content.getScrollPos(content.sidebarSelected);
  relayout();
});

onBeforeUnmount(() => {
  // 卸载（进预览等）前保存当前视口滚动位置
  content.saveScrollPos(content.sidebarSelected, pos.value);
  clearTimeout(resizeTimer);
  viewportRef.value?.removeEventListener("wheel", onCtrlWheel);
  unregisterGridResolver();
  detach();
  ro.disconnect();
  headerRo?.disconnect();
});
</script>

<template>
  <div
    ref="viewportRef"
    class="grid-viewport"
    tabindex="0"
    @pointerdown="gridPointerDown($event, onMarqueeDown)"
    @pointermove="onMarqueeMove"
    @pointerup="onMarqueeUp"
    @pointercancel="onMarqueeCancel"
    @contextmenu="onCardContextMenu"
  >
    <div class="grid-canvas" :style="{ transform: `translateY(${-pos}px)` }">
      <div ref="headerRef" class="grid-header" @pointerdown="headerBlankClear">
        <!-- 当前视口无子文件夹时整个文件夹区块隐藏（阶段3修改定案） -->
        <FolderStrip v-if="content.stripFolders.length > 0" />
        <!-- 收件箱视口（4.4）：磁盘实况"工程文件"区块在 FolderStrip 同位（快速访问不出文件夹区块） -->
        <InboxDiskSection v-if="isInbox && content.inboxDisk.length > 0" />
        <!-- "内容"副标题：无子文件夹时隐藏（无需区分）；有子文件夹但无文件时同样隐藏（2026-07-24 用户定）。
             收件箱视口改题"已导入"（4.4）：无文件夹区块概念，只要有已导入内容就显示 -->
        <SectionHeader
          v-if="items.length > 0 && (isInbox || content.stripFolders.length > 0)"
          :title="isInbox ? '已导入' : '内容'"
          :count="items.length"
          pad="8px 0 0 0"
          v-model:open="assetsOpen"
        />
      </div>
      <div
        v-show="assetsOpen"
        ref="cardsLayerRef"
        class="cards-layer"
        :style="{
          transform: `translateY(${headerH + 6 + listExtra}px) scale(${canvasScale})`,
          height: `${gridTotal}px`,
        }"
      >
        <template v-if="ui.layoutMode === 'list'">
          <ListRowCard
            v-for="c in visibleCards"
            :key="`${c.asset.assetId}|${c.asset.folderId}`"
            :asset="c.asset"
            :box="c.box"
            :odd="c.odd"
            :selected="selection.isSelected(`${c.asset.assetId}|${c.asset.folderId}`)"
            :renaming="selection.renamingKey === `${c.asset.assetId}|${c.asset.folderId}`"
            :faded="assetCardFade(`${c.asset.assetId}|${c.asset.folderId}`)"
            :data-key="`${c.asset.assetId}|${c.asset.folderId}`"
            :data-drag-name="c.asset.name"
            @select="onCardClick(c.asset, $event)"
            @dblclick="openPreview(c.asset)"
          />
        </template>
        <template v-else>
          <AssetCard
            v-for="c in visibleCards"
            :key="`${c.asset.assetId}|${c.asset.folderId}`"
            :asset="c.asset"
            :box="c.box"
            :selected="selection.isSelected(`${c.asset.assetId}|${c.asset.folderId}`)"
            :renaming="selection.renamingKey === `${c.asset.assetId}|${c.asset.folderId}`"
            :faded="assetCardFade(`${c.asset.assetId}|${c.asset.folderId}`)"
            :data-key="`${c.asset.assetId}|${c.asset.folderId}`"
            :data-drag-name="c.asset.name"
            @click="onCardClick(c.asset, $event)"
            @dblclick="openPreview(c.asset)"
            @zoom="zoomAsset = c.asset"
            @zoom-close="zoomAsset = null"
          />
        </template>
        <!-- 拖拽卡片间距落点（C4a）：圆角竖向矩形色块填间距（--brand-1-12 + inset 1px --brand-1），
             坐标与卡片同层同变换，事件穿透 -->
        <div
          v-if="dragView.gap"
          class="drop-gap"
          :style="{
            left: `${dragView.gap.left}px`,
            top: `${dragView.gap.top}px`,
            width: `${dragView.gap.width}px`,
            height: `${dragView.gap.height}px`,
          }"
        />
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
    <!-- 框选矩形（6.1）：--brand-1-12 填充 + 1px --brand-1 描边，事件穿透不挡卡片 -->
    <div
      v-if="marquee"
      class="marquee"
      :style="{
        left: `${marquee.left}px`,
        top: `${marquee.top}px`,
        width: `${marquee.width}px`,
        height: `${marquee.height}px`,
      }"
    />
    <!-- 悬停放大浮层（5.3）：Teleport 到 body，点击任意处 / Esc 关闭 -->
    <HoverZoomOverlay v-if="zoomAsset" :asset="zoomAsset" @close="zoomAsset = null" />
    <!-- 卡片右键菜单（6.1 C6）：组件内 Teleport body（transform 祖先坑，同 InboxDiskSection） -->
    <AssetCardMenu v-if="cardMenu" :x="cardMenu.x" :y="cardMenu.y" @close="cardMenu = null" />
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
/* 卡片层：translateY 实时跟随头部高度 + scale 跟随宽度差（重排前的视觉过渡）；
   显式高度 = 网格总高（2026-07-28 复验反馈 2/3 根因：子卡片全绝对定位、层高度塌缩为 0——
   间隙落点容差与"点在层上但未点中卡片"的框选判定都依赖层有自己的命中区域） */
.cards-layer {
  position: absolute;
  left: 0;
  top: 0;
  right: 0;
  transform-origin: 0 0;
  will-change: transform;
}
/* border-1：滚动条 4px 胶囊、贴 viewport 右缘、不影响布局；命中区 12px */
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
  /* 滚动条常态 font-icon-5（选中色），悬停 border-1；不用抓手光标（2026-07-23 用户定） */
  background: var(--font-icon-5);
  transition: background 0.12s ease;
}
.grid-thumb:hover {
  background: var(--border-1);
}
/* 框选矩形（6.1，2026-07-28 用户定）：--brand-1-12 填充 + 1px --brand-1 描边
   （inset shadow 模拟边框——规范第 4 条 UI 线条不用 border）；滚动条（z 10）之下、卡片之上 */
.marquee {
  position: absolute;
  z-index: 5;
  pointer-events: none;
  background: var(--brand-1-12);
  box-shadow: inset 0 0 0 1px var(--brand-1);
}
/* 拖拽卡片间距落点色块（C4a，2026-07-28 定案）：圆角竖向矩形填间距，同框选配色 */
.drop-gap {
  position: absolute;
  z-index: 5;
  pointer-events: none;
  border-radius: var(--radius-4);
  background: var(--brand-1-12);
  box-shadow: inset 0 0 0 1px var(--brand-1);
}
</style>
