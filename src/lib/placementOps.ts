// 素材基础操作命令封装（6.1 阶段 C，2026-07-28 用户定案）：UI 唯一调用入口。
// 每个写操作统一三段：invoke 命令 → content.refresh() 全量对齐（盘上是事实来源）→
// 操作反馈 toast（C7：成功带「撤销」动作按钮，文案与后端 undo 标签一致；失败中文人话 info 条）。
// 后端命令见 src-tauri/src/commands/operations.rs（全部已接撤销）。
// 2026-07-28 复验反馈修复：toast 计数文案改惰性求值（label 在 action 完成后才拼——
// 原先把模板串在 invoke 前求值，移动 / 关联 / 副本恒显示"0 个"，复验误判"没有实际事件执行"）；
// 新增 move_folders 入口（第 9 条命令，文件夹移动层级，替换原"暂不支持移动文件夹层级"toast）。
// 2026-07-29 二轮：回收站操作入口（restore_recycle 带撤销；delete_recycle / clear_recycle
// 两段式确认、不可恢复无撤销；后端契约并行开发，UI 直接 invoke）。
import { invoke } from "@tauri-apps/api/core";
import { useContentStore } from "../stores/content";
import { useSelectionStore, type PlacementRef } from "../stores/selection";
import { useToastStore } from "../stores/toast";
import { useUndoStore } from "../stores/undo";

export type { PlacementRef };

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
// beforeFolderId 给了 = 插到该兄弟（targetParent 直接子级）之前——跨父级上/下 1/3 落点定位
// （2026-07-29 复验：嵌套文件夹拖出），null = 同级末尾；
// 循环护栏与同父跳过在后端（撤销标签「移动 N 个文件夹」）；树 item 中间 1/3 与文件夹卡片并入走此
export async function moveFolders(
  folderIds: string[],
  targetParent: string | null,
  beforeFolderId: string | null = null,
): Promise<boolean> {
  let moved = 0;
  return await runOp(async () => {
    moved = await invoke<number>("move_folders", { folderIds, targetParent, beforeFolderId });
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

// 重命名文件夹（2026-07-29 复验三轮，第 10 条命令）：FolderCard 名称双击 / 新建后命名保存共用；
// 非法名后端拒绝走失败 toast，调用方不保留输入态
export async function renameFolder(folderId: string, newName: string): Promise<boolean> {
  return await runOp(async () => {
    await invoke("rename_folder", { folderId, newName });
  }, () => `重命名文件夹为 ${newName.trim()}`);
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

// —— 回收站（6.1 二轮，2026-07-29；后端契约并行开发：restore_recycle / delete_recycle / clear_recycle）——

// 彻底删除返回（delete_recycle / clear_recycle 同构）：needConfirm = 有虚拟片段引用需确认
interface RecycleDeleteResult {
  needConfirm: boolean;
  segmentRefs: number;
  deleted: number;
}

// 硬删除统一收尾：刷新 + 成功 toast（不可恢复，无撤销按钮——与 runOp 的区别仅此）
async function runHardDelete(label: () => string): Promise<void> {
  const content = useContentStore();
  await content.refresh();
  useToastStore().notify({ message: `已${label()}` });
}

// 回收站视口选中项的 assetId 清单（右键菜单 / Del 键入参；按回收站条目顺序去重输出）
export function recycleSelectedAssetIds(): string[] {
  const selection = useSelectionStore();
  const content = useContentStore();
  const out: string[] = [];
  for (const e of content.recycleEntries) {
    if (selection.selected.has(`${e.assetId}|${e.folderId}`) && !out.includes(e.assetId)) {
      out.push(e.assetId);
    }
  }
  return out;
}

// 还原（restore_recycle）：成功 toast 带撤销（后端撤销标签「还原 N 个项目」）
export async function restoreRecycle(assetIds: string[]): Promise<boolean> {
  if (assetIds.length === 0) return false;
  let restored = 0;
  return await runOp(async () => {
    restored = await invoke<number>("restore_recycle", { assetIds });
  }, () => `还原 ${restored} 个项目`);
}

// 彻底删除两段式：confirmed=false 干跑——needConfirm 时弹确认窗（虚拟片段引用警示，
// RecycleConfirmDialog），确认后 confirmRecycleDelete 带 confirmed=true 再调
export async function deleteRecycle(assetIds: string[]): Promise<void> {
  if (assetIds.length === 0) return;
  const selection = useSelectionStore();
  const toast = useToastStore();
  try {
    const r = await invoke<RecycleDeleteResult>("delete_recycle", { assetIds, confirmed: false });
    if (r.needConfirm) {
      selection.recyclePrompt = { kind: "delete", assetIds, segmentRefs: r.segmentRefs };
      return;
    }
    await runHardDelete(() => `彻底删除 ${r.deleted} 个项目`);
  } catch (e) {
    toast.notify({ message: String(e), icon: "info" });
  }
}

// 清空回收站两段式（同 deleteRecycle 口径）
export async function clearRecycle(): Promise<void> {
  const selection = useSelectionStore();
  const toast = useToastStore();
  try {
    const r = await invoke<RecycleDeleteResult>("clear_recycle", { confirmed: false });
    if (r.needConfirm) {
      selection.recyclePrompt = { kind: "clear", assetIds: [], segmentRefs: r.segmentRefs };
      return;
    }
    await runHardDelete(() => `清空回收站（${r.deleted} 个项目）`);
  } catch (e) {
    toast.notify({ message: String(e), icon: "info" });
  }
}

// 确认窗确认后执行（RecycleConfirmDialog 唯一入口）
export async function confirmRecycleDelete(): Promise<void> {
  const selection = useSelectionStore();
  const prompt = selection.recyclePrompt;
  selection.recyclePrompt = null;
  if (!prompt) return;
  const toast = useToastStore();
  try {
    const r =
      prompt.kind === "delete"
        ? await invoke<RecycleDeleteResult>("delete_recycle", {
            assetIds: prompt.assetIds,
            confirmed: true,
          })
        : await invoke<RecycleDeleteResult>("clear_recycle", { confirmed: true });
    await runHardDelete(() =>
      prompt.kind === "delete" ? `彻底删除 ${r.deleted} 个项目` : `清空回收站（${r.deleted} 个项目）`,
    );
  } catch (e) {
    toast.notify({ message: String(e), icon: "info" });
  }
}

// 新建文件夹（2026-07-29 复验三轮改"先建后命名"）：直接以默认名建库，成功返回新文件夹 id
// （调用方据此让树新节点进命名态）；失败 = null + 中文 toast——不再有"命名中临时节点"，
// 也就不再有命名行内报错（旧流程的取消不入库 / 行内提示一并移除）
export async function createFolder(
  parentId: string | null,
  name: string,
  afterFolderId: string | null,
): Promise<string | null> {
  const content = useContentStore();
  const toast = useToastStore();
  const undo = useUndoStore();
  try {
    const folder = await invoke<{ id: string }>("create_folder", { parentId, name, afterFolderId });
    await content.refresh();
    void undo.refreshState();
    toast.notify({ message: "已新建文件夹", actionLabel: "撤销", onAction: () => void undo.undo() });
    return folder.id;
  } catch (e) {
    toast.notify({ message: String(e), icon: "info" });
    return null;
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
