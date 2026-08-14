// 区块操作命令封装（2026-08-14 区块子系统，PRD 02 §01）：UI 唯一调用入口，与 placementOps 同范式——
// invoke → content.refresh() 全量对齐 → toast（成功带「撤销」，后端 undo 标签「区块管理」/「排序区块」）。
// 区块管理弹窗（BlockManageDialog）草稿式一次提交 apply_block_changes；
// 侧栏区块标题拖拽排序即时提交 reorder_blocks。
import { invoke } from "@tauri-apps/api/core";
import { useContentStore } from "../stores/content";
import { useToastStore } from "../stores/toast";
import { useUndoStore } from "../stores/undo";

// 区块草稿行（后端 services/operations/block.rs BlockInput）：id = null 新建（后端发 id）
export interface BlockInput {
  id: string | null;
  name: string;
  icon: string | null;
}

// 区块管理弹窗草稿行（BlockInput + 列表渲染 key；新建行 id = null、key 前端临时发）
export interface BlockDraft extends BlockInput {
  key: string;
}

// 删除非空区块的确认条目（后端 BlockDeleteConflict，camelCase）
export interface BlockDeleteConflict {
  name: string;
  folderCount: number;
}

// apply_block_changes 返回（后端 ApplyBlockReport）：needConfirm = true 时未动任何数据，
// blocks 为涉及的非空待删区块（前端弹 26 稿删除区块警告后带 confirmed = true 重提）
export interface ApplyBlockReport {
  needConfirm: boolean;
  blocks: BlockDeleteConflict[];
}

// 写操作统一收尾（同 placementOps runOp 口径；本文件独立一份防循环依赖）
async function runOp(action: () => Promise<void>, label: () => string): Promise<boolean> {
  const content = useContentStore();
  const toast = useToastStore();
  const undo = useUndoStore();
  try {
    await action();
    await content.refresh();
    void undo.refreshState();
    toast.notify({
      message: `已${label()}`,
      actionLabel: "撤销",
      onAction: () => void undo.undo(),
    });
    return true;
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
    return false;
  }
}

// 区块管理弹窗确认提交：needConfirm 时不刷新不提示（调用方弹警告窗后 confirmed = true 重提）；
// 返回 null = 失败（失败 toast 已弹，弹窗保持打开）
export async function applyBlockChanges(
  blocks: BlockInput[],
  deleted: string[],
  confirmed: boolean,
): Promise<ApplyBlockReport | null> {
  const toast = useToastStore();
  let report: ApplyBlockReport;
  try {
    report = await invoke<ApplyBlockReport>("apply_block_changes", { blocks, deleted, confirmed });
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
    return null;
  }
  if (report.needConfirm) return report;
  await runOp(async () => {}, () => "更新区块");
  return report;
}

// 侧栏区块标题拖拽排序即时提交：ids 必须恰好覆盖全部区块（前端按当前 blocks 组装全量序）
export async function reorderBlocks(ids: string[]): Promise<boolean> {
  return await runOp(async () => {
    await invoke("reorder_blocks", { ids });
  }, () => "调整区块顺序");
}
