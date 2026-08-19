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
import { useGridLayout, type GridSection } from "../lib/useGridLayout";
import { groupPlacements } from "../lib/grouping";
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
import { groupHeaderSpec } from "../lib/menuSpecs/groupHeader";
import { openContextMenu } from "../lib/contextMenus";
import { useContentStore, type FolderNode, type PlacementView } from "../stores/content";
import { useSelectionStore } from "../stores/selection";
import { useUiStore } from "../stores/ui";
import { usePreviewStore } from "../stores/preview";

const props = withDefaults(
  defineProps<{
    items: PlacementView[];
    viewKey: string;
    panelActive?: boolean;
    // 3.6 搜索态（ContentArea 判定传入）：分组失效平铺 / 文件夹区块隐藏 / 档案区显示命中档案
    searching?: boolean;
    searchArchives?: FolderNode[];
  }>(),
  { panelActive: false, searching: false, searchArchives: () => [] },
);
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
//（文件夹选择域在 selection store，clickItem 内部互斥清理，2026-07-28 复验反馈 5）。
// 2026-08-18 用户定：简介区（含 meta 行）点击 = 选中——名称+简介有底色框后用户预期整卡可点，
// 原".card-info 视为空白"（2026-07-29 对齐 Eagle 旧口径）废除，card-info 分类整体退役
function onCardClick(p: PlacementView, e: MouseEvent): void {
  if (dragView.suppressClick) return; // 有效拖拽结束后紧随的 click 不当选择（拖拽引擎置位）
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

// —— 分组（3.5 二轮，PRD 02-04）：仅文件夹/档案视图；分组 = 显示层归组（不改数据）——
// displayItems = 组拼接扁平序（未分组在头 + 定义序，组内保持 placementsFor 相对序——
// 新增区置顶 / 排序键效果自然带入组内）；gridSections = 组段边界 + 折叠态（布局偏移用）
const gridFolderId = computed(() => (props.viewKey.startsWith("folder:") ? props.viewKey.slice(7) : null));
const groupSections = computed(() => {
  if (props.searching) return null; // 3.6 搜索态：分组失效平铺（2026-08-19 用户定）
  if (!gridFolderId.value) return null;
  const defs = content.folderGroupsFor(props.viewKey);
  return defs ? groupPlacements(props.items, defs) : null;
});
const displayItems = computed(() =>
  groupSections.value ? groupSections.value.flatMap((s) => s.items) : props.items,
);
const gridSections = computed<GridSection[] | null>(() => {
  const secs = groupSections.value;
  const fid = gridFolderId.value;
  if (!secs || !fid) return null;
  let at = 0;
  return secs.map((s) => {
    const sec: GridSection = {
      groupId: s.groupId,
      name: s.name,
      start: at,
      end: at + s.items.length,
      collapsed: content.isGroupCollapsed(fid, s.groupId),
    };
    at += s.items.length;
    return sec;
  });
});

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
  groupHeaders,
  scrollTo, // 12 稿分组菜单「跳转」
} = useGridLayout({
  items: () => displayItems.value,
  sections: () => gridSections.value,
  viewKey: () => props.viewKey,
  viewportRef,
  trackRef,
  thumbRef,
  headerRef,
});

// 分组标题折叠 / 菜单（12 稿）：折叠态在 content store（会话级）；菜单由 groupHeaderSpec 出
function onGroupFold(groupId: string | null): void {
  if (gridFolderId.value) content.toggleGroupCollapsed(gridFolderId.value, groupId);
}
// 12 稿「跳转」：滚动到目标组标题（组标题层顶 = headerH + 12 + listExtra + h.top）
function scrollToGroup(groupId: string | null): void {
  const h = groupHeaders.value.find((x) => x.groupId === groupId);
  if (!h) return;
  scrollTo(h.top + headerH.value + 12 + listExtra.value);
}
// 12 稿分组菜单（150 宽；未分组 = 简化菜单——spec 内分流）
function onGroupMenu(h: { groupId: string | null }, e: MouseEvent): void {
  const fid = gridFolderId.value;
  if (!fid) return;
  openContextMenu(e, groupHeaderSpec({ folderId: fid, groupId: h.groupId, scrollToGroup }), undefined, { width: 150 });
}
// sticky 置顶分组标题（05_区块标题：滚到容器顶部停留，下一组接替）：transform 容器内 sticky 无效，
// 悬浮层手动实现——当前组 = 最后滚过视口顶的组；下一组标题接近 30 内时上移让位（顶走动画）
const stickyGroup = computed(() => {
  const hs = groupHeaders.value;
  if (hs.length === 0) return null;
  const off = headerH.value + 12 + listExtra.value;
  let cur: (typeof hs)[number] | null = null;
  let next: (typeof hs)[number] | null = null;
  for (let i = 0; i < hs.length; i++) {
    if (hs[i].top + off <= pos.value) cur = hs[i];
    else {
      next = hs[i];
      break;
    }
  }
  if (!cur) return null; // 第一组标题还在视口内——原位标题可见，不叠悬浮
  let shift = 0;
  if (next) {
    const nextY = next.top + off - pos.value;
    if (nextY < 30) shift = nextY - 30;
  }
  return { ...cur, shift, open: !content.isGroupCollapsed(gridFolderId.value ?? "", cur.groupId) };
});

