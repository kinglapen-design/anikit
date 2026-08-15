<script setup lang="ts">
// 详情面板（3.2 一轮 8 区块体系，2026-08-15 用户定案）：文件与档案共用固定 8 区块——
// 自定义数据（pinned 无副标题）/ 基础信息 / 副本 / 链接区块 / 归属档案 / 关系 / 音频 / 提示词；
// 顺序与显隐 = ui.detailSections（22 稿「编辑数据区块」面板操作，持久化后经注册表兜底合并）。
// 无内容区块 = 空数据显示（不隐藏，标题照常）；一轮全部区块只读（编辑态归二轮）。
// 数据源：选中文件卡片 → watch selectedAssetKey 调 get_asset_detail（await 期间切选中防错配，
// DetailNotes 先例）；选中档案卡片（selection 文件夹选择域首项，kind = archive）→ content.archives
// （无记录用默认空）；无选中 → 空面板（不渲染区块）。
// DetailNotes 在体系外：维持"有笔记才显示"，固定渲染在首位可见区块之后（仅文件选中时）。
import { computed, ref, watch, type Component } from "vue";
import { invoke } from "@tauri-apps/api/core";
import DetailNotes from "./DetailNotes.vue";
import SectionCustom from "./sections/SectionCustom.vue";
import SectionInfo from "./sections/SectionInfo.vue";
import SectionLinks from "./sections/SectionLinks.vue";
import SectionArchive from "./sections/SectionArchive.vue";
import SectionEmpty from "./sections/SectionEmpty.vue";
import {
  detailSectionTitle,
  emptyArchiveData,
  type AssetDetail,
  type DetailTarget,
} from "../lib/detailSections";
import { useContentStore, type FolderNode } from "../stores/content";
import { useSelectionStore } from "../stores/selection";
import { useUiStore } from "../stores/ui";

const content = useContentStore();
const selection = useSelectionStore();
const ui = useUiStore();

// 区块 id → 组件（副本 / 关系 / 音频 / 提示词一轮空态共用 SectionEmpty）
const SECTION_COMPS: Record<string, Component> = {
  custom: SectionCustom,
  info: SectionInfo,
  copies: SectionEmpty,
  links: SectionLinks,
  archive: SectionArchive,
  relations: SectionEmpty,
  audio: SectionEmpty,
  prompt: SectionEmpty,
};

// 可见区块（ui.detailSections 顺序，visible 过滤；标题取注册表）
const sections = computed(() =>
  ui.detailSections.filter((s) => s.visible).map((s) => ({ id: s.id, title: detailSectionTitle(s.id) })),
);

// 资产详情（选中文件卡片驱动；切换即清空，拉到后校验选中未变再落）
const detail = ref<AssetDetail | null>(null);
watch(
  () => content.selectedAssetKey,
  async (key) => {
    detail.value = null;
    if (!key) return;
    const assetId = key.split("|")[0];
    try {
      const d = await invoke<AssetDetail>("get_asset_detail", { assetId });
      if (content.selectedAssetKey === key) detail.value = d; // await 期间已切走防错配
    } catch (e) {
      console.warn("资产详情拉取失败（静默降级）：", e);
    }
  },
  { immediate: true },
);

// 选中档案节点：文件夹选择域首项（ArchiveCard 复用 FolderCard 选择机制）；
// 仅 kind = archive 进档案形态，普通文件夹选中 = 空面板
function findFolder(nodes: FolderNode[], id: string): FolderNode | null {
  for (const n of nodes) {
    if (n.id === id) return n;
    const hit = findFolder(n.children, id);
    if (hit) return hit;
  }
  return null;
}
const archiveNode = computed<FolderNode | null>(() => {
  for (const id of selection.selectedFolders) {
    const n = findFolder(content.folders, id);
    return n?.kind === "archive" ? n : null;
  }
  return null;
});

// 当前展示目标（素材 / 文件夹两选择域互斥，实际只会中一个）
const target = computed<DetailTarget | null>(() => {
  const key = content.selectedAssetKey;
  if (key && detail.value) {
    const p = content.placements.find((pl) => `${pl.assetId}|${pl.folderId}` === key);
    if (p) return { kind: "file", placement: p, detail: detail.value };
  }
  const node = archiveNode.value;
  if (node) {
    return { kind: "archive", node, data: content.archives[node.id] ?? emptyArchiveData(node.id) };
  }
  return null;
});
</script>

<template>
  <div v-if="target" class="detail-panel">
    <template v-for="(s, i) in sections" :key="s.id">
      <component :is="SECTION_COMPS[s.id]" :target="target" :title="s.title" />
      <!-- 笔记区块（体系外，5.2 既有业务）：有笔记才显示，固定在首位可见区块之后 -->
      <DetailNotes v-if="i === 0 && target.kind === 'file'" />
    </template>
    <!-- 全部区块被隐藏时笔记区块仍照常（体系外） -->
    <DetailNotes v-if="sections.length === 0 && target.kind === 'file'" />
  </div>
</template>

<style scoped>
/* 详情容器：区块统一规则（2026-07-23 用户定）——内边距 12 16 / 间距 8 /
   通栏分割线 border-2 以 inset box-shadow 绘制（在背景之上，hover 底色 C2 盖不住；
   首个区块不画）；侧栏外层无 padding，分割线横贯 */
.detail-panel {
  display: flex;
  flex-direction: column;
}
.detail-panel :deep(.detail-block) {
  gap: var(--space-8);
  padding: var(--space-12) var(--space-16);
  box-shadow: inset 0 1px 0 0 var(--border-2);
}
.detail-panel :deep(.detail-block:first-child) {
  box-shadow: none;
}
/* 自定义数据区块（pinned：无副标题、不参与拖拽排序）：
   内边距 8 16 12 16 / 间距 16 */
.detail-panel :deep(.detail-block.pinned) {
  gap: var(--space-16);
  padding: var(--space-8) var(--space-16) 12px;
}
</style>
