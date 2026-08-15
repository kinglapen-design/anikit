<script setup lang="ts">
// 档案二级侧边栏（2026-08-15 迁入标签页视口，PRD 02-03）：视口内部组成——当前标签页视口
// 为档案（folder kind === "archive"）时由 ContentArea 渲染在视口内容区最左；标签栏不被推挤，
// 分屏 / 多标签各管各；搜索栏行在视口内通栏（面板不占其高度）。
// 顶部工具行（54，照抄右侧详情面板工具栏 SidebarRight .panel-toolbar：54 高 /
// padding 12 16 / gap 4 / border-bottom 暗-悬停 / C3）：收起钮（collapse-detail 同款图标，
// 面板在左侧收起朝左 = 水平镜像）+ 编辑数据区块铅笔（edit-data-section，开同一个 22 稿
// SectionEditPanel，host = archive）。
// 展开入口照右栏现成模式：面板展开时中栏工具栏不放钮，收起时"展开面板"钮归位中栏工具栏
// （ContentTitleBar，collapse-detail 无镜像无 active 底色）。
import { computed, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import IconButton from "./IconButton.vue";
import ScrollThumb from "./ScrollThumb.vue";
import DetailSections from "./DetailSections.vue";
import SectionEditPanel from "./SectionEditPanel.vue";
import { ARCHIVE_PANEL_W, animateArchivePanel, sidebarShowWidths } from "../lib/useSidebarAnim";
import { emptyArchiveData, type DetailTarget } from "../lib/detailSections";
import { useUiStore } from "../stores/ui";
import type { ArchiveDetailView, FolderNode } from "../stores/content";

const props = defineProps<{ node: FolderNode; panelActive?: boolean }>();
const ui = useUiStore();
const rootRef = ref<HTMLElement>();
const bodyRef = ref<HTMLElement>();

// 22 稿「编辑数据区块」面板（host = archive；按钮右缘对齐、下方展开，同 SidebarRight 口径）
const sectionAnchor = ref({ x: 0, y: 0 });
const sectionBtnRef = ref<HTMLElement>();
function toggleSectionEdit(e: MouseEvent): void {
  if (ui.sectionEditOpen && ui.sectionEditHost === "archive") {
    ui.sectionEditOpen = false;
    return;
  }
  const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
  sectionAnchor.value = { x: rect.right - 175, y: rect.bottom + 4 };
  ui.sectionEditHost = "archive";
  ui.sectionEditOpen = true;
}

// 本档案开关态（per-folder map，缺省 true）
const open = computed(() => ui.archivePanelOpenFor(props.node.id));

// 宽度快照（无动画）：挂载时 + 本标签页重新激活时按本档案开关态归位——
// 多档案标签页共享 sidebarShowWidths.archive，激活快照防"别页宽度串到本页"
function snapWidth(): void {
  sidebarShowWidths.archive = open.value ? ARCHIVE_PANEL_W : 0;
}
onMounted(snapWidth);
watch(
  () => props.panelActive,
  (a) => {
    if (a) snapWidth();
  },
);

// 收展动画（animateArchivePanel：rAF 逐帧写 sidebarShowWidths.archive，与左右栏同参数）
watch(open, (isOpen, was) => {
  if (isOpen === was) return;
  const el = rootRef.value;
  if (el) animateArchivePanel(el, isOpen ? ARCHIVE_PANEL_W : 0);
});

// 档案详情（ArchiveDetailView：数据 + 文件数/占用空间统计；防 await 错配同 DetailPanel 口径）
const detail = ref<ArchiveDetailView | null>(null);
watch(
  () => props.node.id,
  async (id) => {
    detail.value = null;
    try {
      const d = await invoke<ArchiveDetailView>("get_archive_data", { folderId: id });
      if (props.node.id === id) detail.value = d;
    } catch (e) {
      console.warn("档案详情拉取失败（静默降级）：", e);
    }
  },
  { immediate: true },
);

const target = computed<DetailTarget>(() => ({
  kind: "archive",
  node: props.node,
  data: detail.value ?? emptyArchiveData(props.node.id),
}));
</script>

<template>
  <aside ref="rootRef" class="archive-side" :style="{ width: `${sidebarShowWidths.archive}px` }">
    <div class="as-inner">
      <!-- 顶部工具行（54，右侧详情面板工具栏同款）：右 = 收起（collapse-detail 镜像朝左）+
           编辑数据区块铅笔（edit-data-section，开 22 稿面板 host=archive） -->
      <div class="as-tools">
        <span ref="sectionBtnRef" style="display: contents">
          <IconButton name="edit-data-section" title="编辑数据区块" @click="toggleSectionEdit" />
        </span>
        <IconButton name="collapse-detail" title="收起档案面板" class="as-collapse flip-h" @click="ui.toggleArchivePanel(node.id)" />
      </div>
      <!-- 内容滚动区：档案详情区块栈（DetailSections 共享组合；ScrollThumb 口径） -->
      <div class="scroll-wrap">
        <div ref="bodyRef" class="as-body kit-scroll">
          <DetailSections :target="target" />
        </div>
        <ScrollThumb :body="bodyRef" />
      </div>
    </div>
    <!-- 22 稿「编辑数据区块」面板（与右栏工具栏同一个，host 分流） -->
    <SectionEditPanel
      v-if="ui.sectionEditOpen && ui.sectionEditHost === 'archive'"
      :anchor="sectionAnchor"
      :trigger="sectionBtnRef ?? null"
      @close="ui.sectionEditOpen = false"
    />
  </aside>
</template>

<style scoped>
/* 档案二级侧边栏（视口内）：250 定宽内层 + align-end（收窄时内容锚右缘、左缘裁剪，
   同左右栏 v6 口径；本文件禁挂 width 的 CSS transition——宽度由 animateArchivePanel 逐帧写）；
   右缘 1px 暗-悬停分隔线全高（面板底 = 搜索栏顶，搜索栏在视口内通栏不占） */
.archive-side {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  flex: none;
  min-height: 0;
  overflow: hidden;
  background: var(--c3);
}
.as-inner {
  display: flex;
  flex-direction: column;
  width: 250px;
  min-height: 0;
  flex: 1;
  border-right: 1px solid var(--line-hover);
}
/* 顶部工具行（照抄右侧详情面板工具栏 .panel-toolbar）：54 高 / padding 12 16 / 间距 4 /
   右对齐 / 下边框 暗-悬停 / C3（面板底色） */
.as-tools {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: var(--space-4);
  height: 54px;
  box-sizing: border-box;
  padding: var(--space-12) var(--space-16);
  flex: none;
  border-bottom: 1px solid var(--line-hover);
}
/* 收起钮：collapse-detail 源图形朝右，面板在左侧收起朝左 = 水平镜像（同 ContentTitleBar flip-h） */
.flip-h :deep(svg) {
  transform: scaleX(-1);
}
.scroll-wrap {
  position: relative;
  flex: 1;
  min-height: 0;
}
.as-body {
  height: 100%;
  overflow-y: auto;
  overflow-x: hidden;
}
</style>
