<script setup lang="ts">
// 左侧边栏（7.1 十八轮贴稿重构，稿 = docs/UI 重构/Design Token/布局/主窗口/左侧边栏/）：
// 254 宽 column 三段——菜单栏 46（收起侧边栏 plain 钮 + 菜单/编辑/插件/视图 四文字钮，ghost sm；
// 四钮点亮不接业务，2026-08-30 用户定——汉堡菜单 / 新增菜单 / Ctrl+N / 区块管理 / 切库入口
// 随旧 rail 结构一并撤下，spec 与快捷键逻辑保留在 lib（网格右键新增菜单仍在），接业务时恢复）；
// 分段控制器 54（SegmentedControl icon 型：资源库 / 工作空间 / AI 对话框——三段可切换，
// 工作空间 / AI = 空态「功能正在开发中」居中，2026-08-30 用户定；选择模式下禁用切换，
// 视觉不改——稿无禁用态）；
// 树容器 flex 1（LibraryView：快速访问 + 文件夹区块；ScrollThumb 悬浮滚轮，速查 §2）。
// 收展动画 = sidebarShowWidths 唯一写点（速查 §8）：模板只绑它，toggle 只翻布尔；
// 本文件禁挂 width 的 CSS transition（与逐帧 JS 写宽打架 = 慢一帧）；
// --sb-w 恒为展开宽、子块定宽 = 收窄时右缘裁剪不挤压；展开按钮在标签页行首（SidebarExpandButton）。
import { ref } from "vue";
import IconButton from "./buttons/IconButton.vue";
import ActionButton from "./buttons/ActionButton.vue";
import SegmentedControl from "./buttons/SegmentedControl.vue";
import ScrollThumb from "./ScrollThumb.vue";
import LibraryView from "./LibraryView.vue";
import { useUiStore } from "../stores/ui";
import { usePickerStore } from "../stores/picker";
import { sidebarShowWidths } from "../lib/useSidebarAnim";

const ui = useUiStore();
const picker = usePickerStore();

// 菜单栏四文字钮（菜单 / 编辑 / 插件 / 视图）：点亮不接业务（2026-08-30 用户定，业务接线归后续轮）
const menuButtons = ["菜单", "编辑", "插件", "视图"] as const;

// 分段控制器（模式切换）：library = 资源库树；workspace / ai = 空态占位（功能正在开发中）。
// 会话级状态（旧 rail 同样不可切、无持久化先例）；选择模式（6.2 白名单收窄）下禁用切换——
// 树 / 快速访问导航是挑选主通路，须恒在
const viewMode = ref<"library" | "workspace" | "ai">("library");
const segOptions = [
  { value: "library", icon: "library" },
  { value: "workspace", icon: "workspace" },
  { value: "ai", icon: "ai-chat" },
] as const;
function onModeChange(v: string): void {
  if (picker.active) return;
  viewMode.value = v as "library" | "workspace" | "ai";
}

const bodyRef = ref<HTMLElement>();
</script>

<template>
  <aside
    class="sidebar-left"
    :style="{ width: `${sidebarShowWidths.left}px`, '--sb-w': `${ui.leftWidth}px` }"
  >
    <!-- 菜单栏（稿 左侧边栏-菜单栏）：46 高 / padding 8 12 / gap 2；兼窗口拖拽区（旧 bar-title 同款） -->
    <header class="menu-bar" data-tauri-drag-region>
      <IconButton name="collapse-sidebar" @click="ui.leftCollapsed = true" />
      <ActionButton v-for="b in menuButtons" :key="b" :label="b" variant="ghost" size="sm" />
    </header>

    <!-- 模式切换区块（稿 左侧边栏-分段控制器）：54 高 / padding 8 12 -->
    <div class="seg-wrap">
      <SegmentedControl
        :model-value="viewMode"
        :options="segOptions"
        variant="icon"
        @update:model-value="onModeChange"
      />
    </div>

    <!-- 树容器（稿：flex-grow 1）：资源库 = LibraryView；工作空间 / AI = 空态占位；
         视图切换 = 旧内容左移出、新内容右移入（2026-08-30 用户实机定）300ms 缓入缓出（与滑块同步） -->
    <div class="side-body">
      <Transition name="view-x">
        <div v-if="viewMode === 'library'" ref="bodyRef" class="tree-scroll kit-scroll" key="library">
          <LibraryView />
        </div>
        <div v-else class="empty-state" key="empty">功能正在开发中</div>
      </Transition>
      <ScrollThumb :body="bodyRef" />
    </div>
  </aside>
</template>

<style scoped>
/* 左侧边栏（稿 左侧边栏-总容器）：column / padding 0 / 254 宽（ui.leftWidth，SplitBar 可拖）；
   无底色——壳块透明透窗口底（7.1 十七轮三段底色结构：实色区 = 内容容器 / 右栏各自画） */
.sidebar-left {
  position: relative;
  display: flex;
  flex-direction: column;
  flex: none;
  min-height: 0;
  overflow: hidden;
}
/* 菜单栏：46 高 / padding 8 12 / gap 2（稿值）；定宽 --sb-w（aside 收窄时裁剪不挤压） */
.menu-bar {
  display: flex;
  align-items: center;
  gap: var(--n-space-2);
  width: var(--sb-w);
  height: 46px;
  box-sizing: border-box;
  padding: var(--n-space-8) var(--n-space-12);
  flex: none;
}
/* 模式切换区块：54 高 / padding 8 12（稿值） */
.seg-wrap {
  width: var(--sb-w);
  box-sizing: border-box;
  padding: var(--n-space-8) var(--n-space-12);
  flex: none;
}
/* 树容器：flex 1 撑满剩余；relative 包裹层 = ScrollThumb 锚点（速查 §2 悬浮滚轮结构） */
.side-body {
  position: relative;
  flex: 1;
  min-height: 0;
  width: var(--sb-w);
}
.tree-scroll {
  height: 100%;
}
/* 视图切换动画（2026-08-30 用户实机定）：旧内容左移出、新内容右移入 + 渐隐；
   300ms 形态切换档 + 缓入缓出（与分段滑块位移同步）；离场 absolute 不占位（两内容同帧并行） */
.view-x-enter-active,
.view-x-leave-active {
  transition:
    transform var(--n-dur-300) var(--n-ease-in-out),
    opacity var(--n-dur-300) var(--n-ease-in-out);
}
.view-x-enter-from {
  transform: translateX(var(--n-space-24));
  opacity: 0;
}
.view-x-leave-to {
  transform: translateX(calc(var(--n-space-24) * -1));
  opacity: 0;
}
.view-x-leave-active {
  position: absolute;
  inset: 0;
}
/* 空态占位（工作空间 / AI 对话框）：一段文字居中（2026-08-30 用户定）；
   正文档 + 亮色/四级（占位符同档语义，输入框族口径） */
.empty-state {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l4);
  user-select: none;
}
</style>
