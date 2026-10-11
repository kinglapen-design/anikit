<script setup lang="ts">
// 详情面板（7.1 二十三轮三 tab 化 + 文件夹详情形态）：数据装配壳——
// 选中文件卡片 → watch selectedAssetKey 调 get_asset_detail（await 期间切选中防错配）；
// 选中档案卡片 / 普通文件夹卡片（selection 文件夹选择域首项按 kind 分流）；
// 选中工作空间磁盘实况文件 / 文件夹（2026-09-26；文件夹随当日选中链接通——文件夹卡片 /
// 树行点击入素材选择域）：assetId = ws: 假 id 禁发 assetId 类命令——
//   数据 = ws store 目录缓存条目（WsEntry 磁盘实况）直达，简化基础信息形态（WsDetail）；
// **视口未选中文件时回退显示当前停留视口的文件夹 / 档案 / 快速访问信息**（2026-08-31 用户定，
// 稿 = 右侧面板-文件夹详情；快速访问 = 简化版：无缩略图 + 名称框禁改名 + 文件数/占用空间两行）；
// **工作空间视口同口径回退**（2026-10-03 用户定）：未选中任何项时显示当前视口目录的信息，
// 形态 = 选中该文件夹同款的 WsDetail 简化基础信息（名称 / 位置 / 类型行）。
// 渲染结构（pinned + 分段控制器三 tab / FolderDetail / WsDetail 形态分流）在共享组合 DetailSections。
import { computed, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import DetailSections from "./DetailSections.vue";
import {
  emptyArchiveData,
  findFolderNode,
  type AssetDetail,
  type DetailTarget,
} from "../lib/detailSections";
import { placementKey, useContentStore, type ArchiveDetailView, type FolderNode } from "../stores/content";
import { useSelectionStore } from "../stores/selection";
import { useWsStore, WS_ID_PREFIX } from "../stores/ws";

const content = useContentStore();
const selection = useSelectionStore();
const ws = useWsStore();

// 资产详情（选中文件卡片驱动；切换即清空，拉到后校验选中未变再落）
const detail = ref<AssetDetail | null>(null);
async function loadDetail(clear: boolean): Promise<void> {
  const key = content.selectedAssetKey;
  if (clear) detail.value = null;
  if (!key) return;
  const assetId = key.split("|")[0];
  // 工作空间磁盘实况合成项（PRD 02 步骤 1）：assetId = ws: 假 id 严禁发 assetId 类命令——
  // 详情由下方 wsTarget 从 ws store 目录缓存直达（2026-09-26 基础数据形态）
  if (assetId.startsWith("ws:")) return;
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

// 工作空间磁盘实况选中条目（2026-09-26 基础数据形态；文件与文件夹同径——文件夹随当日
// 文件夹卡片 / 树行选中链接通）：数据 = ws store 目录缓存条目直达
// （磁盘实况语义，无任何附加数据）；dirPath = 所在目录绝对路径（root 未就绪窗口期 = null，
// 位置框显占位）。多选与库侧同口径 = 显示选择域首项（selectedAssetKey = selection.first 转发）；
// 条目不在缓存（刚被删 / 目录未加载）= null 留空
const wsTarget = computed<Extract<DetailTarget, { kind: "ws" }> | null>(() => {
  const key = content.selectedAssetKey;
  if (!key.startsWith(WS_ID_PREFIX)) return null;
  const rel = key.split("|")[0].slice(WS_ID_PREFIX.length);
  const parentRel = rel.includes("/") ? rel.slice(0, rel.lastIndexOf("/")) : "";
  const entry = ws.entries(parentRel).find((e) => e.relPath === rel);
  return entry ? { kind: "ws", entry, dirPath: ws.absPath(parentRel) } : null;
});

// 工作空间视口回退（2026-10-03 用户定，库 folder/quick 视口回退同口径）：未选中任何项时
// 显示当前视口目录的信息——形态 = 选中该文件夹同款的 WsDetail 简化基础信息。
// 子目录条目 = 父目录缓存条目（wsTarget 同径；深层视口父目录未必加载——下方 watch 幂等
// ensureDir 触发，加载完成响应式到位；窗口期 = null 留白）。根视口（rel ""）非任何目录的
// 条目：合成根目录条目——名称 = content.viewTitle（「工作空间」，根入口行 / 根标签标题同口径），
// 位置 = 根绝对路径；root 未就绪（未开空间 / 启动暂态）= null 留白
const wsViewportTarget = computed<Extract<DetailTarget, { kind: "ws" }> | null>(() => {
  const vk = content.sidebarSelected;
  if (!vk.startsWith(WS_ID_PREFIX)) return null;
  const rel = vk.slice(WS_ID_PREFIX.length);
  if (rel === "") {
    const rootPath = ws.root?.rootPath;
    if (!rootPath) return null;
    return {
      kind: "ws",
      entry: {
        name: content.viewTitle(vk),
        relPath: "",
        isDir: true,
        size: 0,
        mtimeMs: 0,
        ctimeMs: 0,
        w: null,
        h: null,
        ext: "",
        thumbPath: null,
        thumbKind: null,
      },
      dirPath: rootPath,
    };
  }
  const parentRel = rel.includes("/") ? rel.slice(0, rel.lastIndexOf("/")) : "";
  const entry = ws.entries(parentRel).find((e) => e.relPath === rel);
  return entry ? { kind: "ws", entry, dirPath: ws.absPath(parentRel) } : null;
});

// 视口回退的目录条目依赖父目录缓存（深层视口父目录未必加载过）——回退激活（无选中 + ws 视口）
// 且父目录未加载时幂等触发（ensureDir 已加载即跳过；加载完成 entries 变更驱动上方 computed 到位）
watch(
  () => [nothingSelected.value, content.sidebarSelected] as const,
  ([nothing, vk]) => {
    if (!nothing || !vk.startsWith(WS_ID_PREFIX)) return;
    const rel = vk.slice(WS_ID_PREFIX.length);
    if (rel === "") return;
    const parentRel = rel.includes("/") ? rel.slice(0, rel.lastIndexOf("/")) : "";
    void ws.ensureDir(parentRel);
  },
  { immediate: true },
);

// 当前展示目标：选中文件 > 选中/视口档案 > 选中普通文件夹 > 视口回退（文件夹 / 快速访问 / 工作空间视口目录）；
// 回收站选中（2026-08-16 用户拍板）：Placement 不在 content.placements（folderId 恒 "recycle"），
// 回退 recycleEntries 取缩略图/显示数据（get_asset_detail 按 assetId 不依赖 Placement）
const target = computed<DetailTarget | null>(() => {
  const key = content.selectedAssetKey;
  // 工作空间磁盘实况分支优先：假 id 永不出库侧 detail，后续 file 分支恒不命中（缓存直达）
  if (key.startsWith(WS_ID_PREFIX)) return wsTarget.value;
  if (key && detail.value) {
    const p =
      content.placements.find((pl) => placementKey(pl) === key) ??
      content.recycleEntries.find((e) => placementKey(e) === key);
    if (p) return { kind: "file", placement: p, detail: detail.value };
  }
  const an = archiveNode.value;
  if (an) {
    return { kind: "archive", node: an, data: archiveDetail.value ?? emptyArchiveData(an.id) };
  }
  const fn = folderNode.value;
  if (fn) return { kind: "folder", node: fn };
  // 视口回退（未选中文件时显示当前视口信息）：文件夹视口 → folder 形态；快速访问 → quick 形态；
  // 工作空间视口 → ws 形态（2026-10-03 用户定，选中该文件夹同款 WsDetail；null = 父目录未加载
  // 窗口期 / root 未就绪，留白同库侧数据未就位口径）
  if (nothingSelected.value) {
    const vn = viewportFolderNode.value;
    if (vn) return { kind: "folder", node: vn };
    const vk = content.sidebarSelected;
    if (vk.startsWith("quick:")) return { kind: "quick", viewKey: vk, name: content.viewTitle(vk) };
    if (vk.startsWith(WS_ID_PREFIX)) return wsViewportTarget.value;
  }
  return null;
});
</script>

<template>
  <DetailSections v-if="target" :target="target" />
</template>