// 收件箱视口（quick:inbox，4.4）：头部出"工程文件"磁盘实况区块，"内容"副标题改题"已导入"
const isInbox = computed(() => props.viewKey === "quick:inbox");

// "内容"副标题显隐（3.6 搜索态分支：文件夹区隐藏，命中档案区在时有档案才出标题作分隔）
const showContentHeader = computed(() => {
  if (props.items.length === 0) return false;
  if (props.searching) return isInbox.value || props.searchArchives.length > 0;
  return (
    isInbox.value ||
    content.stripFoldersFor(props.viewKey).length > 0 ||
    content.stripArchivesFor(props.viewKey).length > 0
  );
});

// 搜索空态（3.6，用户定）：搜索态且素材与档案皆无命中 → 中央「无搜索结果」
const searchEmpty = computed(
  () => props.searching && props.items.length === 0 && props.searchArchives.length === 0,
);

// 空格放大预览（lib/useSpaceZoom）：浮层态 + 悬停追踪；panelHover 滚动条悬停显隐共用
const { zoomAsset, panelHover, trackPointer, onGridMouseLeave, onZoomSpace } = useSpaceZoom({
  items: () => displayItems.value,
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
  // 头部（文件夹区块、"内容"副标题、工程文件、列表列头）/ 分组标题（原位 + sticky 悬浮层）/
  // 滚动条 / 按钮上不启动框选；
  // 卡片本体（含名称 / 简介区，整卡）由 gridPointerDown 先行分发去拖拽不会到此
  //（2026-08-18 用户定：card-info 空白口径退役，简介区不作框选起点）
  ignoreSelector: ".grid-header, .list-header, .grid-track, button, .group-header, .sticky-group-header",
  hitTest: makeBoxHitTest({
    boxes: () => boxes.value,
    offsetY: () => headerH.value + 12 + listExtra.value - pos.value,
    keyAt: (i) => `${displayItems.value[i].assetId}|${displayItems.value[i].folderId}`,
  }),
});

// —— 右键菜单（6.3 全量改造；1.3 一轮按新稿 02_布局 重写）：
// 统一调度在 lib/menuSpecs/gridDispatch.openGridContextMenu（素材 / 回收站 13 稿 / 空白新增菜单
// 均走全局宿主 lib/contextMenus；RecycleCardMenu 局部菜单 2026-08-11 废除）
function onCardContextMenu(e: MouseEvent): void {
  openGridContextMenu(e, props.items);
}

// —— 卡片间距排序落点解析器（C4a）：注册给拖拽引擎，算法主体在 lib/dragOperation makeGridResolver
// 分组模式下禁用排序落点（2026-08-19 用户定：组归属由规则定，拖拽排序无意义——拖入别组不改变归组）
const resolveGap = makeGridResolver({
  isFolderView: () => props.viewKey.startsWith("folder:") && !gridSections.value,
  layer: () => cardsLayerRef.value,
  boxes: () => boxes.value,
  keyAt: (i) => `${displayItems.value[i].assetId}|${displayItems.value[i].folderId}`,
});

