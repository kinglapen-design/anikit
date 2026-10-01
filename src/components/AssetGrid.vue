<script setup lang="ts">
// 内容区素材网格：自适应布局 + 自定义 transform 虚拟滚动（踩坑 005 定案）。
// 卡片列表共用一个滚动（03_UI 布局/03_中间面板 03）：文件夹区块与"内容"副标题
// 作为 canvas 头部随网格一起滚动，网格 boxes 按头部高度偏移；
// 滚动条贴 viewport 右缘（不占布局），asset-section 右 padding 4 作呼吸。
// 布局 / 虚拟滚动编排在 lib/useGridLayout，空格放大预览在 lib/useSpaceZoom（2026-08-12 拆出）。
import { computed, inject, onBeforeUnmount, onMounted, ref, watch } from "vue";
import AssetCard from "./AssetCard.vue";
import HoverZoomOverlay from "./HoverZoomOverlay.vue";
import ListRowCard from "./ListRowCard.vue";
import ListHeader from "./ListHeader.vue";
import FolderStrip from "./FolderStrip.vue";
import ArchiveStrip from "./ArchiveStrip.vue";
import InboxDiskSection from "./InboxDiskSection.vue";
import EmptyFolderState from "./EmptyFolderState.vue";
import QuickNavButton from "./QuickNavButton.vue";
import SectionTitle from "./items/SectionTitle.vue";
import { FREE_EMPTY_H, useGridLayout, type GridSection } from "../lib/useGridLayout";
import { groupPlacements } from "../lib/grouping";
import { isAudioExt, isSequence, isTextExt, isThumbCapableExt } from "../lib/formatBadge";
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
import { usePickerStore } from "../stores/picker";
import { useSelectionStore } from "../stores/selection";
import { useUiStore } from "../stores/ui";
import { PREVIEW_SESSION_KEY } from "../lib/previewSessionKey";

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
const picker = usePickerStore();
const selection = useSelectionStore();
// 本 panel 预览会话（ContentArea provide；双击进预览与右键菜单 panelId 两用）
const preview = inject(PREVIEW_SESSION_KEY)!;

// 进预览（4.3）：双击卡片
function openPreview(p: PlacementView): void {
  void preview.open(`${p.assetId}|${p.folderId}`);
}

// 卡片点击选择（6.1，2026-07-28 用户定案）：单击 = 单选 / Ctrl+单击 = 加选切换 /
// Shift+单击 = 锚点范围选（selection store 三态）；素材卡片选中与文件夹 / 收件箱条目互斥
//（文件夹选择域在 selection store，clickItem 内部互斥清理，2026-07-28 复验反馈 5）。
// 2026-08-18 用户定：简介区（含 meta 行）点击 = 选中——名称+简介有底色框后用户预期整卡可点，
// 原".card-info 视为空白"（2026-07-29 对齐 Eagle 旧口径）废除，card-info 分类整体退役
// 选择模式（6.2 一轮）：单击 = 加入/移出备选（不再是选中，PRD 03_数据链接 04 交互变化）；
// cover 用途（2026-09-08）= 单击图片即定（单选直回不进备选区，picker.pickCoverAsset）
function onCardClick(p: PlacementView, e: MouseEvent): void {
  if (dragView.suppressClick) return; // 有效拖拽结束后紧随的 click 不当选择（拖拽引擎置位）
  if (picker.active) {
    if (content.isRecycleView) return; // 回收站文件不可加入备选（已删除资产不该建关系）
    // 工作空间合成项（PRD 02 步骤 1，assetId = ws: 假 id）严禁进备选——确认会发 assetId 类命令；
    // 选择模式树导航可切到工作空间（白名单放行），此处兜底拦点选
    if (content.isWorkspaceView) return;
    const target = { kind: "asset" as const, id: p.assetId, name: p.name, ext: p.ext, thumb: p.thumbPath, folderId: p.folderId, origin: p.origin };
    if (picker.purpose === "cover") {
      void picker.pickCoverAsset(target);
      return;
    }
    // thumb 伴生缩略图（2026-09-23）：同 cover 单选直回范式——点图即绑定发起工程
    if (picker.purpose === "thumb") {
      void picker.pickThumbAsset(target);
      return;
    }
    // prompt 指定提示词（提示词轮）：同 cover/thumb 单选直回——点文本卡即对目标集逐个指定
    if (picker.purpose === "prompt") {
      void picker.pickPromptAsset(target);
      return;
    }
    // folderId 随三元组定位源 Placement（link 用途确认时 link_placements 入参）
    picker.togglePick(target);
    return;
  }
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
const displayItems = computed(() => {
  const items = groupSections.value ? groupSections.value.flatMap((s) => s.items) : props.items;
  // 选择模式链接音频（6.2 三轮）：过滤非音频（02 稿"过滤掉非音频文件，去掉噪音，加快选择"）；
  // 选择模式档案封面（2026-09-08）：过滤非封面图——静态图 + gif 动图（isThumbCapableExt，
  // 与烘培可解码格式同口径），排除序列帧包（origin 指向 frames/ 内帧非实体单图，不可烘培）；
  // 选择模式指定提示词（提示词轮）：过滤非文本——txt/md（isTextExt）
  if (picker.filterAudio) return items.filter((p) => isAudioExt(p.ext));
  if (picker.filterImage) return items.filter((p) => isThumbCapableExt(p.ext) && !isSequence(p));
  if (picker.filterText) return items.filter((p) => isTextExt(p.ext));
  return items;
});
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
      free: s.free, // 自由组（2026-09-17 无规则空组）：空组恒渲染占位
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
  smoothScrollTo, // 快速到顶 / 到底圆钮
} = useGridLayout({
  items: () => displayItems.value,
  sections: () => gridSections.value,
  viewKey: () => props.viewKey,
  viewportRef,
  trackRef,
  thumbRef,
  headerRef,
  onUserScroll: onQuickNavScroll,
});

