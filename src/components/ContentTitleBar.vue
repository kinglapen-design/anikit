<script setup lang="ts">
// 中间面板-内容容器-工具栏（7.1 内容容器轮贴稿，稿 = 布局/主窗口/中间面板/中间面板-内容容器-工具栏.md）：
// 54 高（30 内容 + padding 12）/ space-between / gap 10，无底色（透内容容器 通用/一级 g1 卡片底）。
// 左 = 面包屑导航（gap 12）：后退/前进钮组（gap 2，angle-bracket 尖括号：后退原样朝左、
//      前进 flip-h）+ 搜索结果计数（仅搜索态：
//      「搜索结果 / N」= 正文 二级 + 数字-大 三级，3.6 数据本轮接线回填；当前目录名由标签页承担
//      不进工具栏，2026-08-08 用户定）。
// 右 = 尺寸滑块（BarSlider tone=size，稿宽 100）+ 按钮组（gap 2）：添加文件/档案链接 / 插件
//      （2026-09-06 接业务 = 插件列表面板浮层 PluginListPanel，lib/pluginPanel 单例，同菜单栏
//      「插件」钮口径；旧 CepSettingsDialog 入口随本轮退役——CEP 导出设置挪 CEP 面板内编辑）/
//      分组管理 / 查看方式 / 右栏收展钮（恒显最右侧，7.1 二十三轮用户定——右栏新稿
//      无收起入口；收起态 = collapse-sidebar flip-h「展开详细信息」）；
//      缩略图 / AI 工具集两旧占位钮稿无，本轮撤下。
// 图标钮一律 buttons/IconButton plain（30×30 圆角 6、图标 18 三级、hover 半透明-3 底 + 升二级）；
// 禁用态视觉稿未画不做（速查表 §15），仅业务禁用。
// 递归显示开关移出口径不变（2026-08-08 用户定：归内容区区块标题行，随该区块改造时接入）。
import { ref, computed } from "vue";
import IconButton from "./buttons/IconButton.vue";
import BarSlider from "./bars/BarSlider.vue";
import ViewOptionsMenu from "./ViewOptionsMenu.vue";
import { useUiStore } from "../stores/ui";
import { usePickerStore } from "../stores/picker";
import { useSearchStore } from "../stores/search";
import { useTabsStore } from "../stores/tabs";
import { currentLinkTarget, enterLinkPicker } from "../lib/menuOps";
import { openPluginPanel } from "../lib/pluginPanel";

const ui = useUiStore();
const picker = usePickerStore();
const search = useSearchStore();
const tabs = useTabsStore();

// 本标签页视口（ContentArea 传入）；
// panelId（2026-08-20 面包屑接入）：导航历史 per-panel，按钮按本页历史栈禁用/触发
const props = defineProps<{ viewKey: string; panelId?: string }>();

// 面包屑后退 / 前进按钮态（本页历史栈；无历史禁用）
const canBack = computed(() => (props.panelId ? (tabs.navCan[props.panelId]?.back ?? false) : false));
const canFwd = computed(() => (props.panelId ? (tabs.navCan[props.panelId]?.fwd ?? false) : false));

// 搜索结果计数（3.6 接线）：搜索态显示「搜索结果 / N」，N = 文件命中 + 档案命中总数
const searching = computed(() => (props.panelId ? search.isActive(props.panelId) : false));
const searchCount = computed(() =>
  props.panelId ? search.hitsFor(props.panelId).size + search.hitArchivesFor(props.panelId).size : 0,
);

// 卡片尺寸滑块（稿宽 100）：BarSlider 0~1 归一化 ↔ targetRowHeight 80~gridWidth 实际像素换算
const SIZE_MIN = 80;
const sliderRatio = computed({
  get: () => (ui.targetRowHeight - SIZE_MIN) / Math.max(1, ui.gridWidth - SIZE_MIN),
  set: (r) => {
    ui.targetRowHeight = Math.round(SIZE_MIN + Math.min(1, Math.max(0, r)) * (ui.gridWidth - SIZE_MIN));
  },
});

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
// 递归显示时禁用（2026-08-19 复验定案：递归不分组——配置仅对直接子项生效）；
// 打开时锁定当前文件夹 id（弹窗打开后切视口不漂移）
const canGroup = computed(() => props.viewKey.startsWith("folder:") && !ui.showSubfolders);
function openGroupManage(): void {
  if (!canGroup.value) return;
  ui.groupDialogFolder = props.viewKey.slice(7);
  ui.groupManageOpen = true;
}

// 添加文件/档案链接（6.2 三轮点亮）：落点 = 本标签页文件夹视口（快速访问全部/回收站/收件箱不可添加禁用）；
// 选择模式进行中禁用（白名单只放行查看类，2026-08-24 收窄口径）
const linkTarget = computed(() => (picker.active ? null : currentLinkTarget(props.viewKey)));
function addFileLink(): void {
  if (linkTarget.value) enterLinkPicker(linkTarget.value);
}
</script>

