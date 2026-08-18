// 右键菜单流程层（6.3，2026-07-29 用户设计）：菜单项的动作流程——系统对话框、
// 确认弹窗（selection.confirmPrompt / folderPickPrompt）、剪贴板写入、新建文件夹共享入口。
// 命令调用一律走 placementOps（UI 唯一入口），本层只做流程编排。
import { open } from "@tauri-apps/plugin-dialog";
import { useContentStore, DEFAULT_BLOCK_ID, type FolderNode } from "../stores/content";
import { useSelectionStore } from "../stores/selection";
import { useTabsStore } from "../stores/tabs";
import { useToastStore } from "../stores/toast";
import {
  createFolder,
  createArchive,
  convertToArchive,
  convertToFolder,
  applyClipboardThumb,
  setCustomThumb,
  exportAssets,
  exportFolder,
  cutFolders,
  deleteFolder,
  deleteFolders,
  getAssetBase64,
  getAssetOriginPath,
  revealInExplorer,
} from "./placementOps";
import { levelDisplaySorted, materializeLevelOrder } from "./dragFolderOps";

// —— 导入系统对话框（1.3：新增菜单三入口共用——侧栏新增按钮 / 空白右键 / Ctrl+N）——
// PRD 05-01 导入位置：导入文件 → 当前视口目录；导入文件夹 → 根目录
export async function pickImportPaths(kind: "files" | "folder"): Promise<void> {
  const picked = await open(kind === "files" ? { multiple: true } : { directory: true, multiple: false });
  if (!picked) return;
  const content = useContentStore();
  content.importDraft = {
    kind,
    paths: Array.isArray(picked) ? picked : [picked],
    targetFolder: kind === "files" ? content.currentFolderId : null,
  };
}

// —— 新建文件夹（先建后命名，自 LibraryView 提为共享：树右键 / 文件夹卡片 / 添加按钮共用）——

// 父级查找（建兄弟用）：命中顶层 = null（根级）
export function findParentId(nodes: FolderNode[], id: string, parent: string | null = null): string | null | undefined {
  for (const n of nodes) {
    if (n.id === id) return parent;
    const hit = findParentId(n.children, id, n.id);
    if (hit !== undefined) return hit;
  }
  return undefined;
}

// 同级唯一默认名："新建文件夹"、"新建文件夹-2"…（按目标层级现有兄弟名递进；
// 2026-08-14 多区块：顶级排重池收窄到同区块——"同级" = 同区块根级）；
// base（2026-08-15 档案子系统一轮）：档案新建用"新建档案"基座
export function uniqueFolderName(parentId: string | null, blockId: string | null = null, base = "新建文件夹"): string {
  const content = useContentStore();
  let level = content.folders;
  if (parentId) {
    const walk = (nodes: FolderNode[]): FolderNode[] | null => {
      for (const n of nodes) {
        if (n.id === parentId) return n.children;
        const hit = walk(n.children);
        if (hit) return hit;
      }
      return null;
    };
    level = walk(content.folders) ?? [];
  } else {
    const b = blockId ?? DEFAULT_BLOCK_ID;
    level = level.filter((n) => (n.block ?? DEFAULT_BLOCK_ID) === b);
  }
  const names = new Set(level.map((n) => n.name));
  if (!names.has(base)) return base;
  let i = 2;
  while (names.has(`${base}-${i}`)) i++;
  return `${base}-${i}`;
}

// 同级新建的区块归属（2026-08-14 区块子系统）：目标为顶级文件夹时跟随其所在区块——
// 否则在非默认区块的顶级文件夹旁建同级会落进「目录」；子级新建不需要（区块不继承）
export function topLevelBlockId(nodes: FolderNode[], id: string): string | null {
  return nodes.find((n) => n.id === id)?.block ?? null;
}

// 统一入口：直接以默认名建库 → 成功返回的新 id 进命名态（FolderTree 按 id 定位 naming 节点）。
// blockId（2026-08-14 区块子系统）：顶级新建（parentId null）时落入的区块；非顶级新建无效。
// 建后清理（2026-08-14 用户定）：其他选中一律清掉单选新文件夹 + 跳转其视图——
// 否则旧选中域与新项双高亮并存，一眼无法命中新建项
export async function createAndName(
  parentId: string | null,
  afterFolderId: string | null,
  blockId: string | null = null,
): Promise<void> {
  // 名称排序显示下新建（2026-08-14 用户定：排序依据加"自定义"项，手动操作自动切回并物化可见序）——
  // 先物化该层再建，新文件夹才落在"选中项下一行"（否则被名称排序盖回，落点看起来乱跑）
  if (levelDisplaySorted(parentId)) {
    try {
      await materializeLevelOrder(parentId, blockId);
    } catch (e) {
      useToastStore().notify({ message: String(e), icon: "fail" });
      return;
    }
  }
  const id = await createFolder(parentId, uniqueFolderName(parentId, blockId), afterFolderId, blockId);
  if (!id) return;
  const selection = useSelectionStore();
  selection.clickFolder(id); // 单选新文件夹（clickFolder 内已互斥清素材/收件箱域）
  useTabsStore().openView(`folder:${id}`);
  selection.namingFolderId = id;
}

