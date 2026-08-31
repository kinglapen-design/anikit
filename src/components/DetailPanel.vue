<script setup lang="ts">
// 详情面板（7.1 二十三轮三 tab 化 + 文件夹详情形态）：数据装配壳——
// 选中文件卡片 → watch selectedAssetKey 调 get_asset_detail（await 期间切选中防错配）；
// 选中档案卡片 / 普通文件夹卡片（selection 文件夹选择域首项按 kind 分流）；
// **视口未选中文件时回退显示当前停留视口的文件夹 / 档案 / 快速访问信息**（2026-08-31 用户定，
// 稿 = 右侧面板-文件夹详情；快速访问 = 简化版：无缩略图 + 名称框禁改名 + 文件数/占用空间两行）。
// 渲染结构（pinned + 分段控制器三 tab / FolderDetail 形态分流）在共享组合 DetailSections。
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
async function loadDetail(clear: boolean): Promise<void> {
  const key = content.selectedAssetKey;
  if (clear) detail.value = null;
  if (!key) return;
  const assetId = key.split("|")[0];
  try {
    const d = await invoke<AssetDetail>("get_asset_detail", { assetId });
    if (content.selectedAssetKey === key) detail.value = d; // await 期间已切走防错配
  } catch (e) {
    console.warn("资产详情拉取失败（静默降级）：", e);
  }
}
watch(() => content.selectedAssetKey, () => void loadDetail(true), { immediate: true });
// 6.1：属性 / 描述等详情编辑落盘后 bumpDetailRev 重拉（不清空防闪烁）
watch(() => content.detailRev, () => void loadDetail(false));

// 无任何选中（素材 / 文件夹选择域皆空；seq 包内视图 selectedAssetKey 回退包资产视为有选中）
const nothingSelected = computed(
  () => !content.selectedAssetKey && selection.selectedFolders.size === 0,
);

// 视口文件夹节点（当前停留视口 = sidebarSelected；folder:{id} 才有点）
const viewportFolderNode = computed<FolderNode | null>(() => {
  const vk = content.sidebarSelected;
  return vk.startsWith("folder:") ? findFolderNode(content.folders, vk.slice(7)) : null;
});

// 档案上下文节点：选中档案卡片优先；无选中时回退视口档案节点（2026-08-31 文件夹详情定案同覆盖档案）
const archiveNode = computed<FolderNode | null>(() => {
  for (const id of selection.selectedFolders) {
    const n = findFolderNode(content.folders, id);
    return n?.kind === "archive" ? n : null;
  }
  return nothingSelected.value && viewportFolderNode.value?.kind === "archive"
    ? viewportFolderNode.value
    : null;
});

// 档案详情（ArchiveDetailView：数据 + 文件数/占用空间统计；防 await 错配同资产）
const archiveDetail = ref<ArchiveDetailView | null>(null);
async function loadArchiveDetail(clear: boolean): Promise<void> {
  const id = archiveNode.value?.id;
  if (clear) archiveDetail.value = null;
  if (!id) return;
  try {
    const d = await invoke<ArchiveDetailView>("get_archive_data", { folderId: id });
    if (archiveNode.value?.id === id) archiveDetail.value = d;
  } catch (e) {
    console.warn("档案详情拉取失败（静默降级）：", e);
  }
}
watch(() => archiveNode.value?.id, () => void loadArchiveDetail(true), { immediate: true });
// 6.1：档案属性 / 描述 / 封面设置落盘后 bumpDetailRev 重拉（不清空防闪烁）
watch(() => content.detailRev, () => void loadArchiveDetail(false));

// 选中普通文件夹节点（选择域首项非 archive 节点）
const folderNode = computed<FolderNode | null>(() => {
  for (const id of selection.selectedFolders) {
    const n = findFolderNode(content.folders, id);
    return n?.kind === "archive" ? null : (n ?? null);
  }
  return null;
});

// 当前展示目标：选中文件 > 选中/视口档案 > 选中普通文件夹 > 视口回退（文件夹 / 快速访问）；
// 回收站选中（2026-08-16 用户拍板）：Placement 不在 content.placements（folderId 恒 "recycle"），
// 回退 recycleEntries 取缩略图/显示数据（get_asset_detail 按 assetId 不依赖 Placement）
const target = computed<DetailTarget | null>(() => {
  const key = content.selectedAssetKey;
  if (key && detail.value) {
    const p =
      content.placements.find((pl) => `${pl.assetId}|${pl.folderId}` === key) ??
      content.recycleEntries.find((e) => `${e.assetId}|${e.folderId}` === key);
    if (p) return { kind: "file", placement: p, detail: detail.value };
  }
  const an = archiveNode.value;
  if (an) {
    return { kind: "archive", node: an, data: archiveDetail.value ?? emptyArchiveData(an.id) };
  }
  const fn = folderNode.value;
  if (fn) return { kind: "folder", node: fn };
  // 视口回退（未选中文件时显示当前视口信息）：文件夹视口 → folder 形态；快速访问 → quick 形态
  if (nothingSelected.value) {
    const vn = viewportFolderNode.value;
    if (vn) return { kind: "folder", node: vn };
    const vk = content.sidebarSelected;
    if (vk.startsWith("quick:")) return { kind: "quick", viewKey: vk, name: content.viewTitle(vk) };
  }
  return null;
});
</script>

<template>
  <DetailSections v-if="target" :target="target" />
</template>
