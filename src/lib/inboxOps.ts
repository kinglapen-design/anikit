// 收件箱"工程文件"磁盘区命令封装（4.4，规范第 1 条）：打开 / 删除 / 重命名 / 手动导入
// 四命令统一出口，UI 不再裸 invoke/Channel。
// 删除 / 重命名成功后由本层全量刷新磁盘实况（content.refreshInboxDisk——盘上是事实来源）；
// 手动导入 = 移走入库（Move + Keep），复用现有导入事件流（Channel → content.onImportEvent，
// 进度条 / 重复提示与常规导入一致），finished 后刷新实况剔除已移走条目。
// 失败原样抛出：错误条 / 选中态同步等 UI 反馈留在组件。
import { Channel, invoke } from "@tauri-apps/api/core";
import { useContentStore, type ImportEvent } from "../stores/content";
import { useSelectionStore } from "../stores/selection";

// 系统默认方式打开条目（占用 / 失败后端中文人话原样抛出）
export async function openInboxEntry(name: string): Promise<void> {
  await invoke("open_inbox_entry", { name });
}

// 删除条目：成功后全量刷新磁盘实况
export async function deleteInboxEntry(name: string): Promise<void> {
  await invoke("delete_inbox_entry", { name });
  await useContentStore().refreshInboxDisk();
}

// 重命名条目：成功后全量刷新磁盘实况
export async function renameInboxEntry(name: string, newName: string): Promise<void> {
  await invoke("rename_inbox_entry", { name, newName });
  await useContentStore().refreshInboxDisk();
}

// 手动导入前置确认（PRD 05-01，2.6）：单个 / 批量均只确认一次，明示源文件将移走
export function confirmImportInboxEntries(names: string[], onError: (e: unknown) => void): void {
  const selected = [...new Set(names)];
  if (selected.length === 0) return;
  useSelectionStore().confirmPrompt = {
    title: "导入到资源库",
    body: selected.length === 1
      ? `「${selected[0]}」将从收件箱移走入库，原位置不再保留。`
      : `已选择的 ${selected.length} 个条目将从收件箱移走入库，原位置不再保留。`,
    confirmLabel: selected.length === 1 ? "导入" : `导入 ${selected.length} 项`,
    kind: "warning", // 「原位置不再保留」= 警示语义，警告黄钮（2026-09-08 用户定按钮颜色区分口径）
    onConfirm: () => void importInboxEntries(selected).catch(onError),
  };
}

// 手动导入 = 移走入库：整批共用进度、冲突与失败流；失败复位进度条后抛出（防卡"扫描中"）
export async function importInboxEntries(names: string[]): Promise<void> {
  if (names.length === 0) return;
  const content = useContentStore();
  content.lastImportSource = "move"; // 收件箱恒 Move（过境：导入后即移走）
  content.importProgress = { taskId: null, total: 0, done: 0, startedAt: Date.now() };
  const onEvent = new Channel<ImportEvent>();
  onEvent.onmessage = (e) => {
    content.onImportEvent(e);
    if (e.type === "finished") void content.refreshInboxDisk(); // 移走后实况剔除该条目
  };
  try {
    const taskId = await invoke<number>("import_inbox_entries", { names, onEvent });
    if (content.importProgress && content.importProgress.taskId === null) {
      content.importProgress.taskId = taskId;
    }
  } catch (e) {
    content.importProgress = null;
    throw e;
  }
}
