<script setup lang="ts">
// 内容区素材网格：自适应布局 + 自定义 transform 虚拟滚动（踩坑 005 定案）。
// 卡片列表共用一个滚动（03_UI 布局/03_中间面板 03）：文件夹区块与"内容"副标题
// 作为 canvas 头部随网格一起滚动，网格 boxes 按头部高度偏移；
// 滚动条贴 viewport 右缘（不占布局），asset-section 右 padding 4 作呼吸。
import { computed, onBeforeUnmount, onMounted, ref, shallowRef, watch } from "vue";
import AssetCard from "./AssetCard.vue";
import HoverZoomOverlay from "./HoverZoomOverlay.vue";
import ListRowCard from "./ListRowCard.vue";
import ListHeader from "./ListHeader.vue";
import FolderStrip from "./FolderStrip.vue";
import InboxDiskSection from "./InboxDiskSection.vue";
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
import { useMarquee, makeBoxHitTest, headerBlankClear } from "../lib/useMarquee";
import {
  dragView,
  gridPointerDown,
  makeGridResolver,
  registerGridResolver,
  unregisterGridResolver,
  assetCardFade,
} from "../lib/dragOperation";
import { openContextMenu } from "../lib/contextMenus";
import { openGridContextMenu } from "../lib/menuSpecs";
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

// 空格放大预览（2026-08-10 用户定：访达 Quick Look 逻辑，取代放大镜按钮）：
// **悬停优先**——鼠标悬停的卡片（无需选中）空格即览，无悬停回退单选项；
// 浮层开着时：悬停在另一张卡上按空格 = 切换查看（同日四轮修复浮层不重载后恢复），
// 同卡 / 无悬停 = 关闭；Esc 由浮层自处理；选择跟随：开启期间单选切换换图、多选 / 清空关闭。
// 悬停判定：按键瞬间 elementFromPoint 取鼠标位命中（穿透 pointer-events:none 浮层，
// 拖拽引擎同款手法）。滚动条悬停显隐（同日）共用 panelHover。
const zoomAsset = ref<PlacementView | null>(null);
const panelHover = ref(false);
let lastPointer = { x: 0, y: 0 };
function findByKey(key: string | null): PlacementView | null {
  return key ? (props.items.find((i) => `${i.assetId}|${i.folderId}` === key) ?? null) : null;
}
function onGridPointerMove(e: PointerEvent): void {
  lastPointer = { x: e.clientX, y: e.clientY };
  onMarqueeMove(e);
}
function hoverKeyAtPoint(): string | null {
  const el = document.elementFromPoint(lastPointer.x, lastPointer.y);
  const card = el?.closest(".asset-card, .list-row");
  return card?.getAttribute("data-key") ?? null;
}
function onGridMouseLeave(): void {
  panelHover.value = false;
}
function onGridKeydown(e: KeyboardEvent): void {
  if (e.key !== " ") return;
  const t = e.target as HTMLElement;
  if (t instanceof HTMLInputElement || t instanceof HTMLTextAreaElement) return; // 重命名输入中不触发
  if (!props.panelActive) return; // window 级监听多 panel 重复注册：仅激活标签页响应（2026-08-10）
  // 浮层开着时交给其 window 级监听处理（同日：焦点不在网格时空格也要生效；此处不处理防双触发）
  if (zoomAsset.value) return;
  e.preventDefault();
  const p = findByKey(hoverKeyAtPoint()) ?? (selection.count === 1 ? findByKey(selection.first) : null);
  if (p) zoomAsset.value = p;
}
// 浮层空格（window 级，焦点任意）：**纯 toggle 再按即关**（2026-08-10 用户定二轮：
// 悬停切换删除——浮层遮挡大、用户不知切换功能时，想关闭却误换图反而困惑）
function onZoomSpace(): void {
  zoomAsset.value = null;
}
watch(
  () => [selection.first, selection.count],
  () => {
    if (!zoomAsset.value) return;
    if (selection.count !== 1) {
      zoomAsset.value = null;
      return;
    }
    zoomAsset.value = findByKey(selection.first);
  },
);

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
const isInbox = computed(() => props.viewKey === "quick:inbox");

// 总高 = 头部（文件夹区块 + 内容副标题 + 间距 12）+ 网格高（折叠"内容"时不计）。
// 偏移恒为 headerH + 12（与 cards-layer translateY 严格一致，2026-08-10 走查：
// 此前此处 +6 而卡片层 +12——判定 / 滚动区间与视觉错位 6px，间隙起框选把相邻行带入）
// 列表模式另有固定列头 46（figma 列表显示稿）+ 列头与列表间距 6（不占布局、不随滚动）
const listExtra = computed(() => (ui.layoutMode === "list" ? 52 : 0));

function currentTotal(): number {
  return (assetsOpen.value ? gridTotal.value : 0) + headerH.value + 12 + listExtra.value;
}

function updateRange(scrollPos: number): void {
  const vh = viewportRef.value?.clientHeight ?? 0;
  const off = headerH.value + 12;
  const rs = rows.value;
  r0.value = Math.max(0, lowerBound(rs, scrollPos - off - BUFFER) - 1);
  r1.value = Math.min(rs.length, lowerBound(rs, scrollPos + vh - off + BUFFER) + 1);
}

