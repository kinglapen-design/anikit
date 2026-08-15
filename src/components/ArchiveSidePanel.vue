<script setup lang="ts">
// 档案二级侧边栏（2026-08-15 任务 A，PRD 02-03）：左侧边栏与中间面板之间，仅档案视口挂载。
// 布局（2026-08-15 用户截图红框修正）：标签栏行与底部搜索栏行横贯档案侧栏与中栏——
// 侧栏只存在于两行之间；实现 = 上填充条（46 / C3 / 下边框，标签栏延续）+
// 面板体（border-right 分隔线）+ 下填充条（46 / C3 / 上边框，搜索栏延续）。
// 宽 250（同右栏）/ C3；收展宽度 = sidebarShowWidths.archive 单一写点（useSidebarAnim 逐帧驱动，
// 同左右栏机制无闪帧；本组件挂载时按 ui.archivePanelOpen 快照初始宽，不产生开合动画）。
// 内容 = 档案详情区块栈（共享组合 DetailSections，与右侧详情面板同一套：pinned + 7 排序区块，
// 只读一轮）；数据源 get_archive_data（ArchiveDetailView：数据 + 统计，失败/无记录用默认空）。
import { computed, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import ScrollThumb from "./ScrollThumb.vue";
import DetailSections from "./DetailSections.vue";
import { ARCHIVE_PANEL_W, sidebarShowWidths } from "../lib/useSidebarAnim";
import { emptyArchiveData, type DetailTarget } from "../lib/detailSections";
import { useUiStore } from "../stores/ui";
import type { ArchiveDetailView, FolderNode } from "../stores/content";

const props = defineProps<{ node: FolderNode }>();
const ui = useUiStore();
const bodyRef = ref<HTMLElement>();

// 挂载快照（进档案视口不播动画：开 = 全宽 / 关 = 0，toggle 才走逐帧）
onMounted(() => {
  sidebarShowWidths.archive = ui.archivePanelOpen ? ARCHIVE_PANEL_W : 0;
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
  <aside class="archive-side" :style="{ width: `${sidebarShowWidths.archive}px` }">
    <div class="as-inner">
      <!-- 上填充条：标签栏行延续（46 / C3 / 下边框，与 dockview 标签条同值） -->
      <div class="as-filler top" />
      <div class="as-main">
        <!-- 标题区：档案名（真实数据；操作钮等随数据阶段接入） -->
        <header class="as-title">{{ node.name }}</header>
        <!-- 内容滚动区：档案详情区块栈（DetailSections 共享组合；ScrollThumb 口径） -->
        <div class="scroll-wrap">
          <div ref="bodyRef" class="as-body kit-scroll">
            <DetailSections :target="target" />
          </div>
          <ScrollThumb :body="bodyRef" />
        </div>
      </div>
      <!-- 下填充条：搜索栏行延续（46 / C3 / 上边框，与 SearchBar 同值） -->
      <div class="as-filler bottom" />
    </div>
  </aside>
</template>

<style scoped>
/* 档案二级侧边栏：250 定宽内层 + align-end（收窄时内容锚右缘、左缘裁剪，同左右栏 v6 口径；
   本文件禁挂 width 的 CSS transition——宽度由 useSidebarAnim 逐帧写） */
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
}
/* 面板体：两行之间，右缘 1px 暗-悬停分隔线（填充条区域无线 = 两行横贯） */
.as-main {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  border-right: 1px solid var(--line-hover);
}
/* 填充条（2026-08-15 红框修正）：46 高 C3；上 = 标签栏延续（下边框），下 = 搜索栏延续（上边框）。
   下条 47：搜索栏是 46 内容 + 1px 上边框（auto 高），填充条须含同等边框才对齐（CDP 实测 1px 差） */
.as-filler {
  height: 46px;
  box-sizing: border-box;
  flex: none;
}
.as-filler.top {
  border-bottom: 1px solid var(--line-hover);
}
.as-filler.bottom {
  height: 47px;
  border-top: 1px solid var(--line-hover);
}
/* 标题区：46 高 / padding 12 16 / 下边框 暗-悬停（主窗口既有面板口径） */
.as-title {
  display: flex;
  align-items: center;
  height: 46px;
  box-sizing: border-box;
  padding: var(--space-12) var(--space-16);
  flex: none;
  border-bottom: 1px solid var(--line-hover);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-highlight);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
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
