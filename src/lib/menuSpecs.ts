// 六种上下文菜单 spec（6.3，2026-07-29 用户设计 `00_修改与需求/05_右键菜单`）：
// 数据驱动（lib/contextMenus 渲染），设计项全列——未实现的一律 disabled 置灰占位，图标一律空。
// 分组用 "divider"（组标题不出文字，Eagle 风格）。
import { invoke } from "@tauri-apps/api/core";
import { useContentStore, type FolderNode, type PlacementView } from "../stores/content";
import { useSelectionStore } from "../stores/selection";
import { useUiStore } from "../stores/ui";
import { usePreviewStore } from "../stores/preview";
import { type CtxEntry } from "./contextMenus";
import {
  copyAssetLink,
  copyAssetPath,
  copyBase64,
  copyFolderLink,
  copyName,
  createAndName,
  deleteAssetsPermanentlyFlow,
  deleteFolderFlow,
  deleteFolderPermanentlyFlow,
  findParentId,
  pickCustomThumb,
  pickExportAssets,
  pickExportFolder,
  pickFolderForLink,
  revealWithWarning,
} from "./menuOps";
import {
  applyClipboardThumb,
  deletePlacements,
  duplicatePlacements,
  openWithDialog,
  pasteClipboard,
  refreshThumb,
} from "./placementOps";
import { collapseAll, collapseLevel, expandAll, expandLevel, toggleOpen, treeExpansion } from "./treeExpansion";

// 默认应用打开（AssetCard / preview store 同款流程：成功后重拉最近使用）
function openDefaultApp(assetId: string): void {
  void invoke("open_with_default_app", { assetId }).then(() => useContentStore().refreshRecent());
}

// 创建副本（素材卡片右键）：各选中项复制进其自身所在文件夹（副本紧随原件，名称同源）
async function duplicateSelected(): Promise<void> {
  const selection = useSelectionStore();
  const byFolder = new Map<string, ReturnType<typeof selection.selectedRefs>>();
  for (const ref of selection.selectedRefs()) {
    const list = byFolder.get(ref.folderId) ?? [];
    list.push(ref);
    byFolder.set(ref.folderId, list);
  }
  for (const [folderId, items] of byFolder) {
    await duplicatePlacements(items, folderId, items.map(() => null));
  }
}

// 文件夹名（id → 名称；root = 未分类）
function folderName(id: string): string {
  if (id === "root") return "未分类";
  const walk = (nodes: FolderNode[]): string | null => {
    for (const n of nodes) {
      if (n.id === id) return n.name;
      const hit = walk(n.children);
      if (hit) return hit;
    }
    return null;
  };
  return walk(useContentStore().folders) ?? id;
}

// —— 素材卡片（右键未选中项调用方已先单选；批量项对全部选中项执行）——
export function assetCardSpec(p: PlacementView): CtxEntry[] {
  const content = useContentStore();
  const selection = useSelectionStore();
  const assetId = p.assetId;
  const assetKey = `${p.assetId}|${p.folderId}`;
  // 打开文件所在位置：同 assetId 的全部 Placement 所在文件夹去重（含关联文件夹）
  const locations: CtxEntry[] = [
    ...new Set(content.placements.filter((q) => q.assetId === assetId).map((q) => q.folderId)),
  ].map((fid) => ({
    label: folderName(fid),
    action: () => {
      content.sidebarSelected = `folder:${fid}`;
    },
  }));
  return [
    { label: "搜索栏", disabled: true },
    "divider",
    { label: "打开预览", action: () => void usePreviewStore().open(assetKey) },
    { label: "新建窗口预览", action: () => void invoke("open_preview_window", { assetKey }) },
    { label: "发送至 xx", disabled: true },
    { label: "默认应用打开", shortcut: "Shift+Enter", action: () => openDefaultApp(assetId) },
    { label: "其他应用打开", action: () => void openWithDialog(assetId) },
    { label: "在文件管理器中打开", action: () => revealWithWarning(assetId) },
    { label: "打开文件所在位置", submenu: locations.length > 0 ? locations : [{ label: "（无）", disabled: true }] },
    "divider",
    { label: "AE", disabled: true },
    { label: "AN", disabled: true },
    { label: "以图搜图", disabled: true },
    "divider",
    { label: "发送至无限画布", disabled: true },
    { label: "编辑提示词", disabled: true },
    {
      label: "工具箱",
      submenu: [
        { label: "放大", disabled: true },
        { label: "超分", disabled: true },
        { label: "抠图", disabled: true },
      ],
    },
    "divider",
    { label: "添加至文件夹", action: () => pickFolderForLink() },
    "divider",
    {
      label: "重命名",
      shortcut: "F2",
      disabled: selection.count !== 1,
      action: () => {
        selection.renamingKey = assetKey;
      },
    },
    { label: "复制文件", disabled: true },
    { label: "粘贴文件", disabled: true },
    {
      label: "复制",
      submenu: [
        { label: "复制链接", action: () => void copyAssetLink(assetId) },
        { label: "复制文件路径", action: () => void copyAssetPath(assetId) },
        { label: "复制 base64", action: () => void copyBase64(assetId) },
        { label: "复制名称", action: () => void copyName(p.name) },
      ],
    },
    { label: "创建副本", action: () => void duplicateSelected() },
    "divider",
    { label: "复制缩略图", disabled: true },
    { label: "转为缩略图", disabled: true },
    { label: "将 xxx 设置为缩略图", disabled: true },
    { label: "将剪贴板应用至缩略图", disabled: selection.count !== 1, action: () => void applyClipboardThumb(assetId) },
    { label: "自定义缩略图", disabled: selection.count !== 1, action: () => void pickCustomThumb(assetId) },
    { label: "刷新缩略图", action: () => void refreshThumb(assetId) },
    { label: "预览图生成", disabled: true },
    "divider",
    { label: "编辑数据", disabled: true },
    { label: "复制数据", disabled: true },
    "divider",
    { label: "置顶", disabled: true },
    { label: "添加至快捷入口", disabled: true },
    "divider",
    {
      label: "导出文件",
      action: () => void pickExportAssets(selection.selectedRefs().map((r) => r.assetId)),
    },
    { label: "添加至其他资源库", disabled: true },
    { label: "丢弃至回收站", shortcut: "Del", action: () => void deletePlacements(selection.selectedRefs()) },
    {
      label: "彻底删除",
      action: () => void deleteAssetsPermanentlyFlow([...new Set(selection.selectedRefs().map((r) => r.assetId))]),
    },
  ];
}

