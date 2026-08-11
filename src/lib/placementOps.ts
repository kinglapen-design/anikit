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
    toast.notify({ message: String(e), icon: "fail" });
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

// 删除（C1；2026-07-29 用户修订：取消两段式询问——回收站不是危险操作，素材没丢，
// 撤销可找回，二次确认纯增操作负担）：直接 lastAction="recycle" 移入回收站，
// 不再有"保留为未分类"选项（入口不同而已）；后端两段式机制保留（lastAction=null 干跑）
export async function deletePlacements(items: PlacementRef[]): Promise<void> {
  const toast = useToastStore();
  try {
    await invoke("delete_placements", { items, lastAction: "recycle" });
    await runOp(async () => {}, () => `删除 ${items.length} 个项目`);
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
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
    toast.notify({ message: String(e), icon: "fail" });
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
    toast.notify({ message: String(e), icon: "fail" });
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
    toast.notify({ message: String(e), icon: "fail" });
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
    toast.notify({ message: String(e), icon: "fail" });
    return null;
  }
}

// 新建文本文档入库（1.3，07 新增菜单"MarkDown 文档 / 文本文档"点亮，2026-08-11 用户定）：
// 落当前视口目录（同导入位置口径），先建后命名——成功返回 PlacementView 后 renamingKey 进重命名态
export async function createTextDocument(kind: "md" | "txt"): Promise<void> {
  const content = useContentStore();
  const selection = useSelectionStore();
  const toast = useToastStore();
  const undo = useUndoStore();
  try {
    const view = await invoke<{ assetId: string; folderId: string }>("create_text_document", {
      kind,
      targetFolder: content.currentFolderId,
    });
    await content.refresh();
    void undo.refreshState();
    selection.renamingKey = `${view.assetId}|${view.folderId}`;
    toast.notify({ message: "已新建文档", actionLabel: "撤销", onAction: () => void undo.undo() });
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
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
    icon: "fail",
    duration: 3000,
  });
}

// 粘贴落点判定（2026-07-28 定案）：快速访问视图不作粘贴目标
function pasteTarget(): string | null {
  const content = useContentStore();
  if (!content.sidebarSelected.startsWith("folder:")) {
    useToastStore().notify({ message: "快速访问视图不支持粘贴", icon: "fail" });
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

// —— 6.3 右键菜单配套（2026-07-29，设计稿 `00_修改与需求/05_右键菜单`）——

// 其他应用打开（系统"打开方式"对话框）：失败中文 toast
export async function openWithDialog(assetId: string): Promise<void> {
  try {
    await invoke("open_with_dialog", { assetId });
  } catch (e) {
    useToastStore().notify({ message: String(e), icon: "fail" });
  }
}

// 在文件管理器中打开（前端先弹警示窗再调——菜单流程见 menuOps.revealWithWarning）
export async function revealInExplorer(assetId: string): Promise<void> {
  try {
    await invoke("reveal_in_explorer", { assetId });
  } catch (e) {
    useToastStore().notify({ message: String(e), icon: "fail" });
  }
}

// 缩略图三件套：不接撤销（同设封面帧既有口径）；成功刷新 + 普通 toast
async function runThumbOp(action: () => Promise<unknown>, okMessage: string): Promise<void> {
  const content = useContentStore();
  const toast = useToastStore();
  try {
    await action();
    await content.refresh();
    toast.notify({ message: okMessage, icon: "success" });
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  }
}

// 将剪贴板应用至缩略图（2026-07-23 待办落地）
export async function applyClipboardThumb(assetId: string): Promise<void> {
  await runThumbOp(() => invoke("set_thumbnail_from_clipboard", { assetId }), "已将剪贴板应用至缩略图");
}

// 自定义缩略图（系统文件选择器选定后调用）
export async function setCustomThumb(assetId: string, sourcePath: string): Promise<void> {
  await runThumbOp(() => invoke("set_thumbnail_custom", { assetId, sourcePath }), "已自定义缩略图");
}

// 刷新缩略图（强制重生成，防自检失败）
export async function refreshThumb(assetId: string): Promise<void> {
  await runThumbOp(() => invoke("refresh_thumbnail", { assetId }), "已刷新缩略图");
}

// 导出（库外写操作，不接撤销）：目录由系统对话框选定（menuOps 流程），成功普通 toast
export async function exportAssets(assetIds: string[], targetDir: string): Promise<void> {
  const toast = useToastStore();
  try {
    const n = await invoke<number>("export_assets", { assetIds, targetDir });
    toast.notify({ message: `已导出 ${n} 个文件`, icon: "success", sound: true });
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  }
}

export async function exportFolder(folderId: string, targetDir: string): Promise<void> {
  const toast = useToastStore();
  try {
    const n = await invoke<number>("export_folder", { folderId, targetDir });
    toast.notify({ message: `已导出 ${n} 个文件`, icon: "success", sound: true });
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  }
}

// 彻底删除素材（两段式，不经过回收站；无撤销）——干跑命中虚拟片段引用返回 null
// （调用方弹确认窗后再调 confirmed=true）；直接成功返回 deleted
export async function deleteAssetsPermanently(
  assetIds: string[],
  confirmed: boolean,
): Promise<RecycleDeleteResult | null> {
  try {
    const r = await invoke<RecycleDeleteResult>("delete_assets_permanently", { assetIds, confirmed });
    if (r.needConfirm) return r;
    await runHardDelete(() => `彻底删除 ${r.deleted} 个项目`);
    return null;
  } catch (e) {
    useToastStore().notify({ message: String(e), icon: "fail" });
    return null;
  }
}

// 删除文件夹报告（后端 DeleteFolderReport 同构）
export interface DeleteFolderResult {
  needConfirm: boolean;
  segmentRefs: number;
  assetCount: number;
  folderCount: number;
  movedPlacements: number;
  deletedFolders: number;
  deleted: number;
}

// 删除文件夹（素材移未分类，可撤销）：成功 toast 带撤销
export async function deleteFolder(folderId: string): Promise<boolean> {
  let report: DeleteFolderResult | null = null;
  return await runOp(async () => {
    report = await invoke<DeleteFolderResult>("delete_folder", { folderId, permanent: false, confirmed: false });
  }, () => `删除文件夹（${report?.movedPlacements ?? 0} 个素材移入未分类）`);
}

// 彻底删除文件夹（两段式，物理递归；无撤销）：needConfirm 返回报告（调用方弹确认窗），
// 否则执行并 toast
export async function deleteFolderPermanently(folderId: string, confirmed: boolean): Promise<DeleteFolderResult | null> {
  try {
    const r = await invoke<DeleteFolderResult>("delete_folder", { folderId, permanent: true, confirmed });
    if (r.needConfirm) return r;
    await runHardDelete(() => `彻底删除文件夹（${r.deleted} 个素材）`);
    return null;
  } catch (e) {
    useToastStore().notify({ message: String(e), icon: "fail" });
    return null;
  }
}

// 复制 base64（后端守卫：仅静态图片且 ≤10MB）
export async function getAssetBase64(assetId: string): Promise<string | null> {
  try {
    return await invoke<string>("get_asset_base64", { assetId });
  } catch (e) {
    useToastStore().notify({ message: String(e), icon: "fail" });
    return null;
  }
}

// 资产源文件真实绝对路径（复制文件路径用；后端恒返回 info 内 origin，与预览路径解析无关）
export async function getAssetOriginPath(assetId: string): Promise<string | null> {
  try {
    return await invoke<string>("get_asset_origin_path", { assetId });
  } catch (e) {
    useToastStore().notify({ message: String(e), icon: "fail" });
    return null;
  }
}
