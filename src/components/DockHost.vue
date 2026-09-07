<script setup lang="ts">
// 中间面板 dock 宿主（1.2 主窗口重构）：多标签页 + 拖拽拆分停靠（dockview）。
// panel 内容 = ContentArea（按各自 viewKey 渲染视口）；标签 = TabLabel；右侧添加钮 = TabAddButton。
// 通知条单例在此层（原 ContentArea 内——多 panel 下每 panel 一份会重复弹出）。
// floating groups / popout 本轮禁用（样式未适配；popout 在 Tauri webview 不可用）。
// 标签溢出下拉（tabs overflow list）禁用（2026-08-30：标签页 shrink 平分策略下无意义，
// 且亚像素取整假溢出会让它压在 + 钮右端占位 ~60px——实机「+ 钮右边空白过多」根因）。
// 窗口拖拽：标签栏不设 data-tauri-drag-region（与 dockview tab 拖拽冲突），
// 窗口拖拽区保留在 panel 内工具栏行（ContentTitleBar 既有 drag-region，记待办后续评估）。
import { DockviewVue, type DockviewGroupPanel, type DockviewReadyEvent, type VueComponent } from "dockview-vue";
import { nextTick, ref } from "vue";
import ContentArea from "./ContentArea.vue";
import TabLabel from "./TabLabel.vue";
import TabAddButton from "./TabAddButton.vue";
import SidebarExpandButton from "./SidebarExpandButton.vue";
import ToastStack from "./ToastStack.vue";
import PickerBanner from "./PickerBanner.vue";
import { anikitDockTheme } from "../lib/dockTheme";
import { useTabsStore } from "../stores/tabs";
import { useUiStore } from "../stores/ui";
import { usePickerStore } from "../stores/picker";

const tabs = useTabsStore();
const ui = useUiStore();
const picker = usePickerStore();

// dockview-vue 7.0.4 的 VueComponent = DefineComponent<T>（单泛型签名），与 Vue 3.5+ SFC
// 编译产物的 DefineComponent 多泛型签名不可互赋——纯类型层不兼容，运行时无碍，统一断言。
const components = { content: ContentArea as unknown as VueComponent };
const tabComponents = { tab: TabLabel as unknown as VueComponent };
const prefixHeaderActions = SidebarExpandButton as unknown as VueComponent;
const leftHeaderActions = TabAddButton as unknown as VueComponent;

const rootRef = ref<HTMLElement>();

// 标签栏空白区（dockview .dv-void-container）挂窗口拖拽（2026-08-08 用户反馈）：
// Tauri drag-region 在 mousedown 时按 closest 动态判定，属性后设有效；
// 标签页 / 按钮不在 void 区内，不受窗口拖拽抢占（dockview tab 拖拽 = pointer 策略）。
// 同时拦截 void 的 pointerdown（capture + stopImmediatePropagation）：防 dockview 在此启动
// group 拖拽（浮动已禁，拖空白移组无意义；否则与窗口拖拽同时触发，2026-08-08 走查）——
// Tauri 拖窗走 mousedown，不受 pointerdown 拦截影响。
function applyDragRegions(): void {
  void nextTick(() => {
    rootRef.value?.querySelectorAll(".dv-void-container").forEach((el) => {
      if (el.hasAttribute("data-tauri-drag-region")) return;
      el.setAttribute("data-tauri-drag-region", "");
      el.addEventListener("pointerdown", (e) => e.stopImmediatePropagation(), { capture: true });
    });
  });
}

