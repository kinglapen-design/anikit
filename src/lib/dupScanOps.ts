// 扫描重复文件（35_扫描重复文件稿 + Eagle 式合并前确认，2.7 二轮 2026-08-13，PRD 03_查找文件-03）：
// 弹窗两步状态 + 菜单入口 + 合并执行。
// 屏 1 = 35 稿结果列表：组级复选默认全选，行尾红色 ⊖ = 从列表移除该组（纯前端，文件不动）；
// 屏 2 = 合并前确认：左组列表 + 右保留者 radio 单选，默认 = 导入最早（后端按 importedAt 升序，
// assets[0] 即默认保留者），用户可逐组改选；
// 合并 = 落选资产含全部 Placement 一次 delete_placements 移回收站（recycle 语义，undo 一步可撤销）。
import { reactive } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useContentStore } from "../stores/content";
import { useToastStore } from "../stores/toast";
import type { PlacementRef } from "../stores/selection";

// —— 后端 DTO（commands::scan::scan_duplicates，camelCase）——
export interface DupPlacement {
  folderId: string;
  origin: boolean;
}

export interface DupAsset {
  assetId: string;
  name: string; // origin Placement 显示名
  folderId: string; // origin Placement 所在文件夹
  folderName: string; // root → "未分类"
  w: number | null;
  h: number | null;
  ext: string;
  size: number;
  thumbPath: string | null;
  thumbKind: "asset" | "formatIcon" | null;
  placements: DupPlacement[]; // 该资产全部 Placement（合并组装 PlacementRef 用）
}

// 后端返回组 + 前端组级复选态（默认全选，35 稿口径）
export interface DupGroup {
  hash: string;
  size: number;
  assets: DupAsset[]; // 后端已按 importedAt 升序排（assets[0] = 导入最早 = 默认保留者）
  checked: boolean;
}

// 扫描范围（锚点过滤：组内任一资产命中，整组返回，PRD 03-03-1）
export type ScanScope =
  | { kind: "all" }
  | { kind: "folder"; folderId: string }
  | { kind: "assets"; assetIds: string[] };

export const dupScanState = reactive({
  groups: null as DupGroup[] | null,
  step: "groups" as "groups" | "confirm",
  currentHash: null as string | null, // 屏 2 当前查看的组
  keepers: {} as Record<string, string>, // hash → 保留 assetId（进屏 2 时初始化 assets[0]）
  busy: false,
});

// 打开扫描（菜单三范围 + 汉堡共用入口）：无结果不弹窗，toast 提示
export async function openScan(scope: ScanScope): Promise<void> {
  const toast = useToastStore();
  if (dupScanState.busy) return;
  dupScanState.busy = true;
  try {
    const groups = await invoke<Array<Omit<DupGroup, "checked">>>("scan_duplicates", { scope });
    if (groups.length === 0) {
      toast.notify({ message: "未发现重复文件", duration: 3000 });
      return;
    }
    dupScanState.groups = groups.map((g) => ({ ...g, checked: true }));
    dupScanState.step = "groups";
    dupScanState.currentHash = null;
    dupScanState.keepers = {};
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  } finally {
    dupScanState.busy = false;
  }
}

// 红色 ⊖：从列表移除该组（纯前端，文件不做任何处理）
export function removeGroup(hash: string): void {
  if (!dupScanState.groups) return;
  dupScanState.groups = dupScanState.groups.filter((g) => g.hash !== hash);
  if (dupScanState.groups.length === 0) closeScan();
}

// 选中组（屏 2 左列 = 此快照）
export function selectedGroups(): DupGroup[] {
  return (dupScanState.groups ?? []).filter((g) => g.checked);
}

// 进屏 2：初始化各组保留者 = 导入最早（assets[0]；旧选择仍有效则保留，返回屏 1 改选后再进不丢）
export function enterConfirm(): void {
  const selected = selectedGroups();
  if (selected.length === 0) return;
  for (const g of selected) {
    const valid = g.assets.some((a) => a.assetId === dupScanState.keepers[g.hash]);
    if (!valid) dupScanState.keepers[g.hash] = g.assets[0]?.assetId ?? "";
  }
  dupScanState.currentHash = selected[0]?.hash ?? null;
  dupScanState.step = "confirm";
}

export function backToGroups(): void {
  dupScanState.step = "groups";
}

// 取消 / 关闭 = 全部不处理（文件从未被动过，纯前端清状态）
export function closeScan(): void {
  if (dupScanState.busy) return;
  dupScanState.groups = null;
  dupScanState.step = "groups";
  dupScanState.currentHash = null;
  dupScanState.keepers = {};
}

// 确认合并：每组保留 keepers 选中的 1 个资产，其余含全部 Placement 一次移回收站（undo 一步）
export async function confirmMerge(): Promise<void> {
  const toast = useToastStore();
  if (dupScanState.busy) return;
  const selected = selectedGroups();
  const items: PlacementRef[] = [];
  let loserCount = 0;
  for (const g of selected) {
    const keep = dupScanState.keepers[g.hash] ?? g.assets[0]?.assetId;
    for (const a of g.assets) {
      if (a.assetId === keep) continue;
      loserCount += 1;
      for (const p of a.placements) {
        items.push({ assetId: a.assetId, folderId: p.folderId, origin: p.origin });
      }
    }
  }
  if (items.length === 0) {
    closeScan();
    return;
  }
  dupScanState.busy = true;
  try {
    await invoke("delete_placements", { items, lastAction: "recycle" });
    await useContentStore().refresh();
    toast.notify({
      message: `已合并 ${selected.length} 组重复文件，${loserCount} 个文件移入回收站`,
      sound: true, // 批量成果类（2026-08-10 口径：成功音仅重要场合）
    });
    dupScanState.groups = null;
    dupScanState.step = "groups";
    dupScanState.currentHash = null;
    dupScanState.keepers = {};
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  } finally {
    dupScanState.busy = false;
  }
}
