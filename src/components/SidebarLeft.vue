<script setup lang="ts">
// 左侧边栏（1.2 重构，figma-css 01_主窗口 左侧边栏段）：
// 标题栏 46（C3 + 下边框 暗-悬停：汉堡菜单 + 右对齐按钮组[新增 / 区块管理 / 收起侧边栏]）
// 内容 row：导航栏 54（C3 + 右边框：资源库[选中] / 工作空间 / AI 对话框 + 帮助）
// ｜ 内容区 250（C3 + 右边框：资源库切换 54 + 树容器）。
// 无业务按钮一律 disabled（1.3 占位点亮机制统一接入）：工作空间 / AI 对话框 / 帮助。
// 区块管理 = 30 稿弹窗（3.1 一轮点亮，ui.blockManageOpen → App.vue 宿主 BlockManageDialog）。
// 新增 = 07 新增菜单三入口；汉堡 = 14 菜单；切换资源库 = 08 菜单（2026-08-11 点亮）。
// 收起 = 整栏收掉（App.vue v-if + 滑动动画，2026-08-08 定案），
// 展开按钮在标签页行首（SidebarExpandButton，dockview prefix actions，不占工具栏）。
import { onBeforeUnmount, onMounted, ref } from "vue";
import AppIcon from "./AppIcon.vue";
import IconButton from "./IconButton.vue";
import LibraryView from "./LibraryView.vue";
import { closeContextMenu, menuState, openContextMenu } from "../lib/contextMenus";
import { addButtonSpec } from "../lib/menuSpecs/addMenu";
import { hamburgerSpec, librarySwitchSpec } from "../lib/menuSpecs/libraryHamburger";
import { loadLibraries, currentLibrary, currentSpace, type LibraryInfo, type SpaceInfo } from "../lib/libraryOps";
import { invoke } from "@tauri-apps/api/core";
import { isEditableTarget } from "../lib/useGlobalKeys";
import { useUiStore } from "../stores/ui";
import { useSideThumb } from "../lib/useSideThumb";

const ui = useUiStore();
const bodyRef = ref<HTMLElement>();
const thumb = useSideThumb(bodyRef);
// 滚动条悬停显隐（2026-08-10 用户定：悬停面板 / 拖拽 thumb 时才出现）
const hover = ref(false);
const addBtnRef = ref<HTMLElement>();

// 新增菜单（07 稿，1.3）：三入口共用一套（2026-08-11 用户定）——
// ① 本按钮 click ② 网格空白处右键（menuSpecs.openGridContextMenu）③ Ctrl+N（下方全局监听）；
// 菜单固定弹在按钮下方（07 稿 320 宽无搜索框），spec 在 lib/menuSpecs（导入动作走 menuOps.pickImportPaths）
function toggleAddMenu(): void {
  if (menuState.entries) {
    closeContextMenu();
    return;
  }
  const rect = addBtnRef.value?.getBoundingClientRect();
  if (!rect) return;
  openContextMenu({ clientX: rect.left, clientY: rect.bottom + 4 }, addButtonSpec(), undefined, { width: 320 });
}

// Ctrl+N 调出新增菜单（按钮位弹出）：输入态排除；Ctrl+Shift+N（菜单内"新建文件夹"）不拦截；
// 侧栏收起时按钮不可见不响应
function onGlobalKeydown(e: KeyboardEvent): void {
  if (!(e.ctrlKey || e.metaKey) || e.shiftKey || e.altKey || e.key.toLowerCase() !== "n") return;
  if (ui.leftCollapsed) return;
  if (isEditableTarget(e.target as HTMLElement)) return;
  e.preventDefault();
  toggleAddMenu();
}
onMounted(() => {
  window.addEventListener("keydown", onGlobalKeydown);
  // 库名 / 空间设置兜底（2.1）：启动直开分支不经 libraryOps 封装，此处补拉一次
  if (currentLibrary.name === null) {
    void invoke<LibraryInfo | null>("get_current_library")
      .then((lib) => {
        currentLibrary.name = lib?.name ?? null;
      })
      .catch(() => {});
  }
  if (currentSpace.name === null) {
    void invoke<SpaceInfo | null>("get_current_space")
      .then((space) => {
        currentSpace.name = space?.name ?? null;
        currentSpace.autoImport = space?.autoImport ?? false;
      })
      .catch(() => {});
  }
});
onBeforeUnmount(() => window.removeEventListener("keydown", onGlobalKeydown));