// —— 快速到顶 / 到底（2026-09-23 用户定）：右下角圆钮，往下持续滚动出「滚到底」、往上持续出「回到顶」；
// 同向累计一屏行高（200px≈滚轮两下）触发，停止滚动 1.5s 收起，点击平滑滚动到位并收起 ——
const quickNav = ref<"top" | "bottom" | null>(null);
let navDir = 0;
let navAcc = 0;
let navHideTimer: ReturnType<typeof setTimeout> | undefined;

function onQuickNavScroll(delta: number): void {
  if (!scrollable.value) return;
  const dir = Math.sign(delta);
  if (dir !== navDir) {
    navDir = dir;
    navAcc = 0;
  }
  navAcc += Math.abs(delta);
  if (navAcc >= 200) quickNav.value = dir > 0 ? "bottom" : "top";
  if (quickNav.value) {
    clearTimeout(navHideTimer);
    navHideTimer = setTimeout(() => (quickNav.value = null), 1500);
  }
}
function onQuickNavClick(): void {
  if (!quickNav.value) return;
  smoothScrollTo(quickNav.value === "top" ? 0 : Number.MAX_SAFE_INTEGER);
  quickNav.value = null;
  clearTimeout(navHideTimer);
}
// 切视口收起圆钮（方向累积一并清，不带入新视口）
watch(
  () => props.viewKey,
  () => {
    quickNav.value = null;
    navDir = 0;
    navAcc = 0;
    clearTimeout(navHideTimer);
  },
);

// 分组标题折叠 / 菜单（12 稿）：折叠态在 content store（会话级）；菜单由 groupHeaderSpec 出
//（SectionTitle 折叠点击守卫在调用方，§20——dragView.suppressClick 吞拖拽后紧随 click）
function onGroupFold(groupId: string | null): void {
  if (dragView.suppressClick) return;
  if (gridFolderId.value) content.toggleGroupCollapsed(gridFolderId.value, groupId);
}
// "内容"区块标题折叠（同守卫口径）
function onAssetsFold(): void {
  if (dragView.suppressClick) return;
  assetsOpen.value = !assetsOpen.value;
}
// 12 稿「跳转」：滚动到目标组标题（组标题层顶 = headerH + 12 + listExtra + h.top）
function scrollToGroup(groupId: string | null): void {
  const h = groupHeaders.value.find((x) => x.groupId === groupId);
  if (!h) return;
  scrollTo(h.top + headerH.value + 12 + listExtra.value);
}
// 12 稿分组菜单（未分组 = 简化菜单——spec 内分流）
function onGroupMenu(h: { groupId: string | null }, e: MouseEvent): void {
  const fid = gridFolderId.value;
  if (!fid) return;
  openContextMenu(e, groupHeaderSpec({ folderId: fid, groupId: h.groupId, scrollToGroup }), undefined);
}
// 空自由组提示框清单（2026-09-17 无规则空组）：展开态且成员为空的自由组——占位提示即拖入落点
const emptyFreeHeaders = computed(() =>
  groupHeaders.value.filter(
    (h) => h.free && h.count === 0 && !content.isGroupCollapsed(gridFolderId.value ?? "", h.groupId),
  ),
);
// sticky 置顶分组标题（05_区块标题：滚到容器顶部停留，下一组接替）：transform 容器内 sticky 无效，
// 悬浮层手动实现——当前组 = 最后滚过视口顶的组；下一组标题接近 26（组标题稿高）内时上移让位（顶走动画）
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
    if (nextY < 26) shift = nextY - 26;
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

