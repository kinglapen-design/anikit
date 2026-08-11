// 收件箱"工程文件"磁盘区命令封装（4.4，规范第 1 条）：打开 / 删除 / 重命名 / 手动导入
// 四命令统一出口，UI 不再裸 invoke/Channel。
// 删除 / 重命名成功后由本层全量刷新磁盘实况（content.refreshInboxDisk——盘上是事实来源）；
// 手动导入 = 移走入库（Move + Keep），复用现有导入事件流（Channel → content.onImportEvent，
// 进度条 / 重复提示与常规导入一致），finished 后刷新实况剔除已移走条目。
// 失败原样抛出：错误条 / 选中态同步等 UI 反馈留在组件。
import { Channel, invoke } from "@tauri-apps/api/core";
import { useContentStore, type ImportEvent } from "../stores/content";

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

// 手动导入 = 移走入库：进度初值 / Channel 事件转发 / taskId 回填与 content.startImport 同口径；
// 失败复位进度条后抛出（防卡"扫描中"）
export async function importInboxEntry(name: string): Promise<void> {
  const content = useContentStore();
  content.lastImportSource = "move"; // 收件箱恒 Move（过境：导入后即移走）
  content.importProgress = { taskId: null, total: 0, done: 0, startedAt: Date.now() };
  const onEvent = new Channel<ImportEvent>();
  onEvent.onmessage = (e) => {
    content.onImportEvent(e);
    if (e.type === "finished") void content.refreshInboxDisk(); // 移走后实况剔除该条目
  };
  try {
    const taskId = await invoke<number>("import_inbox_entry", { name, onEvent });
    if (content.importProgress && content.importProgress.taskId === null) {
      content.importProgress.taskId = taskId;
    }
  } catch (e) {
    content.importProgress = null;
    throw e;
  }
}