// 汉堡菜单（14 稿，宽 220）/ 切换资源库（08 稿，宽 300）：toggle 模式同新增按钮；
// 库列表为动态数据（当前空间内库 + 当前库名，2.1 空间化），开菜单前加载
async function toggleHamburger(e: MouseEvent): Promise<void> {
  if (menuState.entries) {
    closeContextMenu();
    return;
  }
  const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
  const { names, currentName } = await loadLibraries();
  openContextMenu({ clientX: rect.left, clientY: rect.bottom + 4 }, hamburgerSpec(names, currentName), undefined, {
    width: 220,
  });
}
async function toggleLibrarySwitch(e: MouseEvent): Promise<void> {
  if (menuState.entries) {
    closeContextMenu();
    return;
  }
  const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
  const { names, currentName } = await loadLibraries();
  openContextMenu({ clientX: rect.left, clientY: rect.bottom + 4 }, librarySwitchSpec(names, currentName), undefined, {
    width: 300,
  });
}
</script>

<template>
  <aside
    class="sidebar-left"
    :style="{ width: ui.leftCollapsed ? '0px' : `${ui.leftWidth}px`, '--sb-w': `${ui.leftWidth}px` }"
    @mouseenter="hover = true"
    @mouseleave="hover = false"
  >
    <header class="bar-title" data-tauri-drag-region>
      <IconButton name="menu" title="菜单" @click="toggleHamburger" />
      <div class="btn-group">
        <span ref="addBtnRef" class="add-btn-wrap">
          <IconButton name="add-large" :title="{ text: '新增', keys: 'Ctrl+N' }" @click="toggleAddMenu" />
        </span>
        <IconButton name="section-manage" title="区块管理" @click="ui.blockManageOpen = true" />
        <IconButton name="collapse-sidebar" title="收起侧边栏" @click="ui.leftCollapsed = true" />
      </div>
    </header>

    <div class="body">
      <!-- 导航栏 54：常驻（收起时不消失）；视图切换（资源库选中；工作空间 / AI 对话框待对应阶段）+ 帮助 -->
      <nav class="rail">
        <div class="rail-views">
          <IconButton name="library" title="资源库" :active="true" />
          <IconButton name="workspace" title="工作空间" disabled />
          <IconButton name="ai-chat" title="AI 对话框" disabled />
        </div>
        <IconButton name="help" title="帮助" class="help-btn" disabled />
      </nav>

      <!-- 内容区 250：库切换 + 树容器；收展 = 宽度动画（整栏不销毁，防抖） -->
      <div class="side-content">
        <div class="library-switch">
          <span class="lib-badge"><AppIcon name="library-duotone" :size="18" /></span>
          <span class="lib-name">{{ currentLibrary.name ?? "未打开资源库" }}</span>
          <IconButton name="switch-library" title="切换资源库" @click="toggleLibrarySwitch" />
        </div>
        <div ref="bodyRef" class="tree-wrap">
          <LibraryView />
        </div>
        <span
          class="side-thumb"
          :class="{ visible: thumb.show && (hover || thumb.dragging) }"
          :style="{ top: `${thumb.top}px`, height: `${thumb.height}px` }"
          @pointerdown="thumb.onThumbDown"
        />
      </div>
    </div>
  </aside>
</template>

<style scoped>
/* 左侧边栏（figma 39）：column，无底色（各子块提供），宽度可拖（SplitBar，默认 304 = 54 + 250）；
   收展 = 常驻挂载 + 宽度由 App.vue rAF 驱动逐帧写入（2026-08-08 定案 v6：dockview 的 RO 回调
   被 rAF 包裹恒晚一帧，CSS 过渡/挂载切换必撕一帧；同帧写宽+强制 dockview 布局才零滞后）。
   本文件禁挂 width 的 CSS transition（会与逐帧 JS 写宽打架 = 慢一帧）；
   --sb-w 恒为展开宽、子块定宽 = 收窄时右缘裁剪不挤压；展开按钮在标签页行首（SidebarExpandButton） */
.sidebar-left {
  position: relative;
  display: flex;
  flex-direction: column;
  flex: none;
  min-height: 0;
  overflow: hidden;
}
/* 标题栏（figma 58）：46（含边框，box-sizing——与中间标签栏行对齐，2026-08-08 走查修正）/
   C3 / 下边框 暗-悬停 / padding 8 16 8 12 / space-between */
