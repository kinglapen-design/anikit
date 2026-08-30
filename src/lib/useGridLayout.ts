// 网格布局 composable（2026-08-12 自 AssetGrid 拆出，规范第 3 条行数控制）：
// 虚拟滚动接线（useVirtualScroll + updateRange）+ 四布局重排（重排锚点 / 滚动位置恢复）+
// 可见卡片窗口（visibleCards）+ 尺寸观察（视口 / 头部 RO）与重排触发 watch 编排。
// 2026-08-19（3.5 二轮）分组支持：sections 非空时逐组独立布局拼全局坐标（组标题 26 + 间距累加偏移，
// 组间 24 / 末组 PAD.bottom——复验定案；7.1 内容容器轮组标题高 30→26 贴 SectionTitle group 稿值），
// groupHeaders 输出各组标题位置供模板渲染（sticky 置顶在 AssetGrid 悬浮层做——
// transform 容器内 sticky 无效）。
import { computed, onBeforeUnmount, onMounted, ref, shallowRef, watch, type Ref } from "vue";
import {
  PAD,
  computeJustified,
  computeWaterfall,
  computeGrid,
  computeList,
  lowerBound,
  type Box,
  type LayoutResult,
  type Row,
} from "./layouts";
import { useVirtualScroll } from "./useVirtualScroll";
import { useContentStore, type PlacementView } from "../stores/content";
import { useUiStore } from "../stores/ui";

/** 分组段（3.5 二轮）：displayItems 扁平数组上的 [start, end) 区间 + 组信息 */
export interface GridSection {
  groupId: string | null; // null = 未分组
  name: string;
  start: number;
  end: number;
  collapsed: boolean;
}

/** 组标题（模板渲染用）：top = 组标题在 cards-layer 坐标系的 y */
export interface GroupHeaderInfo {
  groupId: string | null;
  name: string;
  count: number;
  top: number;
}

