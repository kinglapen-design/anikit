<script setup lang="ts">
// 中间面板 dock 宿主（1.2 主窗口重构）：多标签页 + 拖拽拆分停靠（dockview）。
// panel 内容 = ContentArea（按各自 viewKey 渲染视口）；标签 = TabLabel；右侧添加钮 = TabAddButton。
// 通知条单例在此层（原 ContentArea 内——多 panel 下每 panel 一份会重复弹出）。
// floating groups / popout 本轮禁用（样式未适配；popout 在 Tauri webview 不可用）。
// 窗口拖拽：标签栏不设 data-tauri-drag-region（与 dockview tab 拖拽冲突），
// 窗口拖拽区保留在 panel 内工具栏行（ContentTitleBar 既有 drag-region，记待办后续评估）。
import { DockviewVue, type DockviewReadyEvent, type VueComponent } from "dockview-vue";
import { nextTick, ref } from "vue";
import ContentArea from "./ContentArea.vue";
import TabLabel from "./TabLabel.vue";
import TabAddButton from "./TabAddButton.vue";
import SidebarExpandButton from "./SidebarExpandButton.vue";
import ToastStack from "./ToastStack.vue";
import { anikitDockTheme } from "../lib/dockTheme";
import { useTabsStore } from "../stores/tabs";
import { useUiStore } from "../stores/ui";

const tabs = useTabsStore();
const ui = useUiStore();

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

function onReady(e: DockviewReadyEvent): void {
  void tabs.bind(e.api);
  applyDragRegions();
  e.api.onDidLayoutChange(() => applyDragRegions()); // 分屏新增 group 的标签栏空白补设
}
</script>

<template>
  <main ref="rootRef" class="dock-host" :class="{ 'pad-win': ui.rightCollapsed || ui.sidebarAnimating }">
    <DockviewVue
      class="dock-root"
      :components="components"
      :tab-components="tabComponents"
      :prefix-header-actions-component="prefixHeaderActions"
      :left-header-actions-component="leftHeaderActions"
      :theme="anikitDockTheme"
      :disable-floating-groups="true"
      dnd-strategy="pointer"
      @ready="onReady"
    />
    <ToastStack />
  </main>
</template>

<style scoped>
/* 中间面板容器：C1 底，dock 铺满；relative 供通知条锚定（原 ContentArea 口径） */
.dock-host {
  position: relative;
  flex: 1;
  min-width: 0;
  background: var(--c1);
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