// 新建档案（2026-08-15 档案子系统一轮）：流程同 createAndName（排序物化 → 先建后命名 →
// 选中清理 → 命名态），默认名基座"新建档案"
export async function createArchiveAndName(
  parentId: string | null,
  afterFolderId: string | null,
  blockId: string | null = null,
): Promise<void> {
  if (levelDisplaySorted(parentId)) {
    try {
      await materializeLevelOrder(parentId, blockId);
    } catch (e) {
      useToastStore().notify({ message: String(e), icon: "fail" });
      return;
    }
  }
  const id = await createArchive(parentId, uniqueFolderName(parentId, blockId, "新建档案"), afterFolderId, blockId);
  if (!id) return;
  const selection = useSelectionStore();
  selection.clickFolder(id);
  useTabsStore().openView(`folder:${id}`);
  selection.namingFolderId = id;
}

// 转换为档案（3.2 一轮两段式）：有子文件夹先弹警告确认（figma 25_警告 - 转换为档案，
// AppModal alert warning 变体）——确认后剔除层级转换；失败 toast 归 convertToArchive
export function convertToArchiveFlow(folderId: string): void {
  void (async () => {
    const r = await convertToArchive(folderId, false);
    if (r !== "need_confirm") return;
    useSelectionStore().confirmPrompt = {
      title: "该文件夹内有子文件夹存在，是否剔除层级转换！",
      body: "档案下不允许出现子级文件夹，可保留全部文件剔除层级转换。",
      confirmLabel: "剔除转换",
      alert: "warning",
      onConfirm: () => void convertToArchive(folderId, true),
    };
  })();
}

// 解散档案为文件夹（3.2 一轮）：直转，失败 toast 归 convertToFolder
export function dissolveArchiveFlow(folderId: string): void {
  void convertToFolder(folderId);
}

// —— 剪贴板写入（复制链接 / 路径 / base64 / 名称）——

async function copyText(text: string, okMessage: string): Promise<void> {
  const toast = useToastStore();
  try {
    await navigator.clipboard.writeText(text);
    toast.notify({ message: okMessage, icon: "success", duration: 3000 });
  } catch {
    toast.notify({ message: "剪贴板写入失败", icon: "fail" });
  }
}

// 复制链接：anikit://item/{assetId}（PRD 02-06 ID 寻址）
export function copyAssetLink(assetId: string): Promise<void> {
  return copyText(`anikit://item/${assetId}`, "已复制链接");
}

// 文件夹复制链接：anikit://folder/{id}（item 同构扩展，2026-07-29）
export function copyFolderLink(folderId: string): Promise<void> {
  return copyText(`anikit://folder/${folderId}`, "已复制链接");
}

export async function copyAssetPath(assetId: string): Promise<void> {
  const path = await getAssetOriginPath(assetId);
  if (path) await copyText(path, "已复制文件路径");
}

export async function copyBase64(assetId: string): Promise<void> {
  const b64 = await getAssetBase64(assetId);
  if (b64) await copyText(b64, "已复制 base64");
}

export function copyName(name: string): Promise<void> {
  return copyText(name, "已复制名称");
}

// —— 在文件管理器中打开（警示弹窗前置，2026-07-29 用户设计文案；
// 2026-08-13 用户定：高危操作走 alert 危险弹出模板——48 警告徽章 + 提示音）——
export function revealWithWarning(assetId: string): void {
  const selection = useSelectionStore();
  selection.confirmPrompt = {
    title: "在文件管理器中打开",
    body: "素材保存在 AniKit，请勿移动、删除、重命名任何资源，否则会导致数据丢失。",
    confirmLabel: "继续",
    alert: "warning",
    onConfirm: () => void revealInExplorer(assetId),
  };
}

// —— 缩略图流程 ——

