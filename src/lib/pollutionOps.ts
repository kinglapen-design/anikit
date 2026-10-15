// 污染文件业务（2026-10-16，1.0 收尾第 5 项，PRD 03_数据链接 02「污染文件」——6.2 工程恢复文件同物）：
// 资产 info 文件夹异物（崩溃恢复文件 / 误拖入 / 残留缓存）在详情「链接关系」tab 区块可见。
// 数据源 = list_pollution_files（已知集字段驱动 + 写死名判定在后端，前端不拼池路径）；
// 三动作 = 打开资源管理器（复用 reveal_orphan_info 池内校验）/ 导入（既有导入管线 move 语义 =
// 导入即清污，序列帧检测 / 冲突挂起 / 进度条全继承）/ 丢回收站（系统回收站，ws 删除同口径无确认弹窗）。
// 刷新时机 = 目标资产变化（TabLinks watch）+ 库内容全量刷新（content.refresh 随导入完成等触发）。
import { reactive } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useContentStore } from "../stores/content";
import { useToastStore } from "../stores/toast";
import { revealOrphanInfo } from "./libraryOps";

/** 污染条目（后端 services/pollution.rs PollutionItem 同形） */
export interface PollutionItem {
  name: string;
  path: string; // 完整路径（reveal 直传）
  isDir: boolean; // 分流导入 kind（目录 = folder 导入）
}

/** 当前详情目标的污染清单（assetId = null 无激活目标——repoll 恒 no-op） */
export const pollutionState = reactive<{ assetId: string | null; items: PollutionItem[] }>({
  assetId: null,
  items: [],
});

let gen = 0; // 代际守卫：切目标后在途回包丢弃（ws loadDir 同口径）

export async function refreshPollution(assetId: string | null): Promise<void> {
  const g = ++gen;
  if (!assetId) {
    pollutionState.assetId = null;
    pollutionState.items = [];
    return;
  }
  try {
    const items = await invoke<PollutionItem[]>("list_pollution_files", { assetId });
    if (g !== gen) return;
    pollutionState.assetId = assetId;
    pollutionState.items = items;
  } catch {
    if (g !== gen) return;
    pollutionState.assetId = assetId;
    pollutionState.items = []; // 静默降级（资产刚被删等）——辅助展示不阻断详情
  }
}

/** 库内容变化后的对齐重扫（导入完成 / 撤销等 content.refresh 链路）；无激活目标 = no-op */
export async function repollPollution(): Promise<void> {
  if (pollutionState.assetId) await refreshPollution(pollutionState.assetId);
}

/** 打开资源管理器定位（复用无主资产同款池内校验命令；失败 toast 在 revealOrphanInfo 内） */
export function revealPollution(path: string): void {
  void revealOrphanInfo(path);
}

/** 丢回收站：成功本地剔除该行（即时反馈），失败 toast 原样文案 */
export async function trashPollution(item: PollutionItem): Promise<void> {
  const assetId = pollutionState.assetId;
  if (!assetId) return;
  const toast = useToastStore();
  try {
    await invoke("trash_pollution_file", { assetId, name: item.name });
    pollutionState.items = pollutionState.items.filter((i) => i.name !== item.name);
    toast.notify({ message: `已丢到回收站：${item.name}`, icon: "success" });
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  }
}

/** 导入为正式资产（剪切语义 move = 导入即清污）：落点 = 当前查看位置所在文件夹；
 * 走 probeSequences 既有链路（序列帧命中弹打包询问、撞 hash 走重复提示、进度条与失败重试不变）；
 * 导入完成 content.refresh() → TabLinks watch 触发 repoll 自动摘行 */
export function importPollution(item: PollutionItem, targetFolder: string | null): void {
  void useContentStore().probeSequences(
    { kind: item.isDir ? "folder" : "files", paths: [item.path], targetFolder },
    "keep",
    "move",
  );
}
