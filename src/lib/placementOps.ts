// 素材基础操作命令封装（6.1 阶段 C，2026-07-28 用户定案）：UI 唯一调用入口。
// 每个写操作统一三段：invoke 命令 → content.refresh() 全量对齐（盘上是事实来源）→
// 操作反馈 toast（C7：成功带「撤销」动作按钮，文案与后端 undo 标签一致；失败中文人话 info 条）。
// 后端命令见 src-tauri/src/commands/operations.rs（全部已接撤销）。
// 2026-07-28 复验反馈修复：toast 计数文案改惰性求值（label 在 action 完成后才拼——
// 原先把模板串在 invoke 前求值，移动 / 关联 / 副本恒显示"0 个"，复验误判"没有实际事件执行"）；
// 新增 move_folders 入口（第 9 条命令，文件夹移动层级，替换原"暂不支持移动文件夹层级"toast）。
import { invoke } from "@tauri-apps/api/core";
import { useContentStore } from "../stores/content";
import { useSelectionStore, type PlacementRef } from "../stores/selection";
import { useToastStore } from "../stores/toast";
import { useUndoStore } from "../stores/undo";

export type { PlacementRef };

// Windows 文件夹 / 文件命名规则（同 CreateLibraryForm 即时校验口径；后端 create_folder 同规则再拦一道）
const ILLEGAL_CHARS = /[\\/:*?"<>|]/;
const RESERVED_NAMES = /^(con|prn|aux|nul|com[1-9]|lpt[1-9])$/i;

export function validateWindowsName(name: string): string {
  if (!name) return "名称不能留空";
  if (ILLEGAL_CHARS.test(name)) return '名称不能包含 \\ / : * ? " < > | 特殊符号';
  if (/[. ]$/.test(name)) return "名称不能以空格或点结尾";
  if (RESERVED_NAMES.test(name)) return "该名称为系统保留名，请换一个";
  return "";
}

// 写操作统一收尾：刷新 + 成功 toast（带撤销）；业务失败 = 中文字符串，info 条原样展示。
// label 惰性求值：action 完成后才拼文案，计数类（移动 N 个）拿到真实返回值
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
    toast.notify({ message: String(e), icon: "info" });
    return false;
  }
}

export async function movePlacements(items: PlacementRef[], targetFolder: string): Promise<boolean> {
  let moved = 0;
  return await runOp(async () => {
    moved = await invoke<number>("move_placements", { items, targetFolder });
  }, () => `移动 ${moved} 个项目`);
}

// 段内排序：items 须同属 targetFolder；beforeItem = null 排到段尾
export async function reorderPlacements(
  items: PlacementRef[],
  targetFolder: string,
  beforeItem: PlacementRef | null,
): Promise<boolean> {
  return await runOp(async () => {
    await invoke("reorder_placements", { items, targetFolder, beforeItem });
  }, () => "调整顺序");
}

// 文件夹同级排序（混排护栏：与素材排序各自独立，2026-07-28 定案）
export async function reorderFolders(
  folderIds: string[],
  parentId: string | null,
  beforeFolderId: string | null,
): Promise<boolean> {
  return await runOp(async () => {
    await invoke("reorder_folders", { folderIds, parentId, beforeFolderId });
  }, () => "调整文件夹顺序");
}

// 移动文件夹层级（2026-07-28 复验反馈 5）：targetParent null 或 "root" = 顶层；
// 循环护栏与同父跳过在后端（撤销标签「移动 N 个文件夹」）；树 item 中间 1/3 与文件夹卡片并入走此
export async function moveFolders(folderIds: string[], targetParent: string | null): Promise<boolean> {
  let moved = 0;
  return await runOp(async () => {
    moved = await invoke<number>("move_folders", { folderIds, targetParent });
  }, () => `移动 ${moved} 个文件夹`);
}

export async function linkPlacements(
  items: PlacementRef[],
  targetFolder: string,
  names: (string | null)[],
): Promise<boolean> {
  let linked = 0;
  return await runOp(async () => {
    linked = await invoke<number>("link_placements", { items, targetFolder, names });
  }, () => `关联 ${linked} 个项目`);
}

export async function duplicatePlacements(
  items: PlacementRef[],
  targetFolder: string,
  names: (string | null)[],
): Promise<boolean> {
  let created = 0;
  return await runOp(async () => {
    const ids = await invoke<string[]>("duplicate_placements", { items, targetFolder, names });
    created = ids.length;
  }, () => `创建 ${created} 个副本`);
}