// —— 素材卡片空白处 ——
export function assetBlankSpec(): CtxEntry[] {
  const ui = useUiStore();
  const selection = useSelectionStore();
  const layoutItem = (label: string, mode: typeof ui.layoutMode) => ({
    label,
    checkable: true,
    checked: ui.layoutMode === mode,
    action: () => {
      ui.layoutMode = mode;
    },
  });
  const briefItem = (label: string, mode: typeof ui.briefMode) => ({
    label,
    checkable: true,
    checked: ui.briefMode === mode,
    action: () => {
      ui.briefMode = mode;
    },
  });
  return [
    {
      label: "布局方式",
      submenu: [
        layoutItem("自适应", "justified"),
        layoutItem("瀑布", "waterfall"),
        layoutItem("网格", "grid"),
        layoutItem("列表", "list"),
      ],
    },
    {
      label: "排序",
      submenu: [
        { label: "自定义", checkable: true, checked: true, disabled: true },
        { label: "标题", disabled: true },
        { label: "时间", disabled: true },
        "divider",
        { label: "升序", disabled: true },
        { label: "降序", disabled: true },
      ],
    },
    {
      label: "显示简介",
      submenu: [
        briefItem("分辨率", "size"),
        briefItem("大小", "fileSize"),
        briefItem("创建日期", "createdAt"),
        briefItem("修改日期", "modifiedAt"),
        briefItem("无", "none"),
      ],
    },
    {
      label: "分组依据",
      submenu: [
        { label: "自定义", checkable: true, checked: true, disabled: true },
        { label: "标题", disabled: true },
      ],
    },
    "divider",
    {
      label: "新建",
      submenu: [
        { label: "Animate", disabled: true },
        { label: "markdown", disabled: true },
        { label: "AE", disabled: true },
        { label: "photoshop", disabled: true },
      ],
    },
    "divider",
    { label: "粘贴到此处", disabled: !selection.clipboard, action: () => void pasteClipboard() },
  ];
}

// —— 文件夹卡片 ——
export function folderCardSpec(folder: { id: string; name: string }): CtxEntry[] {
  const content = useContentStore();
  const selection = useSelectionStore();
  const parentId = findParentId(content.folders, folder.id) ?? null;
  return [
    {
      label: "打开",
      action: () => {
        content.sidebarSelected = `folder:${folder.id}`;
      },
    },
    "divider",
    { label: "新建文件夹", action: () => void createAndName(parentId, folder.id) },
    { label: "新建子文件夹", action: () => void createAndName(folder.id, null) },
    { label: "移动文件夹", disabled: true },
    "divider",
    { label: "添加至快捷入口", disabled: true },
    "divider",
    { label: "复制链接", action: () => void copyFolderLink(folder.id) },
    {
      label: "重命名",
      action: () => {
        selection.namingFolderCardId = folder.id;
      },
    },
    { label: "克隆", disabled: true },
    "divider",
    { label: "导出文件夹", action: () => void pickExportFolder(folder.id) },
    { label: "添加至其他资源库", disabled: true },
    { label: "删除文件夹", action: () => deleteFolderFlow(folder.id, folder.name) },
    { label: "彻底删除", action: () => void deleteFolderPermanentlyFlow(folder.id, folder.name) },
  ];
}

