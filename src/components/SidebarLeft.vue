<script setup lang="ts">
// 左侧边栏（7.1 十八轮贴稿重构，稿 = docs/06_UI重构/Design Token/布局/主窗口/左侧边栏/）：
// 254 宽 column 三段——菜单栏 46（收起侧边栏 plain 钮 + 菜单/编辑/插件/视图 四文字钮，ghost sm；
// 「菜单」2026-09-05 接业务 = 应用主菜单（menuSpecs/appMenu，按钮下方弹出 + toggle）；
// 「编辑」2026-09-06 接业务 = 编辑菜单（menuSpecs/editMenu，同 toggle 口径）；
// 「视图」2026-09-06 接业务 = 视图菜单（menuSpecs/viewMenu，同 toggle 口径）；
// 「插件」2026-09-06 接业务 = 插件菜单（menuSpecs/pluginMenu，同 toggle 口径；同日二轮用户走查
// 改定：菜单栏钮 = 普通上下文菜单，400×450 插件列表浮层只留工具栏 plugin 图标钮）；
// 分段控制器 54（SegmentedControl icon 型：资源库 / 工作空间 / AI 对话框——三段可切换，
// 资源库 = LibraryView 树、工作空间 = WorkspaceTree 树（2026-09-17 接入），AI = 空态
// 「功能正在开发中」居中；模式与停留视口持久化见下方「分段控制器（模式切换）」段；
// 选择模式下禁用切换，视觉不改——稿无禁用态）；
// 树容器 flex 1（LibraryView：快速访问 + 文件夹区块；ScrollThumb 悬浮滚轮，速查 §2）。
// 收展动画 = sidebarShowWidths 唯一写点（速查 §8）：模板只绑它，toggle 只翻布尔；
// 本文件禁挂 width 的 CSS transition（与逐帧 JS 写宽打架 = 慢一帧）；
// --sb-w 恒为展开宽、子块定宽 = 收窄时右缘裁剪不挤压；展开按钮在标签页行首（SidebarExpandButton）。
import { computed, ref } from "vue";
import IconButton from "./buttons/IconButton.vue";
import ActionButton from "./buttons/ActionButton.vue";
import SegmentedControl from "./buttons/SegmentedControl.vue";
import ScrollThumb from "./ScrollThumb.vue";
import { SIDEBAR_SECTION_SPECS, sidebarSectionSpecFor } from "../lib/sidebarSpecs";
import { useUiStore } from "../stores/ui";
import { usePickerStore } from "../stores/picker";
import { useTabsStore } from "../stores/tabs";
import { sidebarShowWidths } from "../lib/useSidebarAnim";
import { closeContextMenu, menuState, openContextMenu } from "../lib/contextMenus";
import { appMenuSpec } from "../lib/menuSpecs/appMenu";
import { editMenuSpec } from "../lib/menuSpecs/editMenu";
import { pluginMenuSpec } from "../lib/menuSpecs/pluginMenu";
import { viewMenuSpec } from "../lib/menuSpecs/viewMenu";
import { loadLibraries } from "../lib/libraryOps";

const ui = useUiStore();
const picker = usePickerStore();

// 菜单栏四文字钮：菜单 = 应用主菜单（2026-09-05 接业务：稿 = 用户当场供稿，spec = menuSpecs/appMenu）；
// 编辑 = 编辑菜单（2026-09-06 接业务：spec = menuSpecs/editMenu，全项 = useGlobalKeys 快捷键的菜单面）；
// 视图 = 视图菜单（2026-09-06 接业务：spec = menuSpecs/viewMenu，查看方式 / 标签页 / 面板显隐）；
// 插件 = 插件菜单（2026-09-06 接业务：spec = menuSpecs/pluginMenu；同日二轮走查改定 = 普通上下文菜单）

// 「菜单」钮 toggle：同一按钮再点 = 关闭（trigger 豁免 AppMenu 组外判定，click 到这里做开关）；
// 选择模式下禁开（白名单收窄：主菜单含切库，速查 §12.1——视觉不改，稿无禁用态）
async function onAppMenu(e: MouseEvent): Promise<void> {
  const el = e.currentTarget as HTMLElement;
  if (menuState.entries && menuState.trigger === el) {
    closeContextMenu();
    return;
  }
  if (picker.active) return;
  const rect = el.getBoundingClientRect();
  const { names } = await loadLibraries();
  openContextMenu({ clientX: rect.left, clientY: rect.bottom + 4 }, appMenuSpec(names), undefined, { trigger: el });
}

// 「编辑」钮（2026-09-06 接业务）：同「菜单」钮 toggle 口径；选择模式禁开（全编辑类，§12.1 白名单）
function onEditMenu(e: MouseEvent): void {
  const el = e.currentTarget as HTMLElement;
  if (menuState.entries && menuState.trigger === el) {
    closeContextMenu();
    return;
  }
  if (picker.active) return;
  const rect = el.getBoundingClientRect();
  openContextMenu({ clientX: rect.left, clientY: rect.bottom + 4 }, editMenuSpec(), undefined, { trigger: el });
}

// 「插件」钮（2026-09-06 接业务，同日二轮走查改定 = 普通上下文菜单，同 toggle 口径）；
// 选择模式禁开（插件入口与挑选无关，同「菜单/编辑」钮闸——视觉不改，稿无禁用态）
function onPluginMenu(e: MouseEvent): void {
  const el = e.currentTarget as HTMLElement;
  if (menuState.entries && menuState.trigger === el) {
    closeContextMenu();
    return;
  }
  if (picker.active) return;
  const rect = el.getBoundingClientRect();
  openContextMenu({ clientX: rect.left, clientY: rect.bottom + 4 }, pluginMenuSpec(), undefined, { trigger: el });
}

