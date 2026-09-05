<script setup lang="ts">
// 全局右键菜单渲染器（6.3；7.1 二十四轮 2026-09-01 按新稿 组件/上下文菜单 重构）：宿主 App.vue
// 唯一实例，读 lib/contextMenus 的 menuState。渲染复用 AppMenu / MenuItem。
// 分组（分割线稿口径）："divider" 标记渲染层切成组容器——总容器 gap 0，组内 item gap 2，
// 组 padding 上下 4（首组上 0、末组下 0），组间 = 非末组 inset 底 1px 通用/五级线（无孤立分割线元素）。
// 多级级联 flyout 栈（2026-09-05 菜单栏主菜单接业务，单级改多级）：hover 带子菜单的项时右侧飞出，
// 右缘溢出改向左飞出（03 稿注"菜单不可超出窗口边缘"）；
// flyout 按 key（层级 + 父项 top）随父项重建（2026-08-11 走查：复用实例位置滞留越偏越远）+ pop=false 不回放弹出动画。
// 搜索区 / 各菜单定宽 随新稿废除（2026-09-01 用户定）；2026-09-05 走查：① 左侧标记槽恢复——
// CtxItem.icon 带图标项（带图标稿）+ 对勾移左侧（对勾稿本意，与快捷键可同存），同菜单无标记项
// 占位对齐（scanMark 扫描，根 / 各级 flyout 各自）；② linkList 二级 = tip 壳（AppMenu tone="tip" + pad 0，
// 与徽章 hover 弹窗同观感，容器无内距行贴边）；
// 宽度内容自适应（2026-09-01 二十五轮）：flyout 宽渲染前不可知——先按右飞放置，
// 挂载后实测宽，右缘溢出再以父项左缘为锚左飞（fixFlyoutX）。
// 组外关闭关系：根菜单与各级 flyout 的 group = 全部 flyout 根元素（点任一级内不关菜单）；
// 点组外任一级 AppMenu 都会 closeAll（幂等）。trigger = 触发按钮豁免（菜单栏「菜单」钮 toggle，2026-09-05）。
import { computed, nextTick, reactive, ref, watch } from "vue";
import AppMenu from "./AppMenu.vue";
import MenuItem from "./MenuItem.vue";
import LibraryMenuItem from "./LibraryMenuItem.vue";
import FolderIconPanel from "./FolderIconPanel.vue";
import FolderIconGrid from "./FolderIconGrid.vue";
import LinkListRows from "./LinkListRows.vue";
import { closeContextMenu, menuState, type CtxEntry, type CtxItem } from "../lib/contextMenus";
import { reorderLibraries, requestDeleteLibrary, currentLibrary, switchLibrary } from "../lib/libraryOps";

// 级联 flyout（2026-09-05 菜单栏主菜单：单级 → 多级栈）：hover 带子菜单的项时右侧飞出，
// 右缘溢出改向左飞出（03 稿注"菜单不可超出窗口边缘"）；flyout 按 key（层级 + 父项 top）随父项重建
//（2026-08-11 走查：复用实例位置滞留越偏越远）+ pop=false 不回放弹出动画。
// 栈语义：flyouts[i] = 第 i+2 级；hover 第 k 级（根 = 0）的项 → 截断到 k 层再按项推新层——
// hover 必先于 pointerdown，截断后深层 AppMenu 的"点外关闭"不会误伤浅层项的 click。
// panel（2026-08-18，3.3 / 05 稿 §3）= 自定义面板形态（图标网格 / linkList 链接位置双行列表 2026-09-05），不走条目列表；
// parentLeft = 父项左缘（左飞修正锚点）
interface FlyoutState {
  key: string;
  x: number;
  y: number;
  entries: CtxEntry[];
  panel?: NonNullable<CtxItem["panel"]>;
  parentLeft: number;
}
const flyouts = ref<FlyoutState[]>([]);
// 各级 flyout 根元素 / AppMenu 实例（函数 ref 收集；AppMenu group 组外判定与左飞实测用，非响应式——事件期现读）
const flyoutEls: (HTMLElement | null)[] = [];
const flyoutMenuRefs: (InstanceType<typeof AppMenu> | null)[] = [];