.bar-title {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-12);
  width: var(--sb-w); /* 定宽：aside 收窄时裁剪不挤压 */
  height: 46px;
  box-sizing: border-box;
  padding: var(--space-8) var(--space-16) var(--space-8) var(--space-12);
  flex: none;
  background: var(--c3);
  border-bottom: 1px solid var(--line-hover);
}
/* 右对齐按钮组（figma 137）：间距 4 */
.btn-group {
  display: flex;
  align-items: center;
  gap: var(--space-4);
}
/* 新增按钮定位壳（菜单弹位 / Ctrl+N rect 来源） */
.add-btn-wrap {
  display: inline-flex;
}
/* 内容（figma 312）：row 撑满剩余；定宽 --sb-w（aside 收窄时右缘裁剪不挤压） */
.body {
  display: flex;
  flex: 1;
  width: var(--sb-w);
  min-height: 0;
}
/* 导航栏（figma 330）：54px / C3 / 右边框 暗-悬停 / padding 12 / space-between */
.rail {
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  gap: var(--space-12);
  padding: var(--space-12);
  width: 54px;
  flex: none;
  box-sizing: border-box;
  background: var(--c3);
  border-right: 1px solid var(--line-hover);
}
/* 视图切换按钮组（figma 357）：间距 8；选中态 = 亮-选中 底（IconButton active） */
.rail-views {
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
}
/* 帮助（figma 567）：C4 底 + 暗-悬停边框 + 图标极淡档。class 落在 IconButton 根元素（button 本身），
   直接作用 .help-btn（单根组件根元素带本 scope id）；!important 压 IconButton disabled 复位；
   悬停底色 C4 → C5（2026-08-08 用户定） */
.help-btn,
.help-btn:disabled {
  background: var(--c4) !important;
  box-shadow: inset 0 0 0 1px var(--line-hover) !important;
  color: var(--text-faint);
  opacity: 1;
}
.help-btn:disabled:hover {
  background: var(--c5) !important;
}
/* 内容区（figma 607）：250 / C3 / 右边框 暗-悬停；拖宽时导航栏固定 54、本区弹性；
   收起时随 aside 收窄被压至 0（flex 1 + min-width 0 + 溢出裁切，无需单独动画） */
.side-content {
  position: relative;
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  overflow: hidden;
  background: var(--c3);
  border-right: 1px solid var(--line-hover);
}
/* 资源库切换按钮（figma 632）：54 / padding 12 16 / 下边框 暗-悬停 */
.library-switch {
  display: flex;
  align-items: center;
  gap: var(--space-8);
  padding: var(--space-12) var(--space-16);
  flex: none;
  border-bottom: 1px solid var(--line-hover);
}
/* 资源库-双色徽章（figma 696）：30×30 品牌色底 + 亮-选中边框 + 圆角 6 */
.lib-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  margin-left: var(--space-8); /* figma 标题+切换容器 padding-left 8 */
  border-radius: var(--radius-6);
  background: var(--brand);
  border: 1px solid var(--line-active);
  color: var(--text-highlight);
  flex: none;
}
/* 库名（figma 758）：标题 16/22 常态档 */
.lib-name {
  flex: 1;
  min-width: 0;
  font-size: var(--text-title-size);
  line-height: var(--text-title-line);
  font-weight: var(--text-title-weight);
  color: var(--text-normal);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
/* 树容器（figma 833）：padding 12 16 0 */
.tree-wrap {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  padding: var(--space-12) var(--space-16) 0;
}
/* 悬浮滚轮条（2026-08-10 用户定二轮：6px 胶囊紧贴面板右边框 / 实色 常态 C4、悬停 C5——
   透明色在通栏分割线上交叉叠加、4px 间距让分隔线视觉截断，均作废；不占布局）；
   悬停显隐：默认透明不接收事件，visible（悬停面板 / 拖拽中）淡入 */
.side-thumb {
  position: absolute;
  right: 0;
  width: 6px;
  border-radius: var(--radius-pill);
  background: var(--c4);
  z-index: 101; /* 盖过窗口边缘 resize 热区 100（2026-07-23 用户定） */
  opacity: 0;
  pointer-events: none;
  transition:
    opacity 0.15s ease,
    background 0.12s ease;
}
.side-thumb.visible {
  opacity: 1;
  pointer-events: auto;
}
.side-thumb:hover {
  background: var(--c5);
}
</style>
