// 右键菜单流程层（6.3，2026-07-29 用户设计）：菜单项的动作流程——系统对话框、
// 确认弹窗（selection.confirmPrompt）、剪贴板写入、新建文件夹共享入口。
// 命令调用一律走 placementOps（UI 唯一入口），本层只做流程编排。
import { open } from "@tauri-apps/plugin-dialog";
import { useContentStore, DEFAULT_BLOCK_ID, type FolderNode, type PlacementView } from "../stores/content";
import { usePickerStore } from "../stores/picker";
import { useSelectionStore } from "../stores/selection";
import { useTabsStore } from "../stores/tabs";
import { useToastStore } from "../stores/toast";
import { useUiStore } from "../stores/ui";
import {
  createFolder,
  createArchive,
  convertToArchive,
  convertToFolder,
  applyClipboardThumb,
  setCustomThumb,
  exportAssets,
  exportFolder,
  cloneFolders,
  cutFolders,
  deleteFolder,
  deleteFolders,
  dissolveSequence,
  getAssetBase64,
  getAssetOriginPath,
  revealInExplorer,
} from "./placementOps";
import { levelDisplaySorted, materializeLevelOrder } from "./dragFolderOps";
import { treeExpansion } from "./treeExpansion";

// —— 导入系统对话框（1.3：新增菜单三入口共用——侧栏新增按钮 / 空白右键 / Ctrl+N）——
// PRD 05-01 导入位置：导入文件 → 当前视口目录；导入文件夹 → 根目录
export async function pickImportPaths(kind: "files" | "folder"): Promise<void> {
  const content = useContentStore();
  // 只读视口兜底（2026-09-08 用户定，isNoImportView 统一判定；2026-09-09 收窄扩五视口加
  // 「全部」）：禁导入视口下任何入口触达都拒入弹窗——菜单栏两项已 disabled、空态双钮不出
  //（范围改 folder: + 未分类），本道兜底防未来新增调用方漏判；未分类放行（落点 root）
  if (content.isNoImportView) {
    useUiStore().importRejectOpen = true;
    return;
  }
  const picked = await open(kind === "files" ? { multiple: true } : { directory: true, multiple: false });
  if (!picked) return;
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
  // 新建子文件夹默认展开父级（2026-08-20 用户定：让用户直接看到新建结果——forceOpenIds 只是
  // 命名期强制展开，命名结束后会随父级原折叠态收回去；写入 openSet 才真正保持展开）
  if (parentId) treeExpansion.openSet.add(parentId);
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
  if (parentId) treeExpansion.openSet.add(parentId); // 同 createAndName：新建子级默认展开父级
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

// —— 解散序列帧包导入（5.2，PRD 05-序列帧打包）：危险操作（改变物理磁盘、不可撤销）——
// alert warning 弹窗明示：帧各自成独立素材（落至原包所在文件夹）、原包删除、逐帧笔记随删、不可撤销
export function dissolveSequenceFlow(p: PlacementView): void {
  const frames = p.frameCount ?? 0;
  const notes = p.noteCount ?? 0;
  const body =
    `「${p.name}」的 ${frames} 张序列帧将各自解散为独立素材（落至原包所在文件夹），` +
    `原序列帧包将一并删除。` +
    (notes > 0 ? `包的 ${notes} 条逐帧笔记将随之删除。` : "") +
    `此操作不可撤销。`;
  useSelectionStore().confirmPrompt = {
    title: "解散序列帧包导入",
    body,
    confirmLabel: "解散导入",
    alert: "warning",
    onConfirm: () => void dissolveSequence(p.assetId),
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

// 克隆文件夹（3.4，2026-08-18）：复制克隆无弹窗（用户定：没有链接克隆——文件夹没有链接语义）；
// 批量分流同 deleteFolderFlow（多选域内触发 → 整个选中集逐棵克隆，单命令单撤销步）
export function cloneFolderFlow(folderId: string): void {
  const selection = useSelectionStore();
  const ids =
    selection.selectedFolders.size > 1 && selection.selectedFolders.has(folderId)
      ? selection.orderedSelectedFolderIds()
      : [folderId];
  void cloneFolders(ids);
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
    kind: "warning", // 2026-09-08 用户定：删除语义 = 警告黄钮（旧徽章没了，纯靠按钮颜色区分）
    onConfirm: () => void deleteFolder(folderId),
  };
}

// 选中域按形态分流（2026-09-05 走查「上下文菜单」档案节：同时多选档案和文件夹时，
// 右键删除分两项——「删除文件夹」只删文件夹 /「将档案丢到回收站」只删档案）：
// 返回 null = 非混选（单选或同构多选），调用方照旧单项流程
export function splitSelectedFolderKinds(triggerId: string): { folders: string[]; archives: string[] } | null {
  const selection = useSelectionStore();
  if (!(selection.selectedFolders.size > 1 && selection.selectedFolders.has(triggerId))) return null;
  const content = useContentStore();
  const isArchive = (id: string): boolean => {
    const walk = (nodes: FolderNode[]): boolean => {
      for (const n of nodes) {
        if (n.id === id) return n.kind === "archive";
        if (walk(n.children)) return true;
      }
      return false;
    };
    return walk(content.folders);
  };
  const folders: string[] = [];
  const archives: string[] = [];
  for (const id of selection.orderedSelectedFolderIds()) (isArchive(id) ? archives : folders).push(id);
  return folders.length > 0 && archives.length > 0 ? { folders, archives } : null;
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
      kind: "warning",
      onConfirm: () => void deleteFolder(folderIds[0]),
    };
    return;
  }
  selection.confirmPrompt = {
    title: "删除文件夹",
    body: `已选中的 ${folderIds.length} 个文件夹将被删除，素材将全部进入未分类，需重新整理。`,
    confirmLabel: "删除文件夹",
    kind: "warning",
    onConfirm: () => void deleteFolders(folderIds),
  };
}

// —— 添加文件/档案链接（6.2 三轮，PRD 03_数据链接 04）：主窗口选择模式文件模板——
// 文件夹/档案右键 Ctrl+L = 落点为被右键项；工具栏 / 新增菜单钮 = 落点当前文件夹视口
//（旧流程 = 文件夹选择弹窗 + LinkDialog 关联弹窗，2026-08-24 用户定随选择模式废弃）
export function enterLinkPicker(folder: { id: string; name: string }): void {
  usePickerStore().enterLink(folder);
}

/** 指定视口的链接落点（缺省 = 全局当前视口 sidebarSelected；工具栏传本标签页 viewKey）：
 *  folder 视口 = 该文件夹/档案；未分类 = root；其余视口（全部 / 回收站 / 收件箱 / 最近使用）
 *  = null（不可添加，菜单 / 工具栏钮禁用） */
export function currentLinkTarget(viewKey?: string): { id: string; name: string } | null {
  const content = useContentStore();
  const vk = viewKey ?? content.sidebarSelected;
  if (vk.startsWith("folder:")) {
    return { id: vk.slice(7), name: content.viewTitle(vk) };
  }
  if (vk === "quick:uncategorized") return { id: "root", name: content.viewTitle("quick:uncategorized") };
  return null;
}