// 左标记槽占位扫描（2026-09-05 走查）：同一菜单（根 / flyout 各自）内有任一项带 icon 或 checkable
// → 无标记项左侧补 18 占位盒（名称纵向对齐贴稿；对勾与图标同槽互斥，扫描合并判定）
function scanMark(list: CtxEntry[] | null | undefined): boolean {
  for (const e of list ?? []) {
    if (e === "divider") continue;
    if (e.icon || e.checkable) return true;
  }
  return false;
}
const rootMark = computed(() => scanMark(menuState.entries));

// 每次打开清空上次 flyout 栈
watch(
  () => menuState.entries,
  () => {
    flyouts.value = [];
    flyoutEls.length = 0;
    flyoutMenuRefs.length = 0;
  },
);

// 分组：按 "divider" 切段（连续 divider 出空组、首末 divider 自动忽略）；index = entries 全局索引（库拖拽用）
interface GroupRow {
  entry: CtxItem;
  index: number;
}
function splitGroups(list: CtxEntry[]): GroupRow[][] {
  const out: GroupRow[][] = [[]];
  list.forEach((e, i) => {
    if (e === "divider") {
      if (out[out.length - 1].length) out.push([]);
    } else {
      out[out.length - 1].push({ entry: e, index: i });
    }
  });
  return out.filter((g) => g.length);
}
const groups = computed(() => splitGroups(menuState.entries ?? []));

// level：0 = 根菜单项，i+1 = flyouts[i] 内项。hover 即截断到本层（关掉更深层），再按项推新层
function onItemHover(entry: CtxItem, e: MouseEvent, level: number): void {
  flyouts.value.splice(level);
  flyoutEls.length = level;
  flyoutMenuRefs.length = level;
  if ((!entry.submenu && !entry.panel) || entry.disabled) return;
  const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
  // 菜单宽内容自适应（二十五轮）：先按右飞放置，挂载后实测宽——右缘溢出改向左飞出
  flyouts.value.push({ key: `${level}:${r.top}`, x: r.right + 2, y: r.top - 6, entries: entry.submenu ?? [], panel: entry.panel, parentLeft: r.left });
  void fixFlyoutX(level);
}

// flyout 左飞修正：实测宽（AppMenu expose root）右缘溢出 → 以父项左缘为锚向左飞出
async function fixFlyoutX(level: number): Promise<void> {
  await nextTick();
  const f = flyouts.value[level];
  const el = flyoutMenuRefs[level]?.root;
  if (!f || !el) return;
  const w = el.offsetWidth;
  if (f.x + w > window.innerWidth - 4) {
    flyouts.value[level] = { ...f, x: Math.max(4, f.parentLeft - w - 2) };
  }
}

function onItemClick(entry: CtxEntry): void {
  if (entry === "divider" || entry.disabled || entry.submenu || entry.panel || entry.swatches) return;
  // 库条目点击动态判定（2026-08-12 走查：菜单 keepOpen 期间可能已切库，
  // 静态 action 快照会让"切回去"失效——当前库跳过，其余直接按名切）
  if (entry.library) {
    if (entry.label !== currentLibrary.name) void switchLibrary(entry.label);
    if (!entry.keepOpen) closeAll(true);
    return;
  }
  entry.action?.();
  if (!entry.keepOpen) closeAll(true); // keepOpen = 状态切换类：执行后菜单保持（切库/排序等）
}

function closeAll(executed = false): void {
  flyouts.value = [];
  flyoutEls.length = 0;
  flyoutMenuRefs.length = 0;
  closeContextMenu(executed);
}

// 整体面板菜单（openPanelMenu）：选中回调 + 关菜单（一次性选择器，executed = true）
function onPanelPick(name: string | null): void {
  const panel = menuState.panel;
  closeAll(true);
  panel?.onPick(name);
}

// —— 库条目拖动排序（08 稿：抓手拖动、其他 item 让位移动动画、落点持久化 space.json libraryOrder）——
const ITEM_STEP = 35; // 库 item 高 33 + 组 gap 2（LibraryMenuItem 行高随 08 稿未迁，本常量随其）
const libDrag = reactive({ idx: -1, startY: 0, dy: 0, target: -1 });
let dragMoved = false; // 拖动超阈值：松手后的 click 误触发抑制
const settling = ref(false); // 松手归位帧：禁 .lib-item transform 过渡（松手即到位）

// entries 中库条目的索引（08 稿连续段）
const libIdxs = computed(() => (menuState.entries ?? []).map((e, i) => (e !== "divider" && e.library ? i : -1)).filter((i) => i >= 0));

