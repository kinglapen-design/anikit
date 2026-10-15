// 扫描文件/档案链接（2026-10-16 立项落地，待办 2026-08-13 兑现，PRD 03_查找文件-04）：
// 弹窗状态 + 菜单入口 + 删除执行。全库单范围（菜单项无二级）；复用 35 稿扫描重复文件样式——
// 复选语义相反：链接卡勾选 = 标记删除该链接位置（选中压暗 30%，默认全不选），原始文件卡恒留不可选；
// 删除 = 选中链接一次 delete_placements（recycle 语义：记录进回收站、物理不动、undo 一步）。
import { reactive } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useContentStore } from "../stores/content";
import { useToastStore } from "../stores/toast";
import type { PlacementRef } from "../stores/selection";
import type { DupAsset } from "./dupScanOps";

// —— 后端 DTO（commands::scan::scan_links，camelCase）+ 前端删除标记态 ——
export interface LinkItem {
  folderId: string;
  folderName: string;
  name: string; // 链接显示名（删除时回传 PlacementRef.name 精确到行）
  checked: boolean; // 复选 = 删除标记（默认 false，35 稿口径）
}

export interface LinkGroup {
  asset: DupAsset; // 首列原始文件卡（无活 origin 时后端兜底第一条 Placement）
  links: LinkItem[];
}

export const linkScanState = reactive({
  groups: null as LinkGroup[] | null,
  busy: false,
});

// 打开扫描（菜单栏查找 / 新增菜单 / 快速访问空白 / 回收站空白共用入口）：无结果不弹窗，toast 提示
export async function openLinkScan(): Promise<void> {
  const toast = useToastStore();
  if (linkScanState.busy) return;
  linkScanState.busy = true;
  try {
    const groups = await invoke<Array<{ asset: DupAsset; links: Array<Omit<LinkItem, "checked">> }>>("scan_links");
    if (groups.length === 0) {
      toast.notify({ message: "未发现链接文件", duration: 3000 });
      return;
    }
    linkScanState.groups = groups.map((g) => ({
      asset: g.asset,
      links: g.links.map((l) => ({ ...l, checked: false })),
    }));
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  } finally {
    linkScanState.busy = false;
  }
}

// 行尾红 X：从列表移除该组（纯前端，链接不做任何处理——DupScanGroupRow 红钮同口径）
export function removeLinkGroup(assetId: string): void {
  if (!linkScanState.groups) return;
  linkScanState.groups = linkScanState.groups.filter((g) => g.asset.assetId !== assetId);
  if (linkScanState.groups.length === 0) closeLinkScan();
}

export function selectedLinkCount(): number {
  return (linkScanState.groups ?? []).reduce((sum, g) => sum + g.links.filter((l) => l.checked).length, 0);
}

// 取消 / 关闭 = 全部不处理（链接从未被动过，纯前端清状态）
export function closeLinkScan(): void {
  if (linkScanState.busy) return;
  linkScanState.groups = null;
}

// 确认删除：选中链接一次 delete_placements（recycle 语义，undo 一步可撤销）；
// 同文件夹同资产重复链接携带 name 精确到行（2026-10-11 选择域配套口径）
export async function confirmDeleteLinks(): Promise<void> {
  const toast = useToastStore();
  if (linkScanState.busy) return;
  const items: PlacementRef[] = [];
  for (const g of linkScanState.groups ?? []) {
    for (const l of g.links) {
      if (l.checked) items.push({ assetId: g.asset.assetId, folderId: l.folderId, origin: false, name: l.name });
    }
  }
  if (items.length === 0) return;
  linkScanState.busy = true;
  try {
    await invoke("delete_placements", { items, lastAction: "recycle" });
    await useContentStore().refresh();
    toast.notify({
      message: `已将 ${items.length} 个链接文件移入回收站`,
      sound: true, // 批量成果类（2026-08-10 口径：成功音仅重要场合）
    });
    linkScanState.groups = null;
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  } finally {
    linkScanState.busy = false;
  }
}
