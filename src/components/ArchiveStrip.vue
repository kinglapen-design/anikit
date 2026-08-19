<script setup lang="ts">
// 档案卡片区（2026-08-15 档案子系统一轮，PRD 02-03 中间面板三区：文件夹 / 档案 / 文件）：
// SectionHeader「档案 - N」+ 固定长宽比档案卡片横排（figma 42 档案卡片区块，卡片 260×298 间距 24）；
// 无档案时整块不渲染（AssetGrid v-if，同"子项无文件夹隐藏区块"既有口径）；
// 交互口径同 FolderStrip（单击选中 / 双击进入 / 右键 04 稿档案菜单），随网格统一滚动。
import { computed, ref } from "vue";
import SectionHeader from "./SectionHeader.vue";
import ArchiveCard from "./ArchiveCard.vue";
import { dragView } from "../lib/dragOperation";
import { openArchiveCardMenu } from "../lib/menuSpecs/gridDispatch";
import { useContentStore, type FolderNode } from "../stores/content";
import { useSelectionStore } from "../stores/selection";
import { useTabsStore } from "../stores/tabs";

const props = defineProps<{ viewKey: string; nodes?: FolderNode[] }>();
const content = useContentStore();
const selection = useSelectionStore();
const tabs = useTabsStore();
const archiveOpen = ref(true);

// 本标签页视口的直接子级档案（1.2 per-view 参数化）；
// nodes 传入时取而代之（3.6 搜索态：命中档案列表由 ContentArea 按 search store 装配）
const stripArchives = computed(() => props.nodes ?? content.stripArchivesFor(props.viewKey));

// 选中口径同 FolderStrip（文件夹选择域；单击单选 / Ctrl 加选 / Shift 按条带顺序范围选）
function onSelect(id: string, e: MouseEvent): void {
  if (dragView.suppressClick) return;
  selection.clickFolder(
    id,
    { ctrl: e.ctrlKey || e.metaKey, shift: e.shiftKey },
    stripArchives.value.map((f) => f.id),
  );
}

// 双击卡片（非名称区域）= 进入该档案（同 FolderStrip 口径）
function onEnter(id: string): void {
  tabs.openView(`folder:${id}`);
}

// 右键（3.2 一轮，04 稿档案菜单 320 宽 + 搜索框）：未选中项先单选（与树 item 同规则），全局菜单宿主
function onMenu(f: { id: string; name: string }, e: MouseEvent): void {
  if (!selection.isFolderSelected(f.id)) selection.clickFolder(f.id);
  openArchiveCardMenu(e, f);
}
</script>

<template>
  <section class="archive-block">
    <SectionHeader title="档案" :count="stripArchives.length" v-model:open="archiveOpen" />
    <div v-if="archiveOpen" class="flow">
      <ArchiveCard
        v-for="f in stripArchives"
        :key="f.id"
        :folder="f"
        class="cell"
        :selected="selection.isFolderSelected(f.id)"
        :dimmed="dragView.folderIds?.has(f.id) ?? false"
        :cut="selection.isFolderCut(f.id)"
        @select="onSelect(f.id, $event)"
        @dblclick="onEnter(f.id)"
        @contextmenu.prevent.stop="onMenu(f, $event)"
      />
    </div>
  </section>
</template>

<style scoped>
/* 档案卡片区块：容器口径同 FolderStrip（右 16 对齐网格 PAD.right；标题↔卡片 gap 12） */
.archive-block {
  flex: none;
  display: flex;
  flex-direction: column;
  gap: var(--space-12);
  padding-right: var(--space-16);
}
/* 卡片列表：grid auto-fill 弹性列宽（min 260 = figma 42 档案卡片稿宽，同 FolderStrip 弹性口径）；
   卡片间距 24（figma 42 档案卡片列表 gap 24）；卡片固定长宽比由 ArchiveCard aspect-ratio 保持 */
.flow {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  column-gap: var(--space-24);
  row-gap: var(--space-24);
}
.cell {
  min-width: 0;
}
</style>