// 让位位移：拖动项跟随指针；被跨越的项反向让出一步（transition 在 LibraryMenuItem 根样式）
function libItemStyle(i: number): string {
  if (libDrag.idx < 0) return "";
  const pos = libIdxs.value;
  const from = pos.indexOf(libDrag.idx);
  const to = pos.indexOf(libDrag.target);
  const me = pos.indexOf(i);
  if (i === libDrag.idx) return `transform: translateY(${libDrag.dy}px); transition: none; position: relative; z-index: 1;`;
  if (me < 0) return "";
  if (from < to && me > from && me <= to) return "transform: translateY(-35px)";
  if (to < from && me >= to && me < from) return "transform: translateY(35px)";
  return "";
}

function onDragMove(e: PointerEvent): void {
  if (libDrag.idx < 0) return;
  libDrag.dy = e.clientY - libDrag.startY;
  if (Math.abs(libDrag.dy) > 3) dragMoved = true;
  const pos = libIdxs.value;
  const from = pos.indexOf(libDrag.idx);
  const to = Math.max(0, Math.min(pos.length - 1, from + Math.round(libDrag.dy / ITEM_STEP)));
  libDrag.target = pos[to];
}

function onDragEnd(): void {
  window.removeEventListener("pointermove", onDragMove);
  const { idx, target } = libDrag;
  if (idx >= 0 && target !== idx && target >= 0 && menuState.entries) {
    const arr = menuState.entries;
    const [moved] = arr.splice(idx, 1);
    arr.splice(target, 0, moved);
    const order = arr.filter((e): e is CtxItem => e !== "divider" && !!e.library).map((e) => e.label);
    void reorderLibraries(order); // 失败 toast 在封装内，顺序以后端为准（下次开菜单重拉）
    // 松手即到位（2026-08-12 走查）：让位项视觉本已在目标位，清 transform 必须禁过渡，
    // 否则 DOM 重排 + 清除的过程会再放一遍交换动画（"弹回去"观感）
    settling.value = true;
    setTimeout(() => {
      settling.value = false;
    }, 50);
  }
  libDrag.idx = -1;
  libDrag.target = -1;
}

// 系统抢指针兜底（规范第 4 条：blur 强制取消，不提交）
function onDragCancel(): void {
  window.removeEventListener("pointermove", onDragMove);
  libDrag.idx = -1;
  libDrag.target = -1;
}

function onGripDown(i: number, e: PointerEvent): void {
  libDrag.idx = i;
  libDrag.startY = e.clientY;
  libDrag.dy = 0;
  libDrag.target = i;
  dragMoved = false;
  window.addEventListener("pointermove", onDragMove);
  window.addEventListener("pointerup", onDragEnd, { once: true });
  window.addEventListener("blur", onDragCancel, { once: true });
}

// 库条目点击：拖动松手后的 click 抑制；否则切库（action 同普通项）
function onLibrarySelect(entry: CtxItem): void {
  if (dragMoved) {
    dragMoved = false;
    return;
  }
  onItemClick(entry);
}

// 删除×：关菜单弹确认（确认模态在 App.vue，libraryOps.deleteLibraryState）
function onLibraryRemove(entry: CtxItem): void {
  const name = entry.label;
  closeAll(false);
  requestDeleteLibrary(name);
}
</script>

