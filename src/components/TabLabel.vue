<script setup lang="ts">
// 标签页渲染（1.2，figma-css 15_标签栏）：dockview tabComponent，全接管默认 tab 内容。
// 标题按 viewKey 实时派生（文件夹重命名自动跟随）；锁定 = 置顶图标替代关闭钮；
// 关闭钮未选中 opacity 0、悬停 / 选中显示；右键 = 39 号稿菜单（lib/tabMenus）。
import { computed, onBeforeUnmount, ref } from "vue";
import type { DockviewPanelApi, IDockviewPanelHeaderProps } from "dockview-vue";
import AppIcon from "./AppIcon.vue";
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

// 选中态（关闭钮显隐判定）：panel api 事件驱动，不靠全局 CSS 类（scoped 样式自包含）
const active = ref(panelApi.isActive);
const disposeActive = panelApi.onDidActiveChange(() => {
  active.value = panelApi.isActive;
});
onBeforeUnmount(() => disposeActive.dispose());

function onClose(e: MouseEvent): void {
  e.stopPropagation();
  tabs.closeTab(panelApi.id);
}

function onMenu(e: MouseEvent): void {
  openTabContextMenu(e, panelApi.id);
}
</script>

<template>
  <div class="tab-label" :class="{ active, locked }" @contextmenu.prevent.stop="onMenu">
    <span class="tab-title">{{ title }}</span>
    <span v-if="locked" class="tab-pin" v-tip="'已锁定'">
      <AppIcon name="window-pin-on" :size="18" />
    </span>
    <button v-else class="tab-close" v-tip="'关闭标签页'" @mousedown.stop @click="onClose">
      <AppIcon name="clear" :size="12" />
    </button>
  </div>
</template>

<style scoped>
/* 标签页内容：标题 + 右侧钮（关闭 20×20 padding 4 / 锁定图标 18，figma 15） */
.tab-label {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
  min-width: 0;
  width: 100%;
}
.tab-title {
  font-size: var(--text-label-size);
  line-height: var(--text-label-line);
  color: var(--text-normal);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.tab-close {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  padding: var(--space-4);
  box-sizing: border-box;
  border-radius: var(--radius-6);
  color: var(--text-label); /* figma：关闭 × = 标签档 */
  flex: none;
  transition: background 0.15s ease;
}
.tab-close:hover {
  background: var(--line-hover);
}
/* 未选中：关闭钮 opacity 0（figma 15 未选中态），悬停标签页时显示 */
.tab-label:not(.active) .tab-close {
  opacity: 0;
}
.tab-label:hover .tab-close {
  opacity: 1;
}
.tab-pin {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  color: var(--text-normal); /* figma 15 锁定态：图标 = 常态档 */
  flex: none;
}
</style>
