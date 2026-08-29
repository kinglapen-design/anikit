<script setup lang="ts">
// 标签页 dockview 适配器（7.1 十九轮重写）：视觉全归 items/TabItem（贴稿），本件只接业务——
// 标题按 viewKey 实时派生（文件夹重命名自动跟随）；锁定 = pin 替代关闭钮（TabItem locked）；
// 右键 = lib/tabMenus；点击激活 / 拖拽换位由 dockview 外壳（.dv-tab）处理，本件不接 click。
import { computed, onBeforeUnmount, ref } from "vue";
import type { DockviewPanelApi, IDockviewPanelHeaderProps } from "dockview-vue";
import TabItem from "./items/TabItem.vue";
import { useContentStore } from "../stores/content";
import { useTabsStore } from "../stores/tabs";
import { openTabContextMenu } from "../lib/tabMenus";

// props 运行时声明（rolldown 环境 SFC 编译器无法解析 node_modules 类型，defineProps 泛型须字面量）。
// dockview-vue 传参约定（v7 源码 VueRenderer.init）：framework props 整组包在一个 params prop 里——
// 组件实际收到 params = { params: 用户参数, api, containerApi, tabLocation }。
const raw = defineProps<{ params: Record<string, unknown> }>();
const frame = computed(() => raw.params as unknown as IDockviewPanelHeaderProps);
const panelApi = frame.value.api as DockviewPanelApi; // api 引用 init 时确定、不变
const content = useContentStore();
const tabs = useTabsStore();

const viewKey = computed(() => (frame.value.params?.viewKey as string) ?? "quick:all");
const locked = computed(() => !!frame.value.params?.locked);
const title = computed(() => content.viewTitle(viewKey.value));

// 选中态：panel api 事件驱动，不靠全局 CSS 类（TabItem selected 自包含样式）
const active = ref(panelApi.isActive);
const disposeActive = panelApi.onDidActiveChange(() => {
  active.value = panelApi.isActive;
});
onBeforeUnmount(() => disposeActive.dispose());

function onClose(): void {
  tabs.closeTab(panelApi.id);
}

function onMenu(e: MouseEvent): void {
  openTabContextMenu(e, panelApi.id);
}
</script>

<template>
  <!-- contextmenu 走 attribute fallthrough 落 TabItem 根；TabItem 根即视觉标签页，撑满 dockview 外壳 -->
  <TabItem
    class="dock-tab"
    :label="title"
    :selected="active"
    :locked="locked"
    @close="onClose"
    @contextmenu.prevent.stop="onMenu"
  />
</template>

<style scoped>
/* 撑满 dockview 外壳（.dv-tab 180×38，主题层定尺寸）；样式全在 TabItem scoped */
.dock-tab {
  width: 100%;
  height: 100%;
}
</style>