// 自定义缩略图：系统文件选择器（图片 / 动图格式）
export async function pickCustomThumb(assetId: string): Promise<void> {
  const picked = await open({
    multiple: false,
    filters: [{ name: "图片", extensions: ["png", "jpg", "jpeg", "bmp", "webp", "gif"] }],
  });
  if (typeof picked === "string") await setCustomThumb(assetId, picked);
}

export { applyClipboardThumb };

// —— 导出流程：系统目录选择器 ——

export async function pickExportAssets(assetIds: string[]): Promise<void> {
  if (assetIds.length === 0) return;
  const dir = await open({ directory: true, multiple: false });
  if (typeof dir === "string") await exportAssets(assetIds, dir);
}

export async function pickExportFolder(folderId: string): Promise<void> {
  const dir = await open({ directory: true, multiple: false });
  if (typeof dir === "string") await exportFolder(folderId, dir);
}

// —— 删除流程（确认弹窗编排）——

// 剪切文件夹（3.4，2026-08-18）：批量分流同 deleteFolderFlow——触发项在多选域内 → 剪切整个选中集；
// 粘贴 = move_folders 移动（pasteClipboard folders 分支）
export function cutFoldersFlow(folderId: string): void {
  const selection = useSelectionStore();
  const ids =
    selection.selectedFolders.size > 1 && selection.selectedFolders.has(folderId)
      ? selection.orderedSelectedFolderIds()
      : [folderId];
  cutFolders(ids);
}

// —— 移动面板（3.4，29 稿）：打开 = 自选择域快照进 moveDraft（宿主 App.vue MovePanel）——

// 移动文件（素材卡片菜单 / Ctrl+Y）：对当前素材选中集
export function moveAssetsFlow(): void {
  const selection = useSelectionStore();
  const refs = selection.selectedRefs();
  if (refs.length === 0) return;
  selection.moveDraft = { kind: "assets", refs };
}

// 移动文件夹 / 档案（菜单 / Ctrl+Y）：批量分流同 deleteFolderFlow
export function moveFoldersFlow(folderId: string): void {
  const selection = useSelectionStore();
  const ids =
    selection.selectedFolders.size > 1 && selection.selectedFolders.has(folderId)
      ? selection.orderedSelectedFolderIds()
      : [folderId];
  selection.moveDraft = { kind: "folders", ids };
}

// 删除文件夹（素材移未分类，可撤销）：确认弹窗文案 2026-07-29 用户定；
// 批量分流（3.4，2026-08-18）：触发项在文件夹多选域内（卡片 / 树右键已保有多选）→
// 整个选中集走批量流程，不再只删被右键的一个
export function deleteFolderFlow(folderId: string, folderName: string): void {
  const selection = useSelectionStore();
  if (selection.selectedFolders.size > 1 && selection.selectedFolders.has(folderId)) {
    deleteFoldersFlow(selection.orderedSelectedFolderIds());
    return;
  }
  selection.confirmPrompt = {
    title: "删除文件夹",
    body: `文件夹「${folderName}」将被删除，素材将全部进入未分类，需重新整理。`,
    confirmLabel: "删除文件夹",
    onConfirm: () => void deleteFolder(folderId),
  };
}

// 批量删除文件夹（3.4）：一次确认 + 后端 delete_folders 单撤销步；
// 单元素时回退单删文案（键盘 Delete 分流入口不分单复数，统一走这里）
export function deleteFoldersFlow(folderIds: string[]): void {
  if (folderIds.length === 0) return;
  const selection = useSelectionStore();
  if (folderIds.length === 1) {
    const name = useContentStore().folderName(folderIds[0]) ?? folderIds[0];
    selection.confirmPrompt = {
      title: "删除文件夹",
      body: `文件夹「${name}」将被删除，素材将全部进入未分类，需重新整理。`,
      confirmLabel: "删除文件夹",
      onConfirm: () => void deleteFolder(folderIds[0]),
    };
    return;
  }
  selection.confirmPrompt = {
    title: "删除文件夹",
    body: `已选中的 ${folderIds.length} 个文件夹将被删除，素材将全部进入未分类，需重新整理。`,
    confirmLabel: "删除文件夹",
    onConfirm: () => void deleteFolders(folderIds),
  };
}

// —— 添加至文件夹（关联或副本）：文件夹选择弹窗 → 关联弹窗（LinkDialog）——
export function pickFolderForLink(): void {
  const selection = useSelectionStore();
  const items = selection.selectedRefs();
  if (items.length === 0) return;
  // 发起不设限（2026-08-13 用户定案：不替用户做整理决定，候车区资产拦截废除）
  selection.folderPickPrompt = {
    onPick: (folderId) => {
      selection.linkDraft = { items, targetFolder: folderId };
    },
  };
}
