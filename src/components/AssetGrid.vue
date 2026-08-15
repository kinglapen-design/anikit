<script setup lang="ts">
// 内容区素材网格：自适应布局 + 自定义 transform 虚拟滚动（踩坑 005 定案）。
// 卡片列表共用一个滚动（03_UI 布局/03_中间面板 03）：文件夹区块与"内容"副标题
// 作为 canvas 头部随网格一起滚动，网格 boxes 按头部高度偏移；
// 滚动条贴 viewport 右缘（不占布局），asset-section 右 padding 4 作呼吸。
// 布局 / 虚拟滚动编排在 lib/useGridLayout，空格放大预览在 lib/useSpaceZoom（2026-08-12 拆出）。
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import AssetCard from "./AssetCard.vue";
import HoverZoomOverlay from "./HoverZoomOverlay.vue";
import ListRowCard from "./ListRowCard.vue";
import ListHeader from "./ListHeader.vue";
import FolderStrip from "./FolderStrip.vue";
import ArchiveStrip from "./ArchiveStrip.vue";
import InboxDiskSection from "./InboxDiskSection.vue";
import SectionHeader from "./SectionHeader.vue";
import { useGridLayout } from "../lib/useGridLayout";
import { useSpaceZoom } from "../lib/useSpaceZoom";
import { useMarquee, makeBoxHitTest, headerBlankClear } from "../lib/useMarquee";
import {
  dragView,
  gridPointerDown,
  makeGridResolver,
  registerGridResolver,
  unregisterGridResolver,
  assetCardFade,
} from "../lib/dragOperation";
import { openGridContextMenu } from "../lib/menuSpecs/gridDispatch";
import { useContentStore } from "../stores/content";
import { useSelectionStore } from "../stores/selection";
import { useUiStore } from "../stores/ui";
import { usePreviewStore } from "../stores/preview";
import type { PlacementView } from "../stores/content";

const props = withDefaults(defineProps<{ items: PlacementView[]; viewKey: string; panelActive?: boolean }>(), {
  panelActive: false,
});
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

const viewportRef = ref<HTMLElement>();
const trackRef = ref<HTMLElement>();
const thumbRef = ref<HTMLElement>();
const headerRef = ref<HTMLElement>();
const cardsLayerRef = ref<HTMLElement>(); // 拖拽间隙落点坐标换算（C4 resolver）

// 布局 / 虚拟滚动 / 重排编排（lib/useGridLayout）：boxes / 可见窗口 / 滚动态均由此供模板与框选、拖拽接线
const {
  pos,
  thumbTop,
  thumbHeight,
  scrollable,
  dragging,
  boxes,
  headerH,
  gridTotal,
  assetsOpen,
  listExtra,
  visibleCards,
} = useGridLayout({
  items: () => props.items,
  viewKey: () => props.viewKey,
  viewportRef,
  trackRef,
  thumbRef,
  headerRef,
});

// 收件箱视口（quick:inbox，4.4）：头部出"工程文件"磁盘实况区块，"内容"副标题改题"已导入"
const isInbox = computed(() => props.viewKey === "quick:inbox");

// 空格放大预览（lib/useSpaceZoom）：浮层态 + 悬停追踪；panelHover 滚动条悬停显隐共用
const { zoomAsset, panelHover, trackPointer, onGridMouseLeave, onZoomSpace } = useSpaceZoom({
  items: () => props.items,
  panelActive: () => props.panelActive,
});
function onGridPointerMove(e: PointerEvent): void {
  trackPointer(e);
  onMarqueeMove(e);
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
    offsetY: () => headerH.value + 12 + listExtra.value - pos.value,
    keyAt: (i) => `${props.items[i].assetId}|${props.items[i].folderId}`,
  }),
});

// —— 右键菜单（6.3 全量改造；1.3 一轮按新稿 02_布局 重写）：
// 统一调度在 lib/menuSpecs/gridDispatch.openGridContextMenu（素材 / 回收站 13 稿 / 空白新增菜单
// 均走全局宿主 lib/contextMenus；RecycleCardMenu 局部菜单 2026-08-11 废除）
function onCardContextMenu(e: MouseEvent): void {
  openGridContextMenu(e, props.items);
}