// 间隙竖块 client 坐标（Teleport body 渲染）：layer 坐标 + 层 rect 实时换算；
// 拖拽中指针持续 move → resolver 每次重算 gap → 本 computed 随之刷新（滚动瞬间的短滞后随下一次 move 自愈）
const gapClient = computed(() => {
  const g = dragView.gap;
  if (!g) return null;
  const r = cardsLayerRef.value?.getBoundingClientRect();
  if (!r) return null;
  return { left: r.left + g.left, top: r.top + g.top, width: g.width, height: g.height };
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
        <!-- 当前视口无子文件夹时整个文件夹区块隐藏（阶段3修改定案；1.2 按本标签页 viewKey 取子文件夹）；
             搜索态（3.6）隐藏文件夹区块（命中只出素材与档案，2026-08-19 用户定） -->
        <FolderStrip v-if="!searching && content.stripFoldersFor(viewKey).length > 0" :view-key="viewKey" />
        <!-- 档案区（3.2 一轮，PRD 02-03 三区：文件夹 / 档案 / 文件）：无档案整块不渲染（同文件夹区口径）；
             搜索态 = 命中档案列表（nodes 数据源切换，快速访问视图后端不返回档案命中——档案区仅文件夹视图出） -->
        <ArchiveStrip
          v-if="searching ? searchArchives.length > 0 : content.stripArchivesFor(viewKey).length > 0"
          :view-key="viewKey"
          :nodes="searching ? searchArchives : undefined"
        />
        <!-- 收件箱视口（4.4）：磁盘实况"工程文件"区块在 FolderStrip 同位（快速访问不出文件夹区块）；
             搜索态隐藏（磁盘实况不入搜索范围——搜索搜库内素材） -->
        <InboxDiskSection v-if="isInbox && !searching && content.inboxDisk.length > 0" />
        <!-- "内容"副标题：无子文件夹（含档案）时隐藏（无需区分）；有子文件夹但无文件时同样隐藏（2026-07-24 用户定）。
             收件箱视口改题"已导入"（4.4）：无文件夹区块概念，只要有已导入内容就显示。
             分组不再强制出内容标题（2026-08-19 复验定：无文件夹视图里"内容"与组标题叠着多余；
             有文件夹/档案区块时保留——分隔文件夹区与内容区）；搜索态 = 有命中档案时才出（分隔档案区与内容区）。
             padding 统一 0（2026-08-08 用户定：仅目录树区块标题 left 8）；标题↔卡片间距 12（下方 translateY） -->
        <SectionHeader
          v-if="showContentHeader"
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
        <!-- 拖拽卡片间距落点（C4a）：Teleport 到 body 按 client 坐标画（2026-08-18 修复
             "首列前插提示线不可见 / 盖住卡片"——grid-viewport overflow 裁剪会把行首间隙
             （负 layer-x = 容器 16 留白区）整条裁掉，原 Math.max(0,…) 钳位又拉回盖住卡片） -->
        <!-- 分组标题（3.5 二轮）：canvas 内随滚动的原位标题（absolute 于 cards-layer 坐标系） -->
        <div
          v-for="h in groupHeaders"
          :key="h.groupId ?? '__ungrouped'"
          class="group-header"
          :style="{ top: `${h.top}px` }"
        >
          <SectionHeader
            group
            :title="h.name"
            :count="h.count"
            :open="!content.isGroupCollapsed(gridFolderId ?? '', h.groupId)"
            @update:open="onGroupFold(h.groupId)"
            @menu="onGroupMenu(h, $event)"
          />
        </div>
      </div>
    </div>
    <!-- 搜索空态（3.6，用户定：中央「无搜索结果」极淡档；pointer-events none 不挡空白右键 / 框选） -->
    <div v-if="searchEmpty" class="search-empty">无搜索结果</div>
    <!-- sticky 置顶分组标题（05_区块标题：滚过顶后停留，下一组接替顶走）；C1 实底盖卡片；
         贴 viewport 顶（2026-08-19 复验：原 top=headerH+12 悬在视口中部截断首行卡片）；
         列表模式让开钉顶列头（46+6=listExtra） -->
    <div
      v-if="stickyGroup && assetsOpen"
      class="sticky-group-header"
      :style="{ top: `${listExtra}px`, transform: `translateY(${stickyGroup.shift}px)` }"
    >
      <SectionHeader
        group
        :title="stickyGroup.name"
        :count="stickyGroup.count"
        :open="stickyGroup.open"
        @update:open="onGroupFold(stickyGroup.groupId)"
        @menu="onGroupMenu(stickyGroup, $event)"
      />
    </div>
    <Teleport to="body">
      <div
        v-if="gapClient"
        class="drop-gap"
        :style="{
          left: `${gapClient.left}px`,
          top: `${gapClient.top}px`,
          width: `${gapClient.width}px`,
          height: `${gapClient.height}px`,
        }"
      />
    </Teleport>
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
/* 搜索空态（3.6）：absolute 居中于 viewport（内容为空无滚动，天然居中） */
.search-empty {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-faint);
  font-size: var(--text-body-size);
  pointer-events: none;
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
/* 分组标题（3.5 二轮）：canvas 内原位标题——absolute 于 cards-layer 坐标系（top 由 groupHeaders 给），
   右缘 16 与卡片右缘对齐（布局 PAD.right）；随 canvas 滚动 */
.group-header {
  position: absolute;
  left: 0;
  right: 16px;
  height: 30px;
}
/* sticky 置顶分组标题（05_区块标题：滚到容器顶部停留，该组滚完下一组接替）：
   viewport 内绝对定位（不随 canvas transform）+ C1 实底盖滚过卡片；shift 位移 = 下一组顶走 */
.sticky-group-header {
  position: absolute;
  left: 0;
  right: 16px;
  height: 30px;
  background: var(--c1);
  z-index: 5;
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
   填充 30%（2026-08-10 用户定：12% 不够明显，与并入填充同档）；
   Teleport 到 body 后 fixed 定位（2026-08-18，脱离 grid-viewport 裁剪） */
.drop-gap {
  position: fixed;
  z-index: 5;
  pointer-events: none;
  border-radius: var(--radius-4);
  background: var(--brand-1-30);
  box-shadow: inset 0 0 0 1px var(--brand-1);
}
</style>