// 空文件夹空态（7.1 内容容器轮，稿 = 中间面板-内容容器-空文件夹）：非搜索态且三区皆空
//（文件夹 / 档案 / 文件全无）→ 中央插画 + 导入双钮；回收站 / 收件箱视口不出（语义不合），
// 范围 = folder: 视口 + quick:uncategorized（2026-09-09 收窄：全部禁导入——不是位置，
// 空库导入双钮撤到未分类 = 候车区总入口）
const folderEmpty = computed(() => {
  if (props.searching || props.items.length > 0) return false;
  if (isInbox.value || content.isRecycleView) return false;
  if (!props.viewKey.startsWith("folder:") && props.viewKey !== "quick:uncategorized") return false;
  if (content.stripFoldersFor(props.viewKey).length > 0) return false;
  if (content.stripArchivesFor(props.viewKey).length > 0) return false;
  return true;
});

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
  const next = ui.rowHeightFor(props.viewKey) + (e.deltaY > 0 ? -10 : 10);
  ui.setRowHeight(props.viewKey, Math.max(80, Math.min(max, next))); // 按视口记忆（2026-09-13 走查）
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
// 均走全局宿主 lib/contextMenus；RecycleCardMenu 局部菜单 2026-08-11 废除）；
// panelId 透传 = 本 panel 预览会话属主（「打开预览」开进本 panel 会话，2026-09-25）
function onCardContextMenu(e: MouseEvent): void {
  openGridContextMenu(e, props.items, preview.panelId);
}

// —— 卡片间距排序落点解析器（C4a）：注册给拖拽引擎，算法主体在 lib/dragOperation makeGridResolver
// 分组模式（2026-08-20 用户定，替换 2026-08-19"分组整体禁排"）：组内允许互相排序——落点经
// groupOfKey 组约束（同组才出落点；跨分组禁止，组归属由规则定、拖过去不改变归组）
const keyGroupMap = computed(() => {
  const m = new Map<string, string | null>();
  const secs = groupSections.value;
  if (secs) {
    for (const s of secs) {
      for (const p of s.items) m.set(`${p.assetId}|${p.folderId}`, s.groupId);
    }
  }
  return m;
});
const resolveGap = makeGridResolver({
  isFolderView: () => props.viewKey.startsWith("folder:"),
  layer: () => cardsLayerRef.value,
  boxes: () => boxes.value,
  keyAt: (i) => `${displayItems.value[i].assetId}|${displayItems.value[i].folderId}`,
  groupOfKey: (key) => (groupSections.value ? keyGroupMap.value.get(key) : undefined),
});

// 间隙竖块 client 坐标（Teleport body 渲染）：layer 坐标 + 层 rect 实时换算；
// 拖拽中指针持续 move → resolver 每次重算 gap → 本 computed 随之刷新；pos 纳入依赖——transform
// 滚动后层 rect 随 pos 变（滚轮 / 拖拽边缘自动滚动），提示线随滚动跟随不滞留。
// 非激活 panel 不渲染（2026-09-13 实机走查修复"提示线跑到列表盒子外"）：dockview 非激活标签页
// 不卸载仅 display:none，隐藏实例的层 rect 全零 → 竖块会按 layer 原始坐标画到窗口左上盒子外。
// 钳制口径（同日四轮定稿）：**纵向求交、横向不钳**——纵向：Teleport 无 overflow 裁剪，滚出视口的
// 间隙行会把竖块画到工具栏 / 标签栏上，求交后不渲染；横向：行首间隙 g.left=-GAP 落在宿主
//（列表容器 padding-left 12）留白带是设计口径（与右侧 PAD.right 对称，"和右侧一样在边距处"），
// 钳进视口反而盖住首卡（三轮"最左侧提示没了 / 覆盖卡片"两轮试错定稿）
const gapClient = computed(() => {
  const g = dragView.gap;
  // 只渲染归属网格的提示线（2026-09-17 分屏跨视口改造，替代 panelActive 门控——
  // 跨分屏拖拽时目标 pane 非激活也要画线；dockview 隐藏标签页永不持有 gapGrid 天然不画）
  if (!g || dragView.gapGrid !== gridToken.value) return null;
  void pos.value;
  const r = cardsLayerRef.value?.getBoundingClientRect();
  const vr = viewportRef.value?.getBoundingClientRect();
  if (!r || !vr) return null;
  const top = Math.max(r.top + g.top, vr.top);
  const bottom = Math.min(r.top + g.top + g.height, vr.bottom);
  if (bottom <= top) return null;
  return { left: r.left + g.left, top, width: g.width, height: bottom - top };
});