<template>
  <!-- 一级条目菜单 min-width 250（2026-09-05 用户实机定：纯自适应太窄显得轻）；面板形态（iconPicker 稿值宽）不传 -->
  <AppMenu v-if="menuState.entries" :x="menuState.x" :y="menuState.y" :min-w="menuState.panel ? undefined : 250" :group="flyoutEls" :trigger="menuState.trigger" @close="closeAll(false)">
    <!-- 整体面板菜单（openPanelMenu）：一级内容 = 面板组件，无条目列表 -->
    <FolderIconGrid v-if="menuState.panel?.kind === 'iconPicker'" :current="menuState.panel.current" @select="onPanelPick" />
    <!-- 条目菜单：分组容器（组内 gap 2 / 组 padding 上下 4 首末 0 / 组间 1px 线） -->
    <div v-else class="menu-groups" :class="{ settling }">
      <div v-for="(group, gi) in groups" :key="gi" class="menu-group">
        <template v-for="row in group" :key="row.entry.library ? `lib:${row.entry.label}` : row.index">
          <LibraryMenuItem
            v-if="row.entry.library"
            :label="row.entry.label"
            :current="row.entry.label === currentLibrary.name"
            :style="libItemStyle(row.index)"
            @select="onLibrarySelect(row.entry)"
            @remove="onLibraryRemove(row.entry)"
            @grip-down="onGripDown(row.index, $event)"
          />
          <MenuItem
            v-else
            :label="row.entry.label"
            :icon="row.entry.icon"
            :mark-pad="rootMark"
            :trailing-icon="row.entry.trailingIcon"
            :trailing-icon-class="row.entry.trailingIconClass"
            :shortcut="row.entry.shortcut"
            :disabled="row.entry.disabled"
            :arrow="!!(row.entry.submenu || row.entry.panel)"
            :checkable="row.entry.checkable"
            :checked="row.entry.checked"
            :swatches="row.entry.swatches"
            @select="onItemClick(row.entry)"
            @pick="row.entry.swatches?.onPick($event)"
            @mouseenter="onItemHover(row.entry, $event, 0)"
          />
        </template>
      </div>
    </div>
  </AppMenu>
  <!-- 级联 flyout 栈（多级）：key = 层级 + 父项 top 随父项重建（防实例复用位置滞留）；pop=false 不回放动画；
       内容包一层 div 供各级 AppMenu group 判定（点任一级 flyout 内 = 组内，不关菜单）；
       面板形态（图标网格）与条目形态 padding 统一 6（2026-09-01 二十五轮，废 24） -->
  <AppMenu
    v-for="(flyout, i) in flyouts"
    :key="flyout.key"
    :ref="(el) => (flyoutMenuRefs[i] = el as InstanceType<typeof AppMenu> | null)"
    :x="flyout.x"
    :y="flyout.y"
    :pop="false"
    :pad="flyout.panel?.kind === 'linkList' ? '0' : undefined"
    :tone="flyout.panel?.kind === 'linkList' ? 'tip' : 'menu'"
    :group="flyoutEls"
    :trigger="menuState.trigger"
    @close="closeAll(false)"
  >
    <div :ref="(el) => (flyoutEls[i] = el as HTMLElement | null)" class="flyout-root">
      <FolderIconPanel v-if="flyout.panel?.kind === 'folderIcon'" :folder-id="flyout.panel.folderId" @done="closeAll(true)" />
      <LinkListRows v-else-if="flyout.panel?.kind === 'linkList'" :asset-id="flyout.panel.assetId" @jump="closeAll(true)" />
      <div v-else class="menu-groups">
        <div v-for="(group, gi) in splitGroups(flyout.entries)" :key="gi" class="menu-group">
          <MenuItem
            v-for="row in group"
            :key="row.index"
            :label="row.entry.label"
            :icon="row.entry.icon"
            :mark-pad="scanMark(flyout.entries)"
            :trailing-icon="row.entry.trailingIcon"
            :trailing-icon-class="row.entry.trailingIconClass"
            :shortcut="row.entry.shortcut"
            :disabled="row.entry.disabled"
            :arrow="!!(row.entry.submenu || row.entry.panel)"
            :checkable="row.entry.checkable"
            :checked="row.entry.library ? row.entry.label === currentLibrary.name : row.entry.checked"
            @select="onItemClick(row.entry)"
            @mouseenter="onItemHover(row.entry, $event, i + 1)"
          />
        </div>
      </div>
    </div>
  </AppMenu>
</template>

<style scoped>
.flyout-root {
  display: contents;
}
/* 分组容器（分割线稿）：组内 item gap 2；组 padding 上下 4、首组上 0 末组下 0（容器 padding 6 已供边距）；
   组间分割 = 非末组盒内底 1px 通用/五级线（inset 不占布局，组间距恒 8px 贴稿；无孤立分割线元素） */
.menu-groups {
  display: contents;
}
.menu-group {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-2);
  padding: var(--n-space-4) 0;
}
.menu-group:first-child {
  padding-top: 0;
}
.menu-group:last-child {
  padding-bottom: 0;
}
.menu-group:not(:last-child) {
  box-shadow: inset 0 -1px 0 var(--n-g5);
}
/* 库排序松手归位帧：禁 transform 过渡（让位项视觉本已在目标位，2026-08-12 走查） */
.menu-groups.settling :deep(.lib-item) {
  transition: none;
}
</style>
