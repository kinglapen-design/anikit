<script setup lang="ts">
// 左侧边栏（C3）：标题栏 / 切换素材库 / 视图内容 / 底部视图切换。
// 折叠后收成窄条（保留菜单、添加与展开按钮），拖拽调宽由 App 的拆分条负责。
import AppIcon from "./AppIcon.vue";
import IconButton from "./IconButton.vue";
import ViewSwitch, { type ViewOption } from "./ViewSwitch.vue";
import LibraryView from "./LibraryView.vue";
import SidebarViews from "./SidebarViews.vue";
import { useUiStore } from "../stores/ui";

const ui = useUiStore();

const leftOptions: ViewOption[] = [
  { value: "library", icon: "library-view", title: "资源库视图" },
  { value: "workspace", icon: "workspace-view", title: "工作空间视图" },
  { value: "tags", icon: "tag-view", title: "标签视图" },
  { value: "pins", icon: "favorites-view", title: "快捷入口" },
];
</script>

<template>
  <aside v-if="!ui.leftCollapsed" class="sidebar-left" :style="{ width: `${ui.leftWidth}px` }">
    <header class="bar-title">
      <IconButton name="menu" title="菜单" />
      <span class="spacer" />
      <IconButton name="add-large" title="添加" />
      <IconButton name="toggle-sidebar-left" title="收起侧边栏" @click="ui.leftCollapsed = true" />
    </header>

    <div class="library-switch">
      <span class="badge"><AppIcon name="library-badge" :size="17" /></span>
      <span class="lib-name">动画素材库</span>
      <span class="spacer" />
      <IconButton name="switch-library" title="切换素材库" />
    </div>

    <div class="view-body">
      <LibraryView v-if="ui.leftView === 'library'" />
      <SidebarViews v-else />
    </div>

    <footer class="bar-footer">
      <ViewSwitch v-model="ui.leftView" :options="leftOptions" />
    </footer>
  </aside>

  <aside v-else class="sidebar-left collapsed">
    <IconButton name="menu" title="菜单" />
    <IconButton name="add-large" title="添加" />
    <IconButton
      name="toggle-sidebar-left"
      title="展开侧边栏"
      class="flip"
      @click="ui.leftCollapsed = false"
    />
  </aside>
</template>

<style scoped>
.sidebar-left {
  display: flex;
  flex-direction: column;
  flex: none;
  background: var(--c3);
  border: 1px solid var(--c4-15);
  border-radius: var(--radius-6);
  padding: var(--space-4);
  min-height: 0;
}
.sidebar-left.collapsed {
  width: 36px;
  align-items: center;
  gap: var(--space-4);
}
.flip {
  transform: scaleX(-1);
}
/* 组件布局 04.1：标题栏内容容器内边距 8 8 8 12 */
.bar-title {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  padding: var(--space-8) var(--space-8) var(--space-8) var(--space-12);
}
.spacer {
  flex: 1;
}
/* token 11 切换素材库：内边距 8 8 8 18 */
.library-switch {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  padding: var(--space-8) var(--space-8) var(--space-8) var(--space-18);
}
.badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: var(--space-4);
  border-radius: var(--radius-6);
  background: var(--brand-1);
  color: var(--c4);
  flex: none;
}
.lib-name {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--c4);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.view-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding-right: var(--space-4);
}
/* 次要区域原生滚动条按 token 15 绘制（主列表为自定义 transform 滚动） */
.view-body::-webkit-scrollbar {
  width: 4px;
}
.view-body::-webkit-scrollbar-thumb {
  background: var(--c4-15);
  border-radius: 2px;
}
/* 组件布局 04.5：视图切换内容容器内边距 4 8 8 8 */
.bar-footer {
  padding: var(--space-4) var(--space-8) var(--space-8) var(--space-8);
}
</style>