// —— 文件夹卡片空白处 ——
export function folderBlankSpec(): CtxEntry[] {
  const content = useContentStore();
  const current = content.sidebarSelected.startsWith("folder:") ? content.sidebarSelected.slice(7) : null;
  return [{ label: "新建文件夹", action: () => void createAndName(current, null) }];
}

// —— 文件树 ——
export function treeItemSpec(node: FolderNode, parentId: string | null): CtxEntry[] {
  const content = useContentStore();
  const selection = useSelectionStore();
  const ui = useUiStore();
  const hasChildren = node.children.length > 0;
  const sortItem = (label: string, mode: typeof ui.treeSort) => ({
    label,
    checkable: true,
    checked: ui.treeSort === mode,
    action: () => {
      ui.treeSort = mode;
      ui.treeSortAnchor = parentId;
    },
  });
  return [
    {
      label: "打开",
      action: () => {
        content.sidebarSelected = `folder:${node.id}`;
      },
    },
    "divider",
    { label: "新建文件夹", action: () => void createAndName(parentId, node.id) },
    { label: "新建子文件夹", action: () => void createAndName(node.id, null) },
    { label: "移动文件夹", disabled: true },
    "divider",
    { label: "添加至快捷入口", disabled: true },
    "divider",
    { label: "复制链接", action: () => void copyFolderLink(node.id) },
    {
      label: "重命名",
      action: () => {
        selection.namingFolderId = node.id;
      },
    },
    { label: "克隆", disabled: true },
    "divider",
    {
      label: "排序",
      submenu: [
        sortItem("自定义", "custom"),
        sortItem("当前层级标题-升序", "levelAsc"),
        sortItem("当前层级标题-降序", "levelDesc"),
        sortItem("全部层级标题-升序", "allAsc"),
        sortItem("全部层级标题-降序", "allDesc"),
      ],
    },
    {
      label: "展开/折叠当前文件夹",
      disabled: !hasChildren,
      action: () => toggleOpen(node.id),
    },
    {
      label: "展开/折叠同层级文件夹",
      action: () => {
        // 同层级已全开则折叠，否则展开
        const level = parentId === null ? content.folders : (findChildrenList(content.folders, parentId) ?? []);
        const ids = level.filter((n) => n.children.length > 0).map((n) => n.id);
        const allOpen = ids.length > 0 && ids.every((id) => treeExpansion.openSet.has(id));
        if (allOpen) collapseLevel(content.folders, parentId);
        else expandLevel(content.folders, parentId);
      },
    },
    {
      label: "展开/折叠所有文件夹",
      action: () => {
        if (treeExpansion.openSet.size > 0) collapseAll();
        else expandAll(content.folders);
      },
    },
    "divider",
    { label: "更改文件夹图标", disabled: true },
    { label: "更改颜色", disabled: true },
    "divider",
    { label: "导出文件夹", action: () => void pickExportFolder(node.id) },
    { label: "添加至其他资源库", disabled: true },
    { label: "删除文件夹", action: () => deleteFolderFlow(node.id, node.name) },
    { label: "彻底删除", action: () => void deleteFolderPermanentlyFlow(node.id, node.name) },
  ];
}

function findChildrenList(nodes: FolderNode[], id: string): FolderNode[] | null {
  for (const n of nodes) {
    if (n.id === id) return n.children;
    const hit = findChildrenList(n.children, id);
    if (hit) return hit;
  }
  return null;
}

// —— 添加按钮（SidebarLeft +；导入动作由调用方注入）——
export function addButtonSpec(pickImport: (kind: "files" | "folder") => void): CtxEntry[] {
  const content = useContentStore();
  const current = content.sidebarSelected.startsWith("folder:") ? content.sidebarSelected.slice(7) : null;
  return [
    { label: "导入文件", action: () => pickImport("files") },
    { label: "导入文件夹", action: () => pickImport("folder") },
    { label: "导入素材包", disabled: true },
    "divider",
    { label: "新建文件夹", action: () => void createAndName(null, null) },
    { label: "新建子文件夹", disabled: current === null, action: () => void createAndName(current, null) },
    "divider",
    { label: "扫描重复文件", disabled: true },
    "divider",
    {
      label: "新建文档",
      submenu: [
        { label: "Animate", disabled: true },
        { label: "PSD", disabled: true },
        { label: "word", disabled: true },
      ],
    },
  ];
}
