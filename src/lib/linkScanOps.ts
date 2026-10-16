// 扫描文件/档案链接（2026-10-16 立项落地，待办 2026-08-13 兑现，PRD 03_查找文件-04；同日两段式改版）：
// 弹窗两步状态 + 菜单入口 + 归并执行。屏 1（714）= 组列表：组级复选默认全选（用户单个取消）+
// 行尾红 X 纯前端摘组，卡片横排只看不动（首列原始文件卡 + 链接卡）；
// 屏 2（970，合并前确认）= 左组列表 + 右保留者 radio 单选——**每组选一个位置保留当原始文件，
// 其余位置（链接 + 被替换的旧 origin）全部清除进回收站**；默认保留者 = 当前 origin（无活 origin 退首条链接）。
// 逐条删链接归链接管理（Ctrl+J / 详情链接区块），本弹窗只做整组归并；
// 归并 = consolidate_asset_links 单命令（物理不动、整批一次撤销步）。
import { reactive } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useContentStore } from "../stores/content";
import { useToastStore } from "../stores/toast";
import type { PlacementRef } from "../stores/selection";
import type { DupAsset } from "./dupScanOps";

// —— 后端 DTO（commands::scan::scan_links，camelCase）+ 前端组级复选态 ——
export interface LinkPlacement {
  folderId: string;
  folderName: string;
  name: string; // 显示名（归并回传 PlacementRef.name 精确到行）
}

export interface LinkGroup {
  asset: DupAsset; // 首列原始文件卡（无活 origin 时后端兜底第一条 Placement）
  origin: LinkPlacement | null; // 活 origin 位置（屏 2 默认保留者；null = origin 在回收站）
  links: LinkPlacement[];
  checked: boolean; // 屏 1 组级复选（默认全选，35 稿族口径）
}

// 屏 2 保留者候选行（origin 行在前）：origin 行 = 当前原始文件，其余 = 全部链接
export interface KeeperRow extends LinkPlacement {
  origin: boolean;
}

export function keeperRows(g: LinkGroup): KeeperRow[] {
  return [
    ...(g.origin ? [{ ...g.origin, origin: true }] : []),
    ...g.links.map((l) => ({ ...l, origin: false })),
  ];
}

export const linkScanState = reactive({
  groups: null as LinkGroup[] | null,
  step: "groups" as "groups" | "confirm",
  currentAsset: null as string | null, // 屏 2 当前查看的组
  keepers: {} as Record<string, number>, // assetId → keeperRows 下标（进屏 2 时初始化 0 = 当前 origin）
  busy: false,
});

// 打开扫描（菜单栏查找 / 新增菜单 / 快速访问空白 / 回收站空白共用入口）：无结果不弹窗，toast 提示
export async function openLinkScan(): Promise<void> {
  const toast = useToastStore();
  if (linkScanState.busy) return;
  linkScanState.busy = true;
  try {
    const groups = await invoke<Array<Omit<LinkGroup, "checked">>>("scan_links");
    if (groups.length === 0) {
      toast.notify({ message: "未发现链接文件", duration: 3000 });
      return;
    }
    linkScanState.groups = groups.map((g) => ({ ...g, checked: true }));
    linkScanState.step = "groups";
    linkScanState.currentAsset = null;
    linkScanState.keepers = {};
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  } finally {
    linkScanState.busy = false;
  }
}

// 行尾红 X：从列表移除该组（纯前端，链接不做任何处理）
export function removeLinkGroup(assetId: string): void {
  if (!linkScanState.groups) return;
  linkScanState.groups = linkScanState.groups.filter((g) => g.asset.assetId !== assetId);
  if (linkScanState.groups.length === 0) closeLinkScan();
}

export function selectedGroups(): LinkGroup[] {
  return (linkScanState.groups ?? []).filter((g) => g.checked);
}

// 进屏 2：初始化各组保留者 = 行 0（origin 行；无活 origin 时 = 首条链接）；旧选择仍有效则保留
export function enterConfirm(): void {
  const selected = selectedGroups();
  if (selected.length === 0) return;
  for (const g of selected) {
    const id = g.asset.assetId;
    const valid = typeof linkScanState.keepers[id] === "number" && linkScanState.keepers[id] < keeperRows(g).length;
    if (!valid) linkScanState.keepers[id] = 0;
  }
  linkScanState.currentAsset = selected[0]?.asset.assetId ?? null;
  linkScanState.step = "confirm";
}

export function backToGroups(): void {
  linkScanState.step = "groups";
}

// 取消 / 关闭 = 全部不处理（链接从未被动过，纯前端清状态）
export function closeLinkScan(): void {
  if (linkScanState.busy) return;
  linkScanState.groups = null;
  linkScanState.step = "groups";
  linkScanState.currentAsset = null;
  linkScanState.keepers = {};
}

// 确认归并：每组保留者成为/保持唯一 origin，其余位置全部进回收站（单命令，整批 undo 一步）
export async function confirmMergeLinks(): Promise<void> {
  const toast = useToastStore();
  if (linkScanState.busy) return;
  const selected = selectedGroups();
  const keepers: PlacementRef[] = [];
  for (const g of selected) {
    const id = g.asset.assetId;
    const row = keeperRows(g)[linkScanState.keepers[id] ?? 0];
    if (!row) continue;
    keepers.push({ assetId: id, folderId: row.folderId, origin: row.origin, name: row.name });
  }
  if (keepers.length === 0) {
    closeLinkScan();
    return;
  }
  linkScanState.busy = true;
  try {
    const cleared = await invoke<number>("consolidate_asset_links", { keepers });
    await useContentStore().refresh();
    toast.notify({
      message: `已归并 ${keepers.length} 组链接，${cleared} 个位置移入回收站`,
      sound: true, // 批量成果类（2026-08-10 口径：成功音仅重要场合）
    });
    linkScanState.groups = null;
    linkScanState.step = "groups";
    linkScanState.currentAsset = null;
    linkScanState.keepers = {};
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  } finally {
    linkScanState.busy = false;
  }
}