export async function renamePlacement(ref: PlacementRef, newName: string): Promise<boolean> {
  return await runOp(async () => {
    await invoke("rename_placement", {
      assetId: ref.assetId,
      folderId: ref.folderId,
      origin: ref.origin,
      newName,
    });
  }, () => `重命名为 ${newName.trim()}`);
}

// 删除两段式（C1）：先 lastAction=null 干跑——涉及资源最后一条 Placement 时返回
// needConfirm 不动数据，前端弹询问窗（DeleteConfirmDialog），用户选择后 confirmDelete 再调
export async function deletePlacements(items: PlacementRef[]): Promise<void> {
  const selection = useSelectionStore();
  const toast = useToastStore();
  try {
    const r = await invoke<{ needConfirm: boolean; lastAssets: string[] }>("delete_placements", {
      items,
      lastAction: null,
    });
    if (r.needConfirm) {
      selection.deletePrompt = { items, lastCount: r.lastAssets.length };
      return;
    }
    await runOp(async () => {}, () => `删除 ${items.length} 个项目`);
  } catch (e) {
    toast.notify({ message: String(e), icon: "info" });
  }
}

// 询问窗选择后执行（"recycle" = 移入回收站 / "uncategorized" = 保留为未分类）
export async function confirmDelete(lastAction: "recycle" | "uncategorized"): Promise<void> {
  const selection = useSelectionStore();
  const prompt = selection.deletePrompt;
  selection.deletePrompt = null;
  if (!prompt) return;
  const toast = useToastStore();
  try {
    await invoke("delete_placements", { items: prompt.items, lastAction });
    await runOp(async () => {}, () => `删除 ${prompt.items.length} 个项目`);
  } catch (e) {
    toast.notify({ message: String(e), icon: "info" });
  }
}

// 新建文件夹（C3）：确认名称后才调 create_folder（命名中节点是前端临时态，取消不入库）；
// 返回空串 = 成功，否则为后端中文错误文案（命名行内联提示，保持输入态）
export async function createFolder(
  parentId: string | null,
  name: string,
  afterFolderId: string | null,
): Promise<string> {
  const selection = useSelectionStore();
  const content = useContentStore();
  const toast = useToastStore();
  const undo = useUndoStore();
  try {
    await invoke("create_folder", { parentId, name, afterFolderId });
    selection.newFolderDraft = null;
    await content.refresh();
    void undo.refreshState();
    toast.notify({ message: "已新建文件夹", actionLabel: "撤销", onAction: () => void undo.undo() });
    return "";
  } catch (e) {
    return String(e);
  }
}

// —— 内部剪贴板（C1）——

// Ctrl+C / Ctrl+X：写入内部剪贴板（仅素材；空选择不动作）；
// 复制 / 剪切即时反馈（2026-07-28 复验反馈 7）：info 型、无动作按钮、3 秒
export function copySelection(action: "copy" | "cut"): void {
  const selection = useSelectionStore();
  const items = selection.selectedRefs();
  if (items.length === 0) return;
  selection.clipboard = { items, action };
  useToastStore().notify({
    message: `已${action === "copy" ? "复制" : "剪切"} ${items.length} 个项目`,
    icon: "info",
    duration: 3000,
  });
}

// 粘贴落点判定（2026-07-28 定案）：快速访问视图不作粘贴目标
function pasteTarget(): string | null {
  const content = useContentStore();
  if (!content.sidebarSelected.startsWith("folder:")) {
    useToastStore().notify({ message: "快速访问视图不支持粘贴", icon: "info" });
    return null;
  }
  return content.currentFolderId;
}

// Ctrl+V 仲裁（内部剪贴板非空分支；空 → App.vue 走系统剪贴板导入）：
// copy = 弹关联弹窗（C5）；cut = 直接移动到当前文件夹（粘贴一次后清空剪贴板）
export async function pasteClipboard(): Promise<void> {
  const selection = useSelectionStore();
  const clip = selection.clipboard;
  if (!clip) return;
  const target = pasteTarget();
  if (target === null) return;
  if (clip.action === "copy") {
    selection.linkDraft = { items: clip.items, targetFolder: target };
    return;
  }
  const ok = await movePlacements(clip.items, target);
  if (ok) selection.clipboard = null;
}

// Ctrl+Shift+V：直接副本不弹窗（名称同源 = names 全 null）
export async function pasteDuplicate(): Promise<void> {
  const selection = useSelectionStore();
  const clip = selection.clipboard;
  if (!clip) return;
  const target = pasteTarget();
  if (target === null) return;
  await duplicatePlacements(clip.items, target, clip.items.map(() => null));
}
