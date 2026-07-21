<script setup lang="ts">
// 中栏标题栏（03_UI 布局/03_中间面板 02）：导航组（返回 / 前进 + 标题 + 搜索结果容器）
// / 滑块 / 显示子文件夹内容 / 按钮组——space-between 等距。
// 侧边栏收起时完全隐藏，其按钮组移入本标题栏（多个图标组合在一个容器里，子项间距 4）：
// 左收起 → 左侧放（汉堡菜单 + 展开按钮）；右收起 → 右侧放（展开 + 置顶 + 窗口控制）。
import { ref } from "vue";
import IconButton from "./IconButton.vue";
import SideSlider from "./SideSlider.vue";
import AppCheckbox from "./AppCheckbox.vue";
import WindowControls from "./WindowControls.vue";
import ViewOptionsMenu from "./ViewOptionsMenu.vue";
import { useUiStore } from "../stores/ui";
import { getCurrentWindow } from "@tauri-apps/api/window";

const ui = useUiStore();
const appWindow = getCurrentWindow();

// 搜索结果容器：搜索后从左侧滑入、取消后往左滑出（阶段 1 演示——点击标题模拟搜索 / 撤销，阶段 4 接真搜索）
const showResult = ref(false);

// 查看方式菜单：按钮右缘对齐、下方展开；再次点击按钮关闭（toggle）
const viewMenuOpen = ref(false);
const viewMenuAnchor = ref({ x: 0, y: 0 });
function openViewMenu(e: MouseEvent): void {
  if (viewMenuOpen.value) {
    viewMenuOpen.value = false;
    return;
  }
  const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
  viewMenuAnchor.value = { x: rect.right - 175, y: rect.bottom + 4 };
  viewMenuOpen.value = true;
}

async function togglePin(): Promise<void> {
  ui.alwaysOnTop = !ui.alwaysOnTop;
  await appWindow.setAlwaysOnTop(ui.alwaysOnTop);
}
</script>

<template>
  <div class="content-titlebar" data-tauri-drag-region>
    <div v-if="ui.leftCollapsed" class="btn-group">
      <IconButton name="menu" title="菜单" />
      <IconButton
        name="toggle-sidebar-left"
        title="展开侧边栏"
        @click="ui.leftCollapsed = false"
      />
    </div>
    <div class="nav-group">
      <div class="btn-group">
        <IconButton name="chevron-large" title="返回" class="rot-ccw" disabled />
        <IconButton name="chevron-large" title="前进" class="rot-cw" disabled />
      </div>
      <button class="crumb-text" v-tip="'演示：点击切换搜索结果'" @click="showResult = !showResult">
        全部
      </button>
      <Transition name="slide-x">
        <span v-if="showResult" class="result-text">/ 搜索结果 (123)</span>
      </Transition>
    </div>
    <SideSlider v-model="ui.targetRowHeight" :min="80" :max="ui.gridWidth" />
    <AppCheckbox v-model="ui.showSubfolders" label="显示子文件夹内容" />
    <div class="btn-group">
      <IconButton name="plugin" title="插件" />
      <IconButton name="view-options" title="查看方式" @click="openViewMenu" />
    </div>
    <div v-if="ui.rightCollapsed" class="btn-group">
      <IconButton
        name="toggle-sidebar-right"
        title="展开侧边栏"
        @click="ui.rightCollapsed = false"
      />
      <IconButton
        :name="ui.alwaysOnTop ? 'window-pin-off' : 'window-pin-on'"
        title="窗口置顶"
        @click="togglePin"
      />
      <WindowControls />
    </div>
    <ViewOptionsMenu v-if="viewMenuOpen" :anchor="viewMenuAnchor" @close="viewMenuOpen = false" />
  </div>
</template>

<style scoped>
/* 总容器：内边距 8 / 各组 space-between 等距 */
.content-titlebar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-8);
  flex: none;
}
/* 面包屑导航容器：返回前进按钮组（间距 4）+ 标题容器（间距 4） */
.nav-group {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  min-width: 0;
}
.btn-group {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  flex: none;
}
/* chevron-large 原图标朝下：返回 = 顺时针 90°（朝左），前进 = 逆时针 90°（朝右） */
.rot-ccw :deep(svg) {
  transform: rotate(90deg);
}
.rot-cw :deep(svg) {
  transform: rotate(-90deg);
}
.crumb-text {
  padding: 0 var(--space-4);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--c4);
  white-space: nowrap;
  border-radius: var(--radius-6);
}
/* 搜索结果容器：/ + 搜索结果 + (数量)，正文 C4-40 */
.result-text {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--c4-40);
  white-space: nowrap;
}
/* 从左侧滑入 / 往左滑出 */
.slide-x-enter-active,
.slide-x-leave-active {
  transition:
    transform 0.18s ease,
    opacity 0.18s ease;
}
.slide-x-enter-from,
.slide-x-leave-to {
  transform: translateX(-16px);
  opacity: 0;
}
</style>