// 单标签拖拽分屏修复（实机走查第一批条 9）：dockview 对「单 panel 组拖自己内容区
// 边缘」走「单 tab = 移组」优化——源/目标同分支时 moveChild 自身 = no-op（分屏永不成功，
// dockview-core moveTo 源码实证）。拦组级 model 的 willDrop（可 preventDefault，
// dockview 随即放弃默认处理），改走 tabs.splitTab 同语义——同视口复制到新组，
// 与右键菜单分屏一致（单标签拖走 = 源组销毁布局回弹，复制是该场景唯一有效结果）。
// 注：组件级 @will-drop 在无 advancedDnD 模块时不转发组级事件，必须逐组订阅 model。
const willDropHooked = new Set<string>();
function hookGroupWillDrop(group: DockviewGroupPanel): void {
  if (willDropHooked.has(group.id)) return; // onDidAddGroup 移组时重复触发，幂等防重挂
  willDropHooked.add(group.id);
  group.model.onWillDrop((e) => {
    if (e.kind !== "content") return; // 只拦内容区边缘拖入（标签栏内排序不碰）
    // willDrop 的 position 词汇是 top/bottom，addPanel 的 direction 词汇是 above/below，映射后传
    const pos = e.position;
    const direction =
      pos === "left" ? ("left" as const)
      : pos === "right" ? ("right" as const)
      : pos === "top" ? ("above" as const)
      : pos === "bottom" ? ("below" as const)
      : null;
    if (!direction) return;
    const data = e.getData();
    if (!data || data.panelId === null || data.tabGroupId) return;
    const target = e.group; // 落点组（自拖 = 源组）
    if (!target || target.id !== data.groupId || target.panels.length !== 1) return;
    e.preventDefault();
    tabs.splitTab(data.panelId, direction);
  });
}

function onReady(e: DockviewReadyEvent): void {
  void tabs.bind(e.api);
  e.api.groups.forEach(hookGroupWillDrop);
  e.api.onDidAddGroup((g) => hookGroupWillDrop(g)); // 分屏新建组同挂
  applyDragRegions();
  e.api.onDidLayoutChange(() => applyDragRegions()); // 分屏新增 group 的标签栏空白补设
}
</script>

<template>
  <main ref="rootRef" class="dock-host" :class="{ 'pad-win': ui.rightCollapsed || ui.sidebarAnimating, 'left-collapsed': ui.leftCollapsed }">
    <DockviewVue
      class="dock-root"
      :components="components"
      :tab-components="tabComponents"
      :prefix-header-actions-component="prefixHeaderActions"
      :left-header-actions-component="leftHeaderActions"
      :theme="anikitDockTheme"
      :disable-floating-groups="true"
      :disable-tabs-overflow-list="true"
      dnd-strategy="pointer"
      @ready="onReady"
    />
    <!-- 通知族统一锚点（2026-08-29 用户定）：选择模式横幅（上）+ 通知条（下）同列，
         列表容器顶部居中不挡工具栏；横幅随选择模式从 App 层迁入本面板锚定 -->
    <div class="notice-anchor">
      <PickerBanner v-if="picker.active" />
      <ToastStack />
    </div>
  </main>
</template>

<style scoped>
/* 中间面板容器：无底色——标签栏 / group 区透窗口云母或实色底（7.1 十九轮二轮用户定：
   旧 c1 实底把云母挡住了）；内容容器 g1 卡片由 dockview-theme .dv-content-container 画。
   relative 供通知条锚定（原 ContentArea 口径） */
.dock-host {
  position: relative;
  flex: 1;
  min-width: 0;
}
/* 左栏收起时：标签栏左 padding 36 → 12（2026-08-30 用户定——收起态行首展开钮距窗口左缘
   36 太远；展开态保持稿值 36） */
.dock-host.left-collapsed :deep(.dv-tabs-and-actions-container) {
  padding-left: var(--n-space-12);
}
/* DockviewVue 是 Fragment 多根组件：scoped 的 data-v 落不到它内部 div，
   尺寸必须 :deep 穿透（否则容器高 0，dockview 铺满失败 = 全黑，2026-08-08 踩坑） */
:deep(.dock-root) {
  width: 100%;
  height: 100%;
}
/* 右栏收起或收展动画期间：标签页栏右端给窗口控制浮块让位（浮块 156px 恒在窗口右上角，
   aside 宽度渐变途中浮块会漫到标签页栏上——让位覆盖整个动画窗口，防重叠/断开，2026-08-08） */
.dock-host.pad-win :deep(.dv-tabs-and-actions-container) {
  padding-right: 160px;
}
</style>
