// 扫描污染文件（2026-10-16 立项，PRD 03_查找文件-05）：污染文件区块的全库总览与处置入口。
// 详情区块是单资产视图（pollutionOps），不常点开的资产异物会沉淀无感知——本模块给全库扫描弹窗：
// 数据源 = scan_pollution（两段短锁 + spawn_blocking；判定唯一出处 = services/pollution.rs list_pollution）；
// 行动作 = 定位（jumpToAssetPlacement 复用 + 详情切链接关系 tab）/ 打开资源管理器（reveal_orphan_info
// 池内校验复用）/ 全部丢到回收站（逐条 trash_pollution_file，三层校验不变，回收站兜底不进撤销）。
import { reactive } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useToastStore } from "../stores/toast";
import { useUiStore } from "../stores/ui";
import type { DupAsset } from "./dupScanOps";
import { pollutionState, repollPollution, revealPollution, type PollutionItem } from "./pollutionOps";
import { jumpToAssetPlacement } from "./relationOps";

// —— 后端 DTO（commands::pollution::scan_pollution，camelCase）——
export interface PollutionAsset {
  asset: DupAsset; // 行卡数据（复用扫描重复文件 DupAsset 全字段）
  infoPath: string; // info 目录完整路径（reveal 直传）
  items: PollutionItem[];
}

export const pollutionScanState = reactive({
  groups: null as PollutionAsset[] | null,
  busy: false,
});

// 打开扫描（菜单栏「查找 ▸ 扫描污染文件」单入口）：无结果不弹窗，toast 提示（重复扫描同口径）
export async function openPollutionScan(): Promise<void> {
  const toast = useToastStore();
  if (pollutionScanState.busy) return;
  pollutionScanState.busy = true;
  try {
    const groups = await invoke<PollutionAsset[]>("scan_pollution");
    if (groups.length === 0) {
      toast.notify({ message: "未发现污染文件", duration: 3000 });
      return;
    }
    pollutionScanState.groups = groups;
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  } finally {
    pollutionScanState.busy = false;
  }
}

export function closePollutionScan(): void {
  if (pollutionScanState.busy) return;
  pollutionScanState.groups = null;
}

// 定位：跳转资产 origin 所在文件夹视口 + 选中卡片（链接区块跳转共用通道）+ 详情切「链接关系」
// tab（污染区块所在）+ 关弹窗——处置动线在详情区块继续（三动作见 PRD 03_数据链接 02）
export function locatePollution(row: PollutionAsset): void {
  jumpToAssetPlacement(row.asset.assetId, row.asset.folderId, { origin: true });
  useUiStore().detailTab = "links";
  closePollutionScan();
}

// 打开资源管理器：info 目录（复用无主资产同款池内校验命令；失败 toast 在 revealOrphanInfo 内）
export function revealPollutionInfo(row: PollutionAsset): void {
  revealPollution(row.infoPath);
}

// 全部丢到回收站：该资产异物逐条走 trash_pollution_file（三层校验不变，部分失败留行报数）；
// 全清摘行；详情污染区块若正开在该资产，对齐重扫摘行（pollutionOps 代际守卫同口径）
export async function trashPollutionAsset(row: PollutionAsset): Promise<void> {
  const toast = useToastStore();
  if (pollutionScanState.busy) return;
  pollutionScanState.busy = true;
  const total = row.items.length;
  const failed: string[] = [];
  try {
    for (const item of row.items) {
      try {
        await invoke("trash_pollution_file", { assetId: row.asset.assetId, name: item.name });
      } catch {
        failed.push(item.name);
      }
    }
    if (failed.length === 0) {
      pollutionScanState.groups = (pollutionScanState.groups ?? []).filter((g) => g.asset.assetId !== row.asset.assetId);
      toast.notify({ message: `${row.asset.name} 的 ${total} 个污染文件已丢到回收站`, icon: "success" });
      // 全摘空直接收弹窗（busy 在位，不走 closePollutionScan 的闸）
      if (pollutionScanState.groups.length === 0) pollutionScanState.groups = null;
    } else {
      row.items = row.items.filter((i) => failed.includes(i.name));
      toast.notify({ message: `${failed.length} 个条目删除失败（${failed.join("、")}），其余已丢到回收站`, icon: "fail" });
    }
    if (pollutionState.assetId === row.asset.assetId) await repollPollution();
  } finally {
    pollutionScanState.busy = false;
  }
}