// —— 卡片间距排序落点解析器（C4a）：注册给拖拽引擎，算法主体在 lib/dragOperation makeGridResolver
const resolveGap = makeGridResolver({
  isFolderView: () => props.viewKey.startsWith("folder:"),
  layer: () => cardsLayerRef.value,
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

onMounted(() => {
  viewportRef.value!.addEventListener("wheel", onCtrlWheel, { passive: false });
  registerGridResolver(resolveGap); // 拖拽卡片间距落点解析（C4）
});

onBeforeUnmount(() => {
  viewportRef.value?.removeEventListener("wheel", onCtrlWheel);
  unregisterGridResolver();
});
</script>

<template>
  <div
    ref="viewportRef"
    class="grid-viewport"
    tabindex="0"
    @pointerdown="gridPointerDown($event, onMarqueeDown)"
    @pointermove="onGridPointerMove"
    @pointerup="onMarqueeUp"
    @pointercancel="onMarqueeCancel"
    @mouseenter="panelHover = true"
    @mouseleave="onGridMouseLeave"
    @contextmenu="onCardContextMenu"
  >
    <div class="grid-canvas" :style="{ transform: `translateY(${-pos}px)` }">
      <div ref="headerRef" class="grid-header" @pointerdown="headerBlankClear">
        <!-- 当前视口无子文件夹时整个文件夹区块隐藏（阶段3修改定案；1.2 按本标签页 viewKey 取子文件夹） -->
        <FolderStrip v-if="content.stripFoldersFor(viewKey).length > 0" :view-key="viewKey" />
        <!-- 档案区（3.2 一轮，PRD 02-03 三区：文件夹 / 档案 / 文件）：无档案整块不渲染（同文件夹区口径） -->
        <ArchiveStrip v-if="content.stripArchivesFor(viewKey).length > 0" :view-key="viewKey" />
        <!-- 收件箱视口（4.4）：磁盘实况"工程文件"区块在 FolderStrip 同位（快速访问不出文件夹区块） -->
        <InboxDiskSection v-if="isInbox && content.inboxDisk.length > 0" />
        <!-- "内容"副标题：无子文件夹（含档案）时隐藏（无需区分）；有子文件夹但无文件时同样隐藏（2026-07-24 用户定）。
             收件箱视口改题"已导入"（4.4）：无文件夹区块概念，只要有已导入内容就显示。
             padding 统一 0（2026-08-08 用户定：仅目录树区块标题 left 8）；标题↔卡片间距 12（下方 translateY） -->
        <SectionHeader
          v-if="items.length > 0 && (isInbox || content.stripFoldersFor(viewKey).length > 0 || content.stripArchivesFor(viewKey).length > 0)"
          :title="isInbox ? '已导入' : '内容'"
          :count="items.length"
          v-model:open="assetsOpen"
        />
      </div>
      <div
        v-show="assetsOpen"
        ref="cardsLayerRef"
        class="cards-layer"
        :style="{
          transform: `translateY(${headerH + 12 + listExtra}px)`,
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
    <!-- 滚动条悬停显隐（2026-08-10 用户定：只在悬停面板 / 拖拽 thumb 时出现，持久存在视觉噪音） -->
    <div
      ref="trackRef"
      class="grid-track"
      :class="{ visible: scrollable && (panelHover || dragging) }"
    >
      <div
        ref="thumbRef"
        class="grid-thumb"
        :style="{ top: `${thumbTop}px`, height: `${thumbHeight}px` }"
      />
    </div>
    <!-- 列头：随内容滚动至顶部后钉住（自定义 transform 滚动无 CSS sticky，手动计算） -->
    <ListHeader
      v-if="ui.layoutMode === 'list' && assetsOpen"
      :style="{ top: `${Math.max(0, headerH + 12 - pos)}px` }"
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
    <!-- 空格放大预览浮层（2026-08-10 访达 Quick Look 逻辑）：Teleport 到 body；
         关闭 = Esc / 再按空格 / 选择变化（点击穿透到网格自然驱动）。
         :key = asset key（2026-08-10 根因修复：组件只在挂载时加载图、asset prop 变化不重载——
         浮层开着时切换 / 点选状态换了图不换，三轮误判根因实在此；key 变即强制重建重载） -->
    <HoverZoomOverlay
      v-if="zoomAsset"
      :key="`${zoomAsset.assetId}|${zoomAsset.folderId}`"
      :asset="zoomAsset"
      @close="zoomAsset = null"
      @space="onZoomSpace"
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
/* 头部：文件夹区块与"内容"副标题（区块间隔 24——figma 42 列表 gap 24，2026-08-10），
   grid 自然流即时精确，不参与缩放 */
.grid-header {
  display: flex;
  flex-direction: column;
  gap: var(--space-24);
}
/* 卡片层：translateY 实时跟随头部高度（逐帧重排定案后无 scale 过渡，2026-08-10）；
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
/* border-1：滚动条 6px 胶囊、贴 viewport 右缘、不影响布局；命中区 12px。
   悬停显隐（2026-08-10 用户定）：默认透明且不接收事件（不挡卡片右缘），visible 淡入 */
.grid-track {
  position: absolute;
  top: 0;
  bottom: 0;
  right: 0;
  width: 12px;
  z-index: 10;
  opacity: 0;
  pointer-events: none;
  transition: opacity 0.15s ease;
}
.grid-track.visible {
  opacity: 1;
  pointer-events: auto;
}
.grid-thumb {
  position: absolute;
  right: 0;
  width: 6px;
  border-radius: var(--radius-pill);
  /* 滚动条三栏同口径（2026-08-10 用户定二轮）：6px 胶囊紧贴右边框 / 实色——常态 C4、
     悬停 C5（透明色在通栏分割线上会交叉叠加，作废） */
  background: var(--c4);
  transition: background 0.12s ease;
}
.grid-thumb:hover {
  background: var(--c5);
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
/* 拖拽卡片间距落点色块（C4a，2026-07-28 定案）：圆角竖向矩形填间距，同框选配色；
   填充 30%（2026-08-10 用户定：12% 不够明显，与并入填充同档） */
.drop-gap {
  position: absolute;
  z-index: 5;
  pointer-events: none;
  border-radius: var(--radius-4);
  background: var(--brand-1-30);
  box-shadow: inset 0 0 0 1px var(--brand-1);
}
</style>
