// md 编辑器撤销栈（2026-09-25 用户走查「没有接入撤销」）：
// textarea 原生撤销只管连续键入，程序化修改（工具栏动作 / 复选框 toggle / v-model 同步）即断栈——
// 自实现快照历史：组件会话级、编辑态内 Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y 驱动；
// 与库撤销体系无关（「编辑不进 undo」2026-08-23 口径不变——本栈是编辑器内文本历史；
// 全局 Ctrl+Z 本就对输入框让位，useGlobalKeys isEditableTarget）。
// 合并口径：键入类变更 800ms 窗口内连续合并为一步（record(merge=true)）；
// 工具栏动作 / 复选框 toggle（merge=false）恒为独立步。

export interface HistEntry {
  text: string;
  start: number; // 变更前选区起
  end: number; // 变更前选区止
}

const MERGE_MS = 800;
const LIMIT = 100;

export interface MdHistory {
  /** 一次 draft 变更前记录前态；merge=true 且距上次记录 <800ms 时并入上一步（连续键入 = 一步） */
  record(snap: HistEntry, merge: boolean): void;
  /** 撤销：返回应恢复到的前态（cur = 当前态入重做栈）；无可撤销 = null */
  undo(cur: HistEntry): HistEntry | null;
  /** 重做：对称 */
  redo(cur: HistEntry): HistEntry | null;
  /** 切素材 / 重新加载时清空 */
  reset(): void;
}

export function createMdHistory(): MdHistory {
  let past: HistEntry[] = [];
  let future: HistEntry[] = [];
  let lastT = 0;
  let lastMerge = false; // 上一步是否键入合并步（工具栏/toggle 后下次键入必开新步）

  function record(snap: HistEntry, merge: boolean): void {
    const now = Date.now();
    if (merge && lastMerge && now - lastT < MERGE_MS) {
      lastT = now; // 合并窗内续键：前态已是最初，只续窗
      return;
    }
    past.push(snap);
    if (past.length > LIMIT) past.shift();
    future = []; // 新变更截断重做分支
    lastT = now;
    lastMerge = merge;
  }

  function undo(cur: HistEntry): HistEntry | null {
    const entry = past.pop();
    if (!entry) return null;
    future.push(cur);
    lastMerge = false;
    return entry;
  }

  function redo(cur: HistEntry): HistEntry | null {
    const entry = future.pop();
    if (!entry) return null;
    past.push(cur);
    lastMerge = false;
    return entry;
  }

  function reset(): void {
    past = [];
    future = [];
    lastT = 0;
    lastMerge = false;
  }

  return { record, undo, redo, reset };
}