<template>
  <div class="panel-toolbar" data-tauri-drag-region>
    <div class="crumb-group">
      <!-- 后退 / 前进（稿「返回+前进-按钮」组 gap 2）：angle-bracket 尖括号，前进 = flip-h 朝右；
           per-panel 导航历史（openView 替换视口入栈，锁定页新开不入栈） -->
      <div class="nav-pair">
        <IconButton
          name="angle-bracket"
          title="后退"
          :disabled="!canBack"
          @click="panelId && tabs.goHistory(panelId, -1)"
        />
        <IconButton
          name="angle-bracket"
          title="前进"
          class="flip-h"
          :disabled="!canFwd"
          @click="panelId && tabs.goHistory(panelId, 1)"
        />
      </div>
      <!-- 搜索结果计数（稿「搜索结果 / 123」）：仅搜索态显示 -->
      <span v-if="searching" class="result-group">
        <span class="result-label">搜索结果</span>
        <span class="result-sep">/</span>
        <span class="result-count">{{ searchCount.toLocaleString() }}</span>
      </span>
    </div>
    <div class="right-group">
      <div class="slider-wrap">
        <BarSlider v-model="sliderRatio" tone="size" />
      </div>
      <!-- 按钮组（稿）：添加文件/档案链接（6.2 三轮点亮 = 进选择模式，落点当前文件夹视口，无视口禁用）/
           插件（2026-09-06 接业务 = 插件列表面板浮层）/ 分组管理（3.5 二轮点亮，仅文件夹/档案视口；
           递归显示时禁用）/ 查看方式 / 展开详细信息（仅右栏收起时）；「链接文件管理」钮删除口径不变（2026-08-24 用户定） -->
      <div class="btn-group">
        <IconButton name="add-file-archive-link" title="添加文件/档案链接" :disabled="!linkTarget" @click="addFileLink" />
        <IconButton name="plugin" title="插件" @click="openPluginPanel" />
        <IconButton
          name="group-manage"
          :title="viewKey.startsWith('folder:') && ui.showSubfolders ? '分组管理（递归显示时不分组）' : '分组管理'"
          :disabled="!canGroup"
          @click="openGroupManage"
        />
        <span ref="viewBtnRef" style="display: contents">
          <IconButton name="view-options" title="查看方式" @click="openViewMenu" />
        </span>
        <!-- 右栏收展钮（7.1 二十三轮用户定：恒显工具栏最右侧，不再仅收起态出现——
             右栏新稿无收起入口，原右栏 54 工具栏废除；收起态 flip-h + 「展开详细信息」） -->
        <IconButton
          name="collapse-sidebar"
          :title="ui.rightCollapsed ? '展开详细信息' : '收起详细信息'"
          :class="{ 'flip-h': ui.rightCollapsed }"
          @click="ui.rightCollapsed = !ui.rightCollapsed"
        />
      </div>
    </div>
    <ViewOptionsMenu v-if="viewMenuOpen" :anchor="viewMenuAnchor" :trigger="viewBtnRef ?? null" @close="viewMenuOpen = false" />
  </div>
</template>

<style scoped>
/* 工具栏（稿）：54（30 + padding 12×2）/ space-between / gap 10（figma 自动占位值，按 token 档用 8，§14 先例）；无底色（透内容容器 g1） */
.panel-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--n-space-8);
  padding: var(--n-space-12);
  flex: none;
}
/* 面包屑导航（稿 gap 12） */
.crumb-group {
  display: flex;
  align-items: center;
  gap: var(--n-space-12);
  min-width: 0;
}
/* 返回+前进按钮组（稿 gap 2） */
.nav-pair {
  display: flex;
  align-items: center;
  gap: var(--n-space-2);
}
/* 搜索结果计数（稿）：「搜索结果 / N」组 gap 4；文字 = 正文 二级，数字 = 数字-大 三级 */
.result-group {
  display: flex;
  align-items: center;
  gap: var(--n-space-4);
  white-space: nowrap;
}
.result-label,
.result-sep {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l2);
}
.result-count {
  font-family: var(--n-font-mono);
  font-size: var(--n-fs-num-lg);
  line-height: var(--n-lh-num-lg);
  font-weight: var(--n-fw-medium);
  color: var(--n-l3);
}
/* 尺寸滑块+按钮组（稿 gap 12）；滑块稿宽 100 */
.right-group {
  display: flex;
  align-items: center;
  gap: var(--n-space-12);
  flex: none;
}
.slider-wrap {
  width: 100px;
  flex: none;
}
/* 按钮组（稿 gap 2） */
.btn-group {
  display: flex;
  align-items: center;
  gap: var(--n-space-2);
}
/* 前进 / 展开详细信息 = 左向图标水平翻转 */
.flip-h :deep(svg) {
  transform: scaleX(-1);
}
</style>