// 「视图」钮（2026-09-06 接业务）：同 toggle 口径。选择模式下**可开**——§12.1 白名单含
// 查看方式（布局 / 递归显示 / 排序），菜单内非白名单项（管理弹窗 / 标签页 / 侧栏收展）各自禁用
function onViewMenu(e: MouseEvent): void {
  const el = e.currentTarget as HTMLElement;
  if (menuState.entries && menuState.trigger === el) {
    closeContextMenu();
    return;
  }
  const rect = el.getBoundingClientRect();
  openContextMenu({ clientX: rect.left, clientY: rect.bottom + 4 }, viewMenuSpec(), undefined, { trigger: el });
}

// 分段控制器（模式切换）：library = 资源库树；workspace = 工作空间树（PRD 02 步骤 1 只读底座，
// 2026-09-17 接入：真实目录树 + 懒加载子目录）；ai = 空态占位（功能正在开发中）。
// 模式与停留视口持久化（ui.leftMode / lastLibView / lastWsView，2026-09-19 用户定）：
// 切分段 = 恢复该模式停留视口（库 / 工作空间各自记忆，App.vue 按 ws: 前缀归桶持续记录）——
// 旧「切 workspace 仅无 ws 页才开根视口、切回库不动」停留对方目录口径废除；
// 反向同步（2026-10-03 用户定）：切标签页 / 分屏激活变化时 App.vue watch 回写 leftMode，
// 分段选择器跟随定位到当前视口所属视图；
// 选择模式（6.2 白名单收窄）下禁用切换——树 / 快速访问导航是挑选主通路，须恒在。
// AI 占位段不动中间容器。
const tabs = useTabsStore();
// 分段与内容分发同表驱动（7.3 T4 sidebarSpecs 注册表，2026-11-03）：
// 新分段 = SIDEBAR_SECTION_SPECS 加一条 + 内容组件，本文件零改动
const segOptions = SIDEBAR_SECTION_SPECS.map((s) => ({ value: s.key, icon: s.icon }));
const currentSpec = computed(() => sidebarSectionSpecFor(ui.leftMode));
function onModeChange(v: string): void {
  if (picker.active) return;
  ui.leftMode = v;
  // 切分段恢复该模式停留视口（spec.viewToOpen 封装停留记忆键；空串 = 占位段不动中间容器；
  // openView 同视口早退；锁定页按既有 Eagle 语义新开标签页）
  const target = sidebarSectionSpecFor(v).viewToOpen(ui);
  if (target) tabs.openView(target);
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
      <ActionButton label="菜单" variant="ghost" size="sm" @click="onAppMenu" />
      <ActionButton label="编辑" variant="ghost" size="sm" @click="onEditMenu" />
      <ActionButton label="插件" variant="ghost" size="sm" @click="onPluginMenu" />
      <ActionButton label="视图" variant="ghost" size="sm" @click="onViewMenu" />
    </header>

    <!-- 模式切换区块（稿 左侧边栏-分段控制器）：54 高 / padding 8 12 -->
    <div class="seg-wrap">
      <SegmentedControl
        :model-value="ui.leftMode"
        :options="segOptions"
        variant="icon"
        @update:model-value="onModeChange"
      />
    </div>

    <!-- 树容器（稿：flex-grow 1）：资源库 = LibraryView；工作空间 = WorkspaceTree（PRD 02 步骤 1）；
         AI = 空态占位；视图切换 = 旧内容左移出、新内容右移入（2026-08-30 用户实机定）300ms 缓入缓出（与滑块同步） -->
    <div class="side-body">
      <Transition name="view-x">
        <div v-if="currentSpec.component" ref="bodyRef" class="tree-scroll kit-scroll" :key="currentSpec.key">
          <component :is="currentSpec.component" />
        </div>
        <div v-else class="empty-state" :key="currentSpec.key">功能正在开发中</div>
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
/* 菜单栏：38 高 / padding 0 12（上下留白归 0 靠居中——内容物收起钮 30，上下各余 4）/
   gap 2（2026-10-01 用户定稿修订：46 → 38 降标题栏占位）；定宽 --sb-w（aside 收窄时裁剪不挤压） */
.menu-bar {
  display: flex;
  align-items: center;
  gap: var(--n-space-2);
  width: var(--sb-w);
  height: 38px;
  box-sizing: border-box;
  padding: 0 var(--n-space-12);
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
/* 视图切换动画（2026-08-30 用户实机定）：旧内容左移出、新内容右移入 + 渐隐；位移 48px。
   时序（同日实机四调定稿）：退场 150ms 缓出（--n-dur-150 + --n-ease-out）先行；
   入场 250ms 缓入（--n-dur-250 + --n-ease-in），启动 = 退场的 100ms 后（delay --n-dur-100，
   总 350ms）——全部 token 档；离场 absolute 不占位 */
.view-x-enter-active {
  transition:
    transform var(--n-dur-250) var(--n-ease-in) var(--n-dur-100),
    opacity var(--n-dur-250) var(--n-ease-in) var(--n-dur-100);
}
.view-x-leave-active {
  transition:
    transform var(--n-dur-150) var(--n-ease-out),
    opacity var(--n-dur-150) var(--n-ease-out);
}
.view-x-enter-from {
  transform: translateX(48px);
  opacity: 0;
}
.view-x-leave-to {
  transform: translateX(-48px);
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
