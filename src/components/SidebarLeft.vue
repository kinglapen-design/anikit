<script setup lang="ts">
// 左侧边栏（C3）：标题栏 / 切换资源库 / 视图内容 / 底部视图切换。
// 视图切换：旧区块往左滑出、新区块从右滑入（03_UI 布局/02_左侧边栏 01）；
// 切换资源库区块仅在工作空间 ↔ 其他三视图之间切换时才滑出滑入。
// 折叠后收成窄条（保留菜单、添加与展开按钮）。
import { computed, ref } from "vue";
import AppIcon from "./AppIcon.vue";
import IconButton from "./IconButton.vue";
import ViewSwitch, { type ViewOption } from "./ViewSwitch.vue";
import LibraryView from "./LibraryView.vue";
import SidebarViews from "./SidebarViews.vue";
import { useUiStore } from "../stores/ui";
import { useSideThumb } from "../lib/useSideThumb";

const ui = useUiStore();
const bodyRef = ref<HTMLElement>();
const thumb = useSideThumb(bodyRef);

const leftOptions: ViewOption[] = [
  { value: "library", icon: "library-view", title: "资源库视图" },
  { value: "workspace", icon: "workspace-view", title: "工作空间视图" },
  { value: "tags", icon: "tag-view", title: "标签视图" },
  { value: "pins", icon: "favorites-view", title: "快捷入口" },
];

// 资源库视图 / 标签视图 / 快捷入口视图共用资源库；工作空间是另一套（03_UI 布局/02 03）
const isWorkspace = computed(() => ui.leftView === "workspace");
</script>

<template>
  <aside v-if="!ui.leftCollapsed" class="sidebar-left" :style="{ width: `${ui.leftWidth}px` }">
    <header class="bar-title">
      <IconButton name="menu" title="菜单" />
      <span class="spacer" />
      <IconButton name="add-large" title="添加" />
      <IconButton name="toggle-sidebar-left" title="收起侧边栏" @click="ui.leftCollapsed = true" />
    </header>

    <Transition name="slide" mode="out-in">
      <div class="library-switch" :key="isWorkspace ? 'ws' : 'lib'">
        <span class="badge">
          <AppIcon :name="isWorkspace ? 'workspace-badge' : 'library-badge'" :size="16" />
        </span>
        <span class="lib-name">{{ isWorkspace ? "我的项目" : "动画素材库" }}</span>
        <span class="spacer" />
        <IconButton name="switch-library" title="切换" />
      </div>
    </Transition>

    <div ref="bodyRef" class="view-body">
      <Transition name="slide" mode="out-in">
        <LibraryView v-if="ui.leftView === 'library'" key="library" />
        <SidebarViews v-else :key="ui.leftView" />
      </Transition>
    </div>
    <span
      v-show="thumb.show"
      class="side-thumb"
      :style="{ top: `${thumb.top}px`, height: `${thumb.height}px` }"
    />

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
  position: relative;
  display: flex;
  flex-direction: column;
  flex: none;
  background: var(--c3);
  border-radius: var(--radius-6);
  box-shadow: inset 0 0 0 1px var(--c4-15);
  padding: var(--space-4);
  min-height: 0;
}
/* 悬浮滚动条：4px 胶囊 C4-15、距右 4px、不占布局（token 15） */
.side-thumb {
  position: absolute;
  right: 4px;
  width: 4px;
  border-radius: 2px;
  background: var(--c4-15);
  pointer-events: none;
  z-index: 10;
}
.sidebar-left.collapsed {
  width: 36px;
  align-items: center;
  gap: var(--space-4);
}
.flip {
  transform: scaleX(-1);
}
/* 标题栏：内边距 8 8 8 14 */
.bar-title {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  padding: var(--space-8) var(--space-8) var(--space-8) var(--space-14);
}
.spacer {
  flex: 1;
}
/* 切换资源库：内边距 8 8 8 20 / 间距 8；图标容器 24×24 品牌色-1 圆角 6 */
.library-switch {
  display: flex;
  align-items: center;
  gap: var(--space-8);
  padding: var(--space-8) var(--space-8) var(--space-8) var(--space-20);
  border-radius: var(--radius-6);
}
.badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
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
/* 原生条由 base.css 全局隐藏，滚动条为悬浮 side-thumb（不占位） */
.view-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
}
/* 视图切换动画：旧区块往左滑出，新区块从右滑入 */
.slide-enter-active,
.slide-leave-active {
  transition:
    transform 0.18s ease,
    opacity 0.18s ease;
}
.slide-enter-from {
  transform: translateX(40px);
  opacity: 0;
}
.slide-leave-to {
  transform: translateX(-40px);
  opacity: 0;
}
/* 视图切换总容器：内边距 4 8 8 8 */
.bar-footer {
  padding: var(--space-4) var(--space-8) var(--space-8) var(--space-8);
}
</style>
