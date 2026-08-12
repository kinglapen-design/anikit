// 右键菜单流程层（6.3，2026-07-29 用户设计）：菜单项的动作流程——系统对话框、
// 确认弹窗（selection.confirmPrompt / folderPickPrompt）、剪贴板写入、新建文件夹共享入口。
// 命令调用一律走 placementOps（UI 唯一入口），本层只做流程编排。
import { open } from "@tauri-apps/plugin-dialog";
import { useContentStore, type FolderNode } from "../stores/content";
import { useSelectionStore } from "../stores/selection";
import { useToastStore } from "../stores/toast";
import {
  createFolder,
  applyClipboardThumb,
  setCustomThumb,
  exportAssets,
  exportFolder,
  deleteFolder,
  getAssetBase64,
  getAssetOriginPath,
  revealInExplorer,
  hasWaitingAsset,
  notifyWaitingAssetLink,
} from "./placementOps";

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

// 同级唯一默认名："新建文件夹"、"新建文件夹-2"…（按目标层级现有兄弟名递进）
export function uniqueFolderName(parentId: string | null): string {
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
  }
  const names = new Set(level.map((n) => n.name));
  if (!names.has("新建文件夹")) return "新建文件夹";
  let i = 2;
  while (names.has(`新建文件夹-${i}`)) i++;
  return `新建文件夹-${i}`;
}

// 统一入口：直接以默认名建库 → 成功返回的新 id 进命名态（FolderTree 按 id 定位 naming 节点）
export async function createAndName(parentId: string | null, afterFolderId: string | null): Promise<void> {
  const id = await createFolder(parentId, uniqueFolderName(parentId), afterFolderId);
  if (id) useSelectionStore().namingFolderId = id;
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

// —— 在文件管理器中打开（警示弹窗前置，2026-07-29 用户设计文案）——
export function revealWithWarning(assetId: string): void {
  const selection = useSelectionStore();
  selection.confirmPrompt = {
    title: "在文件管理器中打开",
    body: "素材保存在 AniKit，请勿移动、删除、重命名任何资源，否则会导致数据丢失。",
    confirmLabel: "继续",
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

// 删除文件夹（素材移未分类，可撤销）：确认弹窗文案 2026-07-29 用户定
export function deleteFolderFlow(folderId: string, folderName: string): void {
  useSelectionStore().confirmPrompt = {
    title: "删除文件夹",
    body: `文件夹「${folderName}」将被删除，素材将全部进入未分类，需重新整理。`,
    confirmLabel: "删除文件夹",
    onConfirm: () => void deleteFolder(folderId),
  };
}

// —— 添加至文件夹（关联或副本）：文件夹选择弹窗 → 关联弹窗（LinkDialog）——
export function pickFolderForLink(): void {
  const selection = useSelectionStore();
  const items = selection.selectedRefs();
  if (items.length === 0) return;
  // 候车区资产禁发起链接（PRD 02-15，2.5）
  if (hasWaitingAsset(items)) {
    notifyWaitingAssetLink();
    return;
  }
  selection.folderPickPrompt = {
    onPick: (folderId) => {
      selection.linkDraft = { items, targetFolder: folderId };
    },
  };
}