export function useGridLayout(opts: {
  items: () => PlacementView[];
  viewKey: () => string;
  // 3.5 二轮分组：非空 = 分组模式（items 须与 sections 区间一致）；null/缺省 = 单段网格
  sections?: () => GridSection[] | null;
  viewportRef: Ref<HTMLElement | undefined>;
  trackRef: Ref<HTMLElement | undefined>;
  thumbRef: Ref<HTMLElement | undefined>;
  headerRef: Ref<HTMLElement | undefined>;
}) {
  const ui = useUiStore();
  const content = useContentStore();

  // 布局算法只认数值宽高：比例跟缩略图实际尺寸走（5.4——自定义缩略图后自适应，刷新回退）；
  // 回退链 thumbW → w（源文件尺寸，导入时与缩略图同比例）→ 300 方盒（fla 等无尺寸资源，4.2 用户定）
  const layoutItems = computed(() => opts.items().map((it) => ({ w: it.thumbW ?? it.w ?? 300, h: it.thumbH ?? it.h ?? 300 })));

  const BUFFER = 800;

  const boxes = shallowRef<Box[]>([]);
  const rows = shallowRef<Row[]>([]);
  const r0 = ref(0);
  const r1 = ref(0);
  const headerH = ref(0);
  const gridTotal = ref(0);
  const assetsOpen = ref(true);
  // 分组标题（3.5 二轮）：空 = 不分组
  const groupHeaders = shallowRef<GroupHeaderInfo[]>([]);

  // 总高 = 头部（文件夹区块 + 内容副标题 + 间距 12）+ 网格高（折叠"内容"时不计）。
  // 偏移恒为 headerH + 12（与 cards-layer translateY 严格一致，2026-08-10 走查：
  // 此前此处 +6 而卡片层 +12——判定 / 滚动区间与视觉错位 6px，间隙起框选把相邻行带入）
  // 列表模式另有固定列头 34（7.1 内容容器轮贴稿——组件/卡片/列表表头）+ 列头与列表间距 6（不占布局、不随滚动）
  const listExtra = computed(() => (ui.layoutMode === "list" ? 40 : 0));

  function currentTotal(): number {
    return (assetsOpen.value ? gridTotal.value : 0) + headerH.value + 12 + listExtra.value;
  }

  function updateRange(scrollPos: number): void {
    const vh = opts.viewportRef.value?.clientHeight ?? 0;
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
    const items = opts.items();
    const out: { asset: PlacementView; box: Box; odd: boolean }[] = [];
    if (rs.length === 0) return out;
    const from = Math.min(r0.value, rs.length - 1);
    const to = Math.min(r1.value, rs.length);
    for (let r = from; r < to; r++) {
      for (let i = rs[r].start; i < rs[r].end; i++) {
        const asset = items[bs[i].i];
        if (!asset) continue; // 窗口期防御：items 已更新、旧 boxes 未重排（watch 异步）——跳过待 relayout
        out.push({ asset, box: bs[i], odd: r % 2 === 1 });
      }
    }
    return out;
  });

  function relayout(): void {
    const vp = opts.viewportRef.value;
    if (!vp || vp.clientWidth <= 0) return;
    ui.gridWidth = vp.clientWidth;
    // 行高只在布局时按容器宽 clamp（滑块 max = 内容区宽、min 80），
    // 不回写 targetRowHeight——否则窗口收窄到最小再拉出，卡片会保持被压的小尺寸
    const maxH = Math.max(80, vp.clientWidth);
    const rowH = Math.max(80, Math.min(maxH, ui.targetRowHeight));
    const t0 = performance.now();
    const items = opts.items();
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
        // 窗口期防御（2026-08-19 复验）：watch 异步 flush——items 已变短、boxes 还是旧布局时
        // box.i 可能越界（items 里无此下标），取不到卡就不锚定，等下一轮一致化
        const first = items[boxes.value[row.start]?.i ?? -1];
        if (first) {
          anchorKey = `${first.assetId}|${first.folderId}`;
          anchorRatio = row.height > 0 ? (layerY - row.top) / row.height : 0;
        }
      }
    }
    // 盒高 = 纯图高（7.1 满铺定稿，见 layouts.ts 头注）：名称/简介渐变罩叠图内不占布局，
    // 「显示简介」开关只切 AssetCard 简介行显隐（info prop），不再改变卡高、无需随开关重排
    // （2026-08-30 用户实机"卡片没按图片尺寸、都被裁剪"——盒高多 57/36 简介区致 cover 裁图）
    const computeOne = (list: { w: number; h: number }[]): LayoutResult =>
      ui.layoutMode === "waterfall"
        ? computeWaterfall(list, vp.clientWidth, rowH)
        : ui.layoutMode === "grid"
          ? computeGrid(list, vp.clientWidth, rowH)
          : ui.layoutMode === "list"
            ? computeList(list, vp.clientWidth, rowH)
            : computeJustified(list, vp.clientWidth, rowH);
    // 分组模式（3.5 二轮）：逐组独立布局拼全局坐标——组标题 26 + 标题↔内容 12 + 组间 24
    //（2026-08-19 复验定：12 太挤滑过察觉不到；首组离内容标题恒 12 由 cards-layer translateY 承担）；
    // 折叠组只计标题；每组剥掉布局自带底部留白（PAD.bottom），末组底部 PAD.bottom（对齐单段口径）；
    // rows.start/end 是 boxes 下标、box.i 是 items 下标，均按组基址偏移
    const secs = opts.sections?.() ?? null;
    if (secs && secs.length > 0) {
      const li = layoutItems.value;
      let y = 0;
      const allBoxes: Box[] = [];
      const allRows: Row[] = [];
      const headers: GroupHeaderInfo[] = [];
      secs.forEach((s, si) => {
        headers.push({ groupId: s.groupId, name: s.name, count: s.end - s.start, top: y });
        y += 26;
        if (!s.collapsed && s.end > s.start) {
          y += 12;
          const sub = computeOne(li.slice(s.start, s.end));
          const boxBase = allBoxes.length;
          for (const b of sub.boxes) allBoxes.push({ ...b, top: b.top + y, i: b.i + s.start });
          for (const r of sub.rows) allRows.push({ ...r, top: r.top + y, start: r.start + boxBase, end: r.end + boxBase });
          y += sub.totalHeight - PAD.bottom; // 剥单组底部留白
        }
        y += si === secs.length - 1 ? PAD.bottom : 24; // 组间 24；末组底部 PAD.bottom
      });
      boxes.value = allBoxes;
      rows.value = allRows;
      gridTotal.value = y;
      groupHeaders.value = headers;
    } else {
      const result = computeOne(layoutItems.value);
      boxes.value = result.boxes;
      rows.value = result.rows;
      gridTotal.value = result.totalHeight;
      groupHeaders.value = [];
    }
    setTotal(currentTotal());
    // 滚动位置：恢复（进出预览 / 切换目录）优先；否则回锚点；锚点卡已消失（被删等）维持重夹
    if (restorePos !== null) {
      scrollTo(restorePos);
      restorePos = null;
    } else if (anchorKey !== null) {
      const idx = items.findIndex((it) => `${it.assetId}|${it.folderId}` === anchorKey);
      const b = idx >= 0 ? boxes.value.find((bb) => bb.i === idx) : undefined;
      if (b) {
        // box.height = 行高（卡即图满铺，四种布局同构），按比例还原行内偏移
        scrollTo(b.top + headerH.value + 12 + listExtra.value + anchorRatio * b.height);
      }
    }
    console.debug(`[AssetGrid] 布局 ${items.length} 条：${(performance.now() - t0).toFixed(1)}ms`);
  }

  // 宽度变化 = 逐帧实时重排（Obsidian 式 reflow，2026-08-08 收展定案 v3，2026-08-10 推广到
  // 全部宽度变化）：虚拟滚动下 relayout 只更新可见卡片 transform，每帧跑得起；
  // RO 回调在当帧 rAF 之后、绘制之前执行（踩坑 025），重排与可见卡片 transform 同帧提交不闪。
  // scale 视觉补偿 + 停顿 240ms 旧机制废除（2026-08-10 用户实测：补偿期双轴缩放整页"滚动"
  // 与滚动锚点归位反复拉锯——拉走→弹回循环）。
  let lastW = 0;
  let lastH = 0;
  let headerRo: ResizeObserver | undefined;
  const ro = new ResizeObserver(() => {
    const vp = opts.viewportRef.value;
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
  watch(opts.viewKey, (newKey, oldKey) => {
    content.saveScrollPos(oldKey, pos.value);
    restorePos = content.getScrollPos(newKey) ?? 0;
  });
  watch(opts.items, () => relayout());
  // 分组规则 / 折叠态变化（3.5 二轮）：sections 引用变化即重排
  if (opts.sections) {
    watch(opts.sections, () => relayout());
  }
  watch(
    () => ui.targetRowHeight,
    () => relayout(),
  );
  // 折叠 / 展开"内容"：网格高度不计入总高
  watch(assetsOpen, () => setTotal(currentTotal()));
  // 布局方式切换（查看方式菜单）：四种算法盒高语义不同 → 重排；
  // 显示简介开关不在此——满铺卡简介行在渐变罩内显隐，不改盒高（见 relayout 注）
  watch(() => ui.layoutMode, () => relayout());

  onMounted(() => {
    attach(opts.viewportRef.value!, opts.trackRef.value!, opts.thumbRef.value!);
    lastW = opts.viewportRef.value!.clientWidth;
    lastH = opts.viewportRef.value!.clientHeight;
    ro.observe(opts.viewportRef.value!);
    // 头部高度实时跟踪（grid 换行 / 宽度变化时卡片层偏移即时跟随，避免重叠拉远）
    headerH.value = opts.headerRef.value!.clientHeight;
    headerRo = new ResizeObserver(() => {
      headerH.value = opts.headerRef.value!.clientHeight;
      setTotal(currentTotal());
    });
    headerRo.observe(opts.headerRef.value!);
    // 预览返回 / 视口初次挂载：恢复上次滚动位置（2026-07-25 用户定；1.2 key = 本标签页 viewKey）
    restorePos = content.getScrollPos(opts.viewKey());
    relayout();
  });

  onBeforeUnmount(() => {
    // 卸载（进预览 / 关闭标签页等）前保存本视口滚动位置
    content.saveScrollPos(opts.viewKey(), pos.value);
    detach();
    ro.disconnect();
    headerRo?.disconnect();
  });

  return {
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
    scrollTo, // 12 稿分组菜单「跳转」：滚动到组标题
  };
}