const { pos, thumbTop, thumbHeight, scrollable, dragging, scrollTo, setTotal, attach, detach } =
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
  // 重排锚点（2026-08-10 用户实测：缩放卡片 / 拖拆分条改宽后视图跳走——旧行为 pos 像素
  // 原样保留，行数全变后同像素对应完全不同内容）：重排前记录视口顶所在行首张卡片 key +
  // 行内偏移比例，重排后滚回该卡新位置（Eagle 式内容锚定）；
  // restorePos（切视口恢复）优先，视口顶仍在头部区块（layerY ≤ 0）时不锚
  let anchorKey: string | null = null;
  let anchorRatio = 0;
  if (restorePos === null && rows.value.length > 0) {
    const off = headerH.value + 12 + listExtra.value;
    const layerY = pos.value - off;
    if (layerY > 0) {
      const rs = rows.value;
      const ri = Math.max(0, lowerBound(rs, layerY) - 1);
      const row = rs[ri];
      const first = props.items[boxes.value[row.start].i];
      anchorKey = `${first.assetId}|${first.folderId}`;
      anchorRatio = row.height > 0 ? (layerY - row.top) / row.height : 0;
    }
  }
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
  setTotal(currentTotal());
  // 滚动位置：恢复（进出预览 / 切换目录）优先；否则回锚点；锚点卡已消失（被删等）维持重夹
  if (restorePos !== null) {
    scrollTo(restorePos);
    restorePos = null;
  } else if (anchorKey !== null) {
    const idx = props.items.findIndex((it) => `${it.assetId}|${it.folderId}` === anchorKey);
    const b = idx >= 0 ? boxes.value.find((bb) => bb.i === idx) : undefined;
    if (b) {
      // box.height = 缩略图 + 简介 = 行高（四种布局同构），按比例还原行内偏移
      scrollTo(b.top + headerH.value + 12 + listExtra.value + anchorRatio * b.height);
    }
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
    scale: () => 1, // scale 补偿机制已废（2026-08-10，恒 1）
    offsetY: () => headerH.value + 12 + listExtra.value - pos.value,
    keyAt: (i) => `${props.items[i].assetId}|${props.items[i].folderId}`,
  }),
});

// —— 右键菜单（6.3 全量改造；1.3 一轮按新稿 02_布局 重写）：
// 统一调度在 lib/menuSpecs.openGridContextMenu（素材 / 回收站 13 稿 / 空白新增菜单均走全局宿主
// lib/contextMenus；RecycleCardMenu 局部菜单 2026-08-11 废除）
function onCardContextMenu(e: MouseEvent): void {
  openGridContextMenu(e, props.items);
}

// —— 卡片间距排序落点解析器（C4a）：注册给拖拽引擎，算法主体在 lib/dragOperation makeGridResolver
const resolveGap = makeGridResolver({
  isFolderView: () => props.viewKey.startsWith("folder:"),
  layer: () => cardsLayerRef.value,
  scale: () => 1, // scale 补偿机制已废（2026-08-10，恒 1）
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

// 宽度变化 = 逐帧实时重排（Obsidian 式 reflow，2026-08-08 收展定案 v3，2026-08-10 推广到
// 全部宽度变化）：虚拟滚动下 relayout 只更新可见卡片 transform，每帧跑得起；
// RO 回调在当帧 rAF 之后、绘制之前执行（踩坑 025），重排与可见卡片 transform 同帧提交不闪。
// scale 视觉补偿 + 停顿 240ms 旧机制废除（2026-08-10 用户实测：补偿期双轴缩放整页"滚动"
// 与滚动锚点归位反复拉锯——拉走→弹回循环）。
let lastW = 0;
let lastH = 0;
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
  if (widthChanged) relayout();
  else setTotal(currentTotal());
});
// 收展动画结束：按最终宽度对齐重排一次
watch(
  () => ui.sidebarAnimating,
  (animating) => {
    if (!animating) relayout();
  },
);

// 切换目录 / 视图（1.2：本标签页 viewKey 变化——dockview updateParameters 驱动）：
// 保存旧视口位置、登记新视口待恢复（随后 items watch 重排完成时恢复。
// 注册顺序必须在 items watch 之前——同 tick 内按注册顺序触发，restorePos 要先就位）
watch(
  () => props.viewKey,
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
  // 空格预览走 window 级（2026-08-10：焦点不在网格时也要能打开；active 闸防多 panel 重复响应）
  window.addEventListener("keydown", onGridKeydown);
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
  // 预览返回 / 视口初次挂载：恢复上次滚动位置（2026-07-25 用户定；1.2 key = 本标签页 viewKey）
  restorePos = content.getScrollPos(props.viewKey);
  relayout();
});

onBeforeUnmount(() => {
  // 卸载（进预览 / 关闭标签页等）前保存本视口滚动位置
  content.saveScrollPos(props.viewKey, pos.value);
  viewportRef.value?.removeEventListener("wheel", onCtrlWheel);
  window.removeEventListener("keydown", onGridKeydown);
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
        <!-- 收件箱视口（4.4）：磁盘实况"工程文件"区块在 FolderStrip 同位（快速访问不出文件夹区块） -->
        <InboxDiskSection v-if="isInbox && content.inboxDisk.length > 0" />
        <!-- "内容"副标题：无子文件夹时隐藏（无需区分）；有子文件夹但无文件时同样隐藏（2026-07-24 用户定）。
             收件箱视口改题"已导入"（4.4）：无文件夹区块概念，只要有已导入内容就显示。
             padding 统一 0（2026-08-08 用户定：仅目录树区块标题 left 8）；标题↔卡片间距 12（下方 translateY） -->
        <SectionHeader
          v-if="items.length > 0 && (isInbox || content.stripFoldersFor(viewKey).length > 0)"
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