// 就地重命名（C2）：滚出虚拟滚动销毁 = 取消——重命名卡片离开可视渲染集时清 renamingKey
//（卡片组件 blur 晚于本清理到达时按 key 失配判取消，见 AssetCard.commitRename）
watch(visibleCards, (cards) => {
  const key = selection.renamingKey;
  if (key && !cards.some((c) => `${c.asset.assetId}|${c.asset.folderId}` === key)) {
    selection.renamingKey = null;
  }
});

// 拖拽卡片间距落点解析（C4）+ 分屏跨视口落点（2026-09-17 实机走查「标签分屏后文件要能跨分屏拖拽」）：
// 注册表多网格共存——**不再按 panelActive 注册**（跨 pane 落点需要每个可见网格都在册；
// dockview 隐藏标签页层 rect 全零，命中判定天然不中）；token 归属本实例，卸载只清自己；
// visibleKeys = 网格可见序（组拼接序——显示排序物化以它为准）；scroll 供边缘自动滚动；
// layer = **视口根**（pane 级落点命中区——2026-10-01 走查修复：原传卡片层只到内容高，
// 下半视口不触发 pane 高亮；间隙解析的卡片层坐标系在 makeGridResolver 自持，不受影响）；
// dropFolder = 并入移动目标（folder: → id / 未分类 root / 回收站 / ws / 虚拟视图 null 不落）
const gridToken = ref(0);
onMounted(() => {
  viewportRef.value!.addEventListener("wheel", onCtrlWheel, { passive: false });
  gridToken.value = registerGridResolver({
    resolve: resolveGap,
    visibleKeys: () => displayItems.value.map((p) => `${p.assetId}|${p.folderId}`),
    scroll: { viewport: () => viewportRef.value, scrollTo, pos: () => pos.value },
    layer: () => viewportRef.value,
    dropFolder: () => {
      const vk = props.viewKey;
      if (vk.startsWith("folder:")) return vk.slice(7);
      if (vk === "quick:uncategorized") return "root";
      if (vk === "quick:recycle") return "recycle";
      if (vk.startsWith("ws:")) return vk;
      return null;
    },
  });
});

onBeforeUnmount(() => {
  viewportRef.value?.removeEventListener("wheel", onCtrlWheel);
  if (gridToken.value) unregisterGridResolver(gridToken.value);
  clearTimeout(navHideTimer);
});
</script>

