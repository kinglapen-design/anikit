<script setup lang="ts">
// 中间面板工具栏（1.2 重构，figma-css 02_布局/41_工具栏）：54px / padding 12 16 / space-between。
// 左 = 后退 / 前进 + 搜索结果（仅搜索时显示——当前所在目录由标签页承担，工具栏不再显示目录名，
// 2026-08-08 用户定；搜索业务 3.6 接入，本轮搜索结果无数据源不渲染）。
// 左侧位置留给档案看板的收起按钮（档案业务接入时启用）；
// 侧栏收展不再经本栏（左栏展开按钮在左栏汉堡位、右栏展开/收起到右栏顶行，2026-08-08 收展定案）。
// 右 = 卡片尺寸滑块 + 按钮组（链接文件管理 / 添加文件链接 / 缩略图 / AI 工具集 / 分组管理
// disabled 待 1.3 点亮 + 查看方式）+（右栏收起时：展开详情面板 + 窗口置顶 + 窗口控制——注释 6）。
// 递归显示开关已移出（2026-08-08 用户定：归内容区区块标题行，随该区块改造时接入）。
import { ref, computed } from "vue";
import IconButton from "./IconButton.vue";
import SideSlider from "./SideSlider.vue";
import ViewOptionsMenu from "./ViewOptionsMenu.vue";
import { useUiStore } from "../stores/ui";
import { useContentStore, type FolderNode } from "../stores/content";
import { findFolderNode } from "../lib/detailSections";

const ui = useUiStore();
const content = useContentStore();

// 本标签页视口（ContentArea 传入；档案切换钮按本视口档案读写，2026-08-15 迁入视口）
const props = defineProps<{ viewKey: string }>();

// 档案二级侧边栏切换钮（仅本视口 = 档案文件夹时显示，面包屑返回按钮左边）
const archiveViewportNode = computed<FolderNode | null>(() => {
  if (!props.viewKey.startsWith("folder:")) return null;
  const n = findFolderNode(content.folders, props.viewKey.slice(7));
  return n?.kind === "archive" ? n : null;
});

// 搜索结果文案（3.6 分词搜索接入后回填："搜索结果 - 12"）；null = 常态不显示
const searchResult = ref<string | null>(null);

// 查看方式菜单：按钮右缘对齐、下方展开；再次点击按钮收起（toggle）
const viewMenuOpen = ref(false);
const viewMenuAnchor = ref({ x: 0, y: 0 });
const viewBtnRef = ref<HTMLElement>();
function openViewMenu(e: MouseEvent): void {
  if (viewMenuOpen.value) {
    viewMenuOpen.value = false;
    return;
  }
  const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
  viewMenuAnchor.value = { x: rect.right - 175, y: rect.bottom + 4 };
  viewMenuOpen.value = true;
}

// 分组管理（3.5 二轮点亮）：仅文件夹/档案视口可用（快速访问无分组——规则无存放节点，用户定）；
// 打开时锁定当前文件夹 id（弹窗打开后切视口不漂移）
const canGroup = computed(() => props.viewKey.startsWith("folder:"));
function openGroupManage(): void {
  if (!canGroup.value) return;
  ui.groupDialogFolder = props.viewKey.slice(7);
  ui.groupManageOpen = true;
}
</script>

<template>
  <div class="panel-toolbar" data-tauri-drag-region>
    <div class="crumb-group">
      <!-- 档案面板展开钮（2026-08-15 走查修正，照右栏"展开详情面板"归位中栏工具栏的现成模式）：
           仅本视口为档案且面板已收起时出现；collapse-detail 图标、无 active/选中底色；
           面板展开时此处不放任何按钮 -->
      <IconButton
        v-if="archiveViewportNode && !ui.archivePanelOpenFor(archiveViewportNode.id)"
        name="collapse-detail"
        title="展开档案面板"
        class="archive-panel-toggle"
        @click="ui.toggleArchivePanel(archiveViewportNode.id)"
      />
      <!-- 后退 / 前进（figma 41：nav-history 图标，前进 rotate 180°；历史导航业务后接，先 disabled） -->
      <IconButton name="nav-history" title="后退" disabled />
      <IconButton name="nav-history" title="前进" class="rot-180" disabled />
      <span v-if="searchResult" class="result-text">{{ searchResult }}</span>
    </div>
    <div class="right-group">
      <SideSlider v-model="ui.targetRowHeight" :min="80" :max="ui.gridWidth" />
      <!-- 按钮组（figma 41：链接文件管理 / 添加文件链接 / 缩略图 / AI 工具集 / 分组管理 / 查看方式）；
           前四个业务未接一律 disabled（1.3 占位点亮机制统一接入）；分组管理 3.5 二轮点亮（仅文件夹/档案视口） -->
      <div class="btn-group">
        <IconButton name="linked-file-manage" title="链接文件管理" disabled />
        <IconButton name="add-file-link" title="添加文件链接" disabled />
        <IconButton name="thumbnail" title="缩略图" disabled />
        <IconButton name="ai-tools" title="AI 工具集" disabled />
        <IconButton name="group-manage" title="分组管理" :disabled="!canGroup" @click="openGroupManage" />
        <span ref="viewBtnRef" style="display: contents">
          <IconButton name="view-options" title="查看方式" @click="openViewMenu" />
        </span>
        <!-- 右栏收起：仅"展开详情面板"进工具栏（窗口控制组在 App 层右上角浮块，不参与收展） -->
        <IconButton
          v-if="ui.rightCollapsed"
          name="collapse-detail"
          title="展开详情面板"
          class="flip-h"
          @click="ui.rightCollapsed = false"
        />
      </div>
    </div>
    <ViewOptionsMenu v-if="viewMenuOpen" :anchor="viewMenuAnchor" :trigger="viewBtnRef ?? null" @close="viewMenuOpen = false" />
  </div>
</template>

<style scoped>
/* 工具栏（figma 9560）：54 / padding 12 16 / space-between / gap 10；无底色（随中间面板 C1） */
.panel-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: var(--space-12) var(--space-16);
  flex: none;
}
/* 中栏工具栏图标按钮 80% 亮度（2026-07-23 用户定：面板深，满亮太抢眼） */
.panel-toolbar :deep(.icon-btn) {
  color: var(--text-normal);
}
/* 面包屑容器：展开按钮 + 后退/前进 + 搜索结果，间距 4 */
.crumb-group {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  min-width: 0;
}
/* 搜索结果（figma 41）：正文 14/20，标签档 */
.result-text {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-label);
  white-space: nowrap;
}
/* 右对齐按钮组（figma 41）：滑块 + 按钮组，间距 12；按钮组内间距 4 */
.right-group {
  display: flex;
  align-items: center;
  gap: var(--space-12);
  flex: none;
}
.btn-group {
  display: flex;
  align-items: center;
  gap: var(--space-4);
}
/* 收起 / 展开同图标翻转使用 */
.flip-h :deep(svg) {
  transform: scaleX(-1);
}
/* 前进 = nav-history rotate 180°（figma 41 口径） */
.rot-180 :deep(svg) {
  transform: rotate(180deg);
}
</style>
