<script setup lang="ts">
// 详情面板（3.2 一轮区块体系）：数据装配壳——选中文件卡片 → watch selectedAssetKey 调
// get_asset_detail（await 期间切选中防错配，DetailNotes 先例）；选中档案卡片（selection
// 文件夹选择域首项，kind = archive）→ get_archive_data（ArchiveDetailView：数据 + 统计，
// 失败/无记录用默认空）；无选中 → 空面板（不渲染区块）。
// 区块栈渲染（pinned + 笔记 + 注册表 7 区块 + 面板内拖拽排序）在共享组合 DetailSections
// （ArchiveSidePanel 档案侧栏复用同一套，2026-08-15 任务 A 三项）。
import { computed, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import DetailSections from "./DetailSections.vue";
import {
  emptyArchiveData,
  findFolderNode,
  type AssetDetail,
  type DetailTarget,
} from "../lib/detailSections";
import { useContentStore, type ArchiveDetailView, type FolderNode } from "../stores/content";
import { useSelectionStore } from "../stores/selection";

const content = useContentStore();
const selection = useSelectionStore();

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
const archiveNode = computed<FolderNode | null>(() => {
  for (const id of selection.selectedFolders) {
    const n = findFolderNode(content.folders, id);
    return n?.kind === "archive" ? n : null;
  }
  return null;
});

// 档案详情（ArchiveDetailView：数据 + 文件数/占用空间统计；切换即清空，防 await 错配同资产）
const archiveDetail = ref<ArchiveDetailView | null>(null);
watch(
  () => archiveNode.value?.id,
  async (id) => {
    archiveDetail.value = null;
    if (!id) return;
    try {
      const d = await invoke<ArchiveDetailView>("get_archive_data", { folderId: id });
      if (archiveNode.value?.id === id) archiveDetail.value = d;
    } catch (e) {
      console.warn("档案详情拉取失败（静默降级）：", e);
    }
  },
  { immediate: true },
);

// 当前展示目标（素材 / 文件夹两选择域互斥，实际只会中一个）
const target = computed<DetailTarget | null>(() => {
  const key = content.selectedAssetKey;
  if (key && detail.value) {
    const p = content.placements.find((pl) => `${pl.assetId}|${pl.folderId}` === key);
    if (p) return { kind: "file", placement: p, detail: detail.value };
  }
  const node = archiveNode.value;
  if (node) {
    return { kind: "archive", node, data: archiveDetail.value ?? emptyArchiveData(node.id) };
  }
  return null;
});
</script>

<template>
  <DetailSections v-if="target" :target="target" notes />
</template>