<template>
  <div
    ref="viewportRef"
    class="grid-viewport"
    :class="{ 'pane-drop': dragView.paneDrop === gridToken }"
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
        <FolderStrip v-if="!searching && content.stripFoldersFor(viewKey).length > 0" :view-key="viewKey" :panel-active="panelActive" />
        <!-- 档案区（3.2 一轮，PRD 02-03 三区：文件夹 / 档案 / 文件）：无档案整块不渲染（同文件夹区口径）；
             搜索态 = 命中档案列表（nodes 数据源切换，快速访问视图后端不返回档案命中——档案区仅文件夹视图出）；
             选择模式链接音频 / 档案封面 / 指定提示词（档案非可选目标）档案区不渲染（过滤噪音） -->
        <ArchiveStrip
          v-if="!picker.filterAudio && !picker.filterImage && !picker.filterText && (searching ? searchArchives.length > 0 : content.stripArchivesFor(viewKey).length > 0)"
          :view-key="viewKey"
          :nodes="searching ? searchArchives : undefined"
        />
        <!-- 收件箱视口（4.4）：磁盘实况"工程文件"区块在 FolderStrip 同位（快速访问不出文件夹区块）；
             搜索态隐藏（磁盘实况不入搜索范围——搜索搜库内素材） -->
        <InboxDiskSection v-if="isInbox && !searching && content.inboxDisk.length > 0" />
        <!-- "内容"副标题（7.1 内容容器轮 = SectionTitle block 贴稿，稿无动作钮 → actions=false）：
             无子文件夹（含档案）时隐藏（无需区分）；有子文件夹但无文件时同样隐藏（2026-07-24 用户定）。
             收件箱视口改题"已导入"（4.4）：无文件夹区块概念，只要有已导入内容就显示。
             分组不再强制出内容标题（2026-08-19 复验定）；有文件夹/档案区块时保留（分隔文件夹区与内容区）；
             搜索态 = 有命中档案时才出。标题↔卡片间距 12（下方 translateY） -->
        <SectionTitle
          v-if="showContentHeader"
          variant="block"
          :title="isInbox ? '已导入' : '内容'"
          :count="items.length"
          :expanded="assetsOpen"
          :actions="false"
          hot-zone="lead"
          @toggle="onAssetsFold"
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
        <!-- 分组标题（3.5 二轮；7.1 内容容器轮 = SectionTitle group 贴稿）：canvas 内随滚动的原位标题
             （absolute 于 cards-layer 坐标系）；more = 分组菜单，add 点亮不接业务；
             data-group-drop（2026-09-17 自由组 / 未分组段）：素材拖入 = 移动至分组（"" = 移出） -->
        <div
          v-for="h in groupHeaders"
          :key="h.groupId ?? '__ungrouped'"
          class="group-header"
          :class="{ 'drop-into': dragView.groupDrop !== null && (h.groupId ?? '') === dragView.groupDrop }"
          :style="{ top: `${h.top}px` }"
          :data-group-drop="h.free || h.groupId === null ? (h.groupId ?? '') : undefined"
        >
          <SectionTitle
            variant="group"
            :title="h.name"
            :count="h.count"
            :expanded="!content.isGroupCollapsed(gridFolderId ?? '', h.groupId)"
            hot-zone="lead"
            @toggle="onGroupFold(h.groupId)"
            @more="onGroupMenu(h, $event)"
          />
        </div>
        <!-- 空自由组提示框（2026-09-17 无规则空组）：小高度占位 + 居中提示，本身即拖入落点 -->
        <div
          v-for="h in emptyFreeHeaders"
          :key="`free-hint-${h.groupId}`"
          class="free-empty-hint"
          :class="{ 'drop-into': dragView.groupDrop === (h.groupId ?? '') }"
          :style="{ top: `${h.top + 38}px`, height: `${FREE_EMPTY_H}px` }"
          :data-group-drop="h.groupId ?? ''"
        >
          将文件拖入分组
        </div>
      </div>
    </div>
    <!-- 搜索空态（3.6，用户定：中央「无搜索结果」极淡档；pointer-events none 不挡空白右键 / 框选） -->
    <div v-if="searchEmpty" class="search-empty">无搜索结果</div>
    <!-- 空文件夹空态（7.1 内容容器轮贴稿）：三区皆空 → 中央插画 + 导入双钮（组件根穿透，仅钮可点） -->
    <EmptyFolderState v-if="folderEmpty" />
    <!-- sticky 置顶分组标题（05_区块标题：滚过顶后停留，下一组接替顶走）；C1 实底盖卡片；
         贴 viewport 顶（2026-08-19 复验：原 top=headerH+12 悬在视口中部截断首行卡片）；
         列表模式让开钉顶列头（34+6=listExtra，6px 缝由 CSS 零模糊阴影补盖——见样式注） -->
    <div
      v-if="stickyGroup && assetsOpen"
      class="sticky-group-header"
      :style="{ top: `${listExtra}px`, transform: `translateY(${stickyGroup.shift}px)` }"
    >
      <SectionTitle
        variant="group"
        :title="stickyGroup.name"
        :count="stickyGroup.count"
        :expanded="stickyGroup.open"
        hot-zone="lead"
        @toggle="onGroupFold(stickyGroup.groupId)"
        @more="onGroupMenu(stickyGroup, $event)"
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
    <!-- 快速到顶 / 到底圆钮（2026-09-23 用户定）：右下角，持续同向滚动出现 -->
    <QuickNavButton :dir="quickNav" @click="onQuickNavClick" />
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
/* 分屏跨视口并入落点（2026-09-17 实机走查「标签分屏后文件要能跨分屏拖拽」）：
   目标 pane 级高亮 = 并入同款——品牌/半透明填充 + 2px 品牌描边（inset 不占布局，§20 口径）。
   ::after 覆盖层绘制（pointer-events none 不挡卡片 / 滚动条）：右收 12 / 底收 12——视口几何
   右 0 底 0（asset-section 右 0 让滚动条贴边框、底 0），与 section 承担的左 12 / 上 12 对齐成
   四边等距框（2026-10-01 走查：高亮左有边距右贴边，左右不统一） */
.grid-viewport.pane-drop::after {
  content: "";
  position: absolute;
  inset: 0 var(--n-space-12) var(--n-space-12) 0;
  background: var(--n-brand-50);
  box-shadow: inset 0 0 0 2px var(--n-brand);
  pointer-events: none;
  z-index: 10;
}
/* 搜索空态（3.6）：absolute 居中于 viewport（内容为空无滚动，天然居中） */
.search-empty {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--n-l4);
  font-size: var(--n-fs-body);
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
  gap: var(--n-space-24);
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
   高 26（7.1 内容容器轮 SectionTitle group 稿值）；右缘 12 与卡片右缘对齐（布局 PAD.right）；随 canvas 滚动 */
.group-header {
  position: absolute;
  left: 0;
  right: 12px;
  height: 26px;
  border-radius: var(--n-radius-6);
}
/* 自由组 / 未分组段标题拖入落点（2026-09-17）：并入同款——品牌/半透明填充 + 2px 品牌描边（§20 口径） */
.group-header.drop-into {
  background: var(--n-brand-50);
  box-shadow: inset 0 0 0 2px var(--n-brand);
}
/* 空自由组提示框（2026-09-17 无规则空组）：小高度占位 + 居中提示（辅助说明文字档）；
   本身即拖入落点；左缘 0 右缘 12 与卡片列对齐 */
.free-empty-hint {
  position: absolute;
  left: 0;
  right: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--n-radius-6);
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l3);
  user-select: none;
}
.free-empty-hint.drop-into {
  background: var(--n-brand-50);
  box-shadow: inset 0 0 0 2px var(--n-brand);
}
/* sticky 置顶分组标题（05_区块标题：滚到容器顶部停留，该组滚完下一组接替）：
   viewport 内绝对定位（不随 canvas transform）+ g1 实底盖滚过卡片；shift 位移 = 下一组顶走。
   列表模式钉顶列头（34 高）与本标题（top = listExtra = 40）间的 6px 行距缝会漏滚动行
   （2026-10-02 用户实机报，截图行从缝间探出）——零模糊阴影向上补 6px 同色遮盖（§7 零模糊口径），
   标题文本站位不动；阴影随 shift 让位同上移，[34,66] 全程连续覆盖（上移部分由钉顶列头盖住） */
.sticky-group-header {
  position: absolute;
  left: 0;
  right: 12px;
  height: 26px;
  background: var(--n-g1);
  box-shadow: 0 -6px 0 var(--n-g1);
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
  border-radius: var(--n-radius-pill);
  /* 滚动条色档定案（2026-09-06 用户定，2026-09-13 实机走查扫尾旧 token）：6px 胶囊紧贴
     右边框 / 实色——常态 通用/四级、悬停 通用/六级（旧 C4/C5 在 g3 底上几乎不可见，全废） */
  background: var(--n-g4);
  transition: background 0.12s ease;
}
.grid-thumb:hover {
  background: var(--n-g6);
}
/* 框选矩形（6.1）：品牌色-半透明填充 + 品牌色-常态描边（2026-08-30 用户实机统一口径——
   与拖拽插入提示同色系，旧 --brand-1-12/--brand-1 青绿系随新品牌蓝色换代）；
   滚动条（z 10）之下、卡片之上 */
.marquee {
  position: absolute;
  z-index: 5;
  pointer-events: none;
  background: var(--n-brand-50);
  box-shadow: inset 0 0 0 1px var(--n-brand);
}
/* 拖拽卡片间距落点色块（C4a，2026-07-28 定案）：圆角竖向矩形填间距，同框选配色
   （品牌-常态描边 + 品牌-半透明填充，2026-08-30 统一）；
   Teleport 到 body 后 fixed 定位（2026-08-18，脱离 grid-viewport 裁剪） */
.drop-gap {
  position: fixed;
  z-index: 5;
  pointer-events: none;
  border-radius: var(--n-radius-4);
  background: var(--n-brand-50);
  box-shadow: inset 0 0 0 1px var(--n-brand);
}
</style>
