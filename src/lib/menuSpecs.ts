// 上下文菜单 spec（1.3 一轮 2026-08-11 按新稿 `02_布局` 03/04/05/07 重写；旧 6.3 版基于 `05_右键菜单` 废止）：
// 数据驱动（lib/contextMenus 渲染），设计项全列 + 图标按稿配（稿里无图标的项不画蛇添足，2026-08-11 走查）——
// 未实现的一律 disabled 置灰占位（占位点亮机制）。分组用 "divider"（组标题不出文字，Eagle 风格）。
// 03/04/05 稿 = 宽 320 + 搜索框（openGridContextMenu / FolderStrip 调用处传 opts）；07 稿 = 宽 320 无搜索框。
// 菜单快捷键标注按新稿（重命名 ctrl+R 等，显示小写化在 MenuItem 统一做）；
// 全局快捷键绑定另有一套，统一对齐记 09 待办。
import { invoke } from "@tauri-apps/api/core";
import { useContentStore, type FolderNode, type PlacementView } from "../stores/content";
import { useSelectionStore } from "../stores/selection";
import { useTabsStore } from "../stores/tabs";
import { useUiStore } from "../stores/ui";
import { usePreviewStore } from "../stores/preview";
import { openContextMenu, type CtxEntry, type CtxItem } from "./contextMenus";
import {
  copyAssetLink,
  copyAssetPath,
  copyBase64,
  copyFolderLink,
  copyName,
  createAndName,
  deleteFolderFlow,
  findParentId,
  pickCustomThumb,
  pickExportAssets,
  pickExportFolder,
  pickFolderForLink,
  revealWithWarning,
} from "./menuOps";
import {
  applyClipboardThumb,
  copySelection,
  deletePlacements,
  duplicatePlacements,
  openWithDialog,
  pasteClipboard,
  pasteDuplicate,
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

function findNode(nodes: FolderNode[], id: string): FolderNode | null {
  for (const n of nodes) {
    if (n.id === id) return n;
    const hit = findNode(n.children, id);
    if (hit) return hit;
  }
  return null;
}

function findChildrenList(nodes: FolderNode[], id: string): FolderNode[] | null {
  for (const n of nodes) {
    if (n.id === id) return n.children;
    const hit = findChildrenList(n.children, id);
    if (hit) return hit;
  }
  return null;
}

// —— 03 稿 · 素材卡片（右键未选中项调用方已先单选；批量项对全部选中项执行）——
// 上下文规则（稿原文）：无复制/剪切 → 三个粘贴项不出现；非序列帧 → 序列帧两项不出现；
// 链接文件删除组只出「从文件夹中移除文件链接」，非链接出「从文件夹中移除 / 丢到回收站」
//（稿 prose 另提及"去到未分类"，2026-08-11 用户确认无此项，不列）；
// 无提示词 → 「打开提示词编辑器」改「保存提示词」（提示词数据未落地，恒显示保存提示词占位，记 09 待办）
export function assetCardSpec(p: PlacementView): CtxEntry[] {
  const content = useContentStore();
  const selection = useSelectionStore();
  const assetId = p.assetId;
  const assetKey = `${p.assetId}|${p.folderId}`;
  const isSequence = p.frameCount !== null && !p.animated; // 序列帧（动图同 frameCount 字段，靠 animated 区分）
  const isLink = !p.origin;
  const hasClipboard = selection.clipboard !== null;
  // 打开文件链接所在位置：同 assetId 的全部 Placement 所在文件夹去重（含关联文件夹）
  const locations: CtxEntry[] = [
    ...new Set(content.placements.filter((q) => q.assetId === assetId).map((q) => q.folderId)),
  ].map((fid) => ({
    label: folderName(fid),
    action: () => {
      useTabsStore().openView(`folder:${fid}`);
    },
  }));
  const entries: CtxEntry[] = [
    { label: "打开预览", icon: "open-preview", shortcut: "Enter", action: () => void usePreviewStore().open(assetKey) },
  ];
  if (isSequence) {
    entries.push(
      { label: "打开序列帧包", icon: "open-sequence-pack", disabled: true },
      { label: "解散序列帧包导入", icon: "import-sequence-unpack", disabled: true },
    );
  }
  entries.push(
    {
      label: "新标签页打开",
      icon: "open-in-new-tab",
      shortcut: "Ctrl+Enter",
      disabled: true, // 文件无独立标签页视口业务（标签页 = 文件夹视口），占位
    },
    { label: "新窗口打开", icon: "open-new-window", action: () => void invoke("open_preview_window", { assetKey }) },
    { label: "在默认应用打开", icon: "open-default", shortcut: "Shift+Enter", action: () => openDefaultApp(assetId) },
    { label: "在其他应用打开", icon: "open-in-other-app", action: () => void openWithDialog(assetId) },
    { label: "发送至 Animate", icon: "send-to-animate", shortcut: "Ctrl+E", disabled: true },
    { label: "在资源管理器打开", icon: "open-in-explorer", action: () => revealWithWarning(assetId) },
    "divider",
    { label: "移动文件", icon: "move-file", shortcut: "Ctrl+Y", disabled: true },
    { label: "文件链接管理", icon: "file-link-manage", shortcut: "Ctrl+J", disabled: true },
    {
      label: "打开文件链接所在位置",
      icon: "reveal-file-link",
      submenu: locations.length > 0 ? locations : [{ label: "（无）", disabled: true }],
    },
    "divider",
    { label: "剪切文件", icon: "cut-file", shortcut: "Ctrl+X", action: () => copySelection("cut") },
    { label: "复制文件", icon: "copy-file", shortcut: "Ctrl+C", action: () => copySelection("copy") },
  );
  if (hasClipboard) {
    entries.push(
      { label: "作为链接粘贴", icon: "paste-as-link", shortcut: "Ctrl+V", action: () => void pasteClipboard() },
      { label: "作为链接粘贴并重命名", icon: "paste-as-link-rename", shortcut: "Ctrl+Shift+V", disabled: true },
      { label: "作为副本粘贴", icon: "paste-as-copy", shortcut: "Ctrl+Alt+V", action: () => void pasteDuplicate() },
    );
  }
  entries.push(
    {
      label: "重命名",
      icon: "rename",
      shortcut: "Ctrl+R",
      disabled: selection.count !== 1,
      action: () => {
        selection.renamingKey = assetKey;
      },
    },
    "divider",
    { label: "创建副本", icon: "duplicate", shortcut: "Ctrl+D", action: () => void duplicateSelected() },
    "divider",
    {
      label: "缩略图",
      icon: "thumbnail",
      submenu: [
        { label: "打开 Animate 生成缩略图", disabled: true },
        { label: "复制缩略图", disabled: true },
        { label: "转换为缩略图链接并复制", disabled: true },
        { label: "将「xx」设置为缩略图", disabled: true },
        "divider",
        {
          label: "自定义缩略图 - 从剪贴板",
          disabled: selection.count !== 1,
          action: () => void applyClipboardThumb(assetId),
        },
        {
          label: "自定义缩略图 - 选择文件",
          disabled: selection.count !== 1,
          action: () => void pickCustomThumb(assetId),
        },
        "divider",
        { label: "刷新缩略图", action: () => void refreshThumb(assetId) },
      ],
    },
    "divider",
    {
      label: "添加至其他资源库",
      icon: "add-to-library",
      submenuWidth: 180,
      submenu: [{ label: "（暂无其他资源库）", disabled: true }],
    },
    {
      label: "导出文件",
      icon: "export-file",
      submenu: [
        { label: "导出为素材包", icon: "import-pack", disabled: true },
        {
          label: "导出到本地磁盘",
          icon: "export-to-disk",
          action: () => void pickExportAssets(selection.selectedRefs().map((r) => r.assetId)),
        },
        { label: "导出提示词为 .MD 文档", icon: "open-markdown-editor", disabled: true },
      ],
    },
    "divider",
    {
      label: "AI 工具集",
      icon: "ai-tools",
      submenu: [
        { label: "以图搜图", disabled: true },
        "divider",
        { label: "超清放大", disabled: true },
        { label: "自动抠图", disabled: true },
        { label: "裁剪", disabled: true },
        "divider",
        { label: "分析文件向量", disabled: true },
        "divider",
        { label: "引用至 AI 对话框", disabled: true },
      ],
    },
    "divider",
    { label: "保存提示词", icon: "save-prompt", disabled: true },
    {
      label: "复制数据",
      icon: "copy-data",
      submenu: [
        { label: "复制链接", action: () => void copyAssetLink(assetId) },
        { label: "复制文件路径", action: () => void copyAssetPath(assetId) },
        "divider",
        { label: "复制名称", action: () => void copyName(p.name) },
        { label: "复制 base64", action: () => void copyBase64(assetId) },
        { label: "复制提示词", disabled: true },
      ],
    },
    "divider",
    { label: "置顶", icon: "window-pin-on", disabled: true },
    "divider",
  );
  // 删除组：链接 / 非链接两套（稿上下文规则）；「从文件夹中移除」业务归属未确认，占位（09 待办）
  if (isLink) {
    entries.push({ label: "从文件夹中移除文件链接", icon: "remove-from-folder", shortcut: "Shift+Delete", disabled: true });
  } else {
    entries.push(
      { label: "从文件夹中移除", icon: "remove-from-folder", shortcut: "Shift+Delete", disabled: true },
      {
        label: "丢到回收站",
        icon: "drop-to-trash",
        shortcut: "Delete",
        action: () => void deletePlacements(selection.selectedRefs()),
      },
    );
  }
  return entries;
}

// —— 04 稿 · 档案卡片（宽 320 + 搜索框）——
// 档案卡片形态未落地（1.2/1.4 排期，FolderNode 尚无档案标记）→ 本 spec 写好暂不挂载，挂载记 09 待办。
// 「更改封面图」稿漏画二级箭头，按 10 稿补（二级两项，占位）。
export function archiveCardSpec(archive: { id: string; name: string }): CtxEntry[] {
  const content = useContentStore();
  const selection = useSelectionStore();
  const parentId = findParentId(content.folders, archive.id) ?? null;
  return [
    {
      label: "打开档案",
      icon: "archive",
      shortcut: "Enter",
      action: () => {
        useTabsStore().openView(`folder:${archive.id}`);
      },
    },
    {
      label: "新标签页打开",
      icon: "open-in-new-tab",
      shortcut: "Ctrl+Enter",
      action: () => {
        useTabsStore().openView(`folder:${archive.id}`);
      },
    },
    { label: "新建档案", icon: "new-archive", shortcut: "Shift+N", disabled: true },
    { label: "解散档案为文件夹", icon: "folder-open", disabled: true },
    { label: "新建文件夹", icon: "new-folder", shortcut: "Ctrl+Shift+N", action: () => void createAndName(parentId, archive.id) },
    {
      label: "新建子文件夹",
      icon: "new-subfolder",
      shortcut: "Ctrl+Shift+Alt+N",
      action: () => void createAndName(archive.id, null),
    },
    "divider",
    { label: "移动档案", icon: "move-file", shortcut: "Ctrl+Y", disabled: true },
    { label: "档案链接管理", icon: "linked-file-manage", shortcut: "Ctrl+J", disabled: true },
    { label: "添加文件/档案链接", icon: "add-file-link", shortcut: "Ctrl+L", action: () => pickFolderForLink() },
    { label: "打开档案链接所在位置", icon: "reveal-archive-link", submenu: [{ label: "（无）", disabled: true }] },
    "divider",
    { label: "剪切档案", icon: "cut-file", shortcut: "Ctrl+X", disabled: true },
    { label: "复制档案", icon: "copy-file", shortcut: "Ctrl+C", disabled: true },
    { label: "作为链接粘贴", icon: "paste-as-link", shortcut: "Ctrl+V", disabled: true },
    { label: "作为链接粘贴并重命名", icon: "paste-as-link-rename", shortcut: "Ctrl+Shift+V", disabled: true },
    { label: "作为副本粘贴", icon: "paste-as-copy", shortcut: "Ctrl+Alt+V", disabled: true },
    {
      label: "重命名",
      icon: "rename",
      shortcut: "Ctrl+R",
      action: () => {
        selection.namingFolderCardId = archive.id;
      },
    },
    "divider",
    { label: "克隆", icon: "clone", shortcut: "Ctrl+D", disabled: true },
    "divider",
    {
      label: "更改封面图",
      icon: "edit-cover",
      submenu: [
        { label: "从本地磁盘中选择", disabled: true },
        { label: "从资源库中选择", disabled: true },
      ],
    },
    { label: "更改颜色", icon: "edit-color", disabled: true, submenu: [] },
    "divider",
    { label: "添加至其他资源库", icon: "add-to-library", submenuWidth: 180, submenu: [{ label: "（暂无其他资源库）", disabled: true }] },
    {
      label: "导出文件",
      icon: "export-file",
      submenu: [
        { label: "导出为素材包", icon: "import-pack", disabled: true },
        { label: "导出到本地磁盘", icon: "export-to-disk", disabled: true },
        { label: "导出提示词为 .MD 文档", icon: "open-markdown-editor", disabled: true },
      ],
    },
    "divider",
    { label: "打开提示词编辑器", icon: "open-prompt-editor", shortcut: "Ctrl+T", disabled: true },
    {
      label: "复制数据",
      icon: "copy-data",
      submenu: [
        { label: "复制链接", action: () => void copyFolderLink(archive.id) },
        { label: "复制名称", action: () => void copyName(archive.name) },
      ],
    },
    "divider",
    { label: "置顶", icon: "window-pin-on", disabled: true },
    "divider",
    { label: "从文件夹中移除档案链接", icon: "remove-from-folder", shortcut: "Shift+Delete", disabled: true },
    {
      label: "丢到回收站",
      icon: "drop-to-trash",
      shortcut: "Delete",
      action: () => deleteFolderFlow(archive.id, archive.name),
    },
  ];
}

// —— 05 稿 · 文件夹（卡片与树节点共用；差异仅在重命名入口）——
// 排序依据/展开收起三项沿用 treeSort / treeExpansion 业务；更改图标/颜色网格二级面板随第二轮或 3.3 落地（一级占位）。
function folderMenuEntries(folder: { id: string; name: string }, parentId: string | null, rename: () => void): CtxEntry[] {
  const content = useContentStore();
  const ui = useUiStore();
  const node = findNode(content.folders, folder.id);
  const hasChildren = (node?.children.length ?? 0) > 0;
  const sortItem = (label: string, mode: typeof ui.treeSort): CtxItem => ({
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
      label: "打开文件夹",
      icon: "folder-open",
      shortcut: "Enter",
      action: () => {
        useTabsStore().openView(`folder:${folder.id}`);
      },
    },
    {
      label: "新标签页打开",
      icon: "open-in-new-tab",
      shortcut: "Ctrl+Enter",
      action: () => {
        useTabsStore().openView(`folder:${folder.id}`);
      },
    },
    { label: "新建文件夹", icon: "new-folder", shortcut: "Ctrl+Shift+N", action: () => void createAndName(parentId, folder.id) },
    {
      label: "新建子文件夹",
      icon: "new-subfolder",
      shortcut: "Ctrl+Shift+Alt+N",
      action: () => void createAndName(folder.id, null),
    },
    { label: "新建档案", icon: "new-archive", shortcut: "Shift+N", disabled: true },
    { label: "转换为档案", icon: "archive", shortcut: "菜单+N", disabled: true },
    "divider",
    { label: "移动文件夹", icon: "move-file", shortcut: "Ctrl+Y", disabled: true },
    { label: "添加文件/档案链接", icon: "add-file-link", shortcut: "Ctrl+L", action: () => pickFolderForLink() },
    "divider",
    { label: "剪切文件夹", icon: "cut-file", shortcut: "Ctrl+X", disabled: true },
    { label: "粘贴文件夹", icon: "paste-folder", shortcut: "Ctrl+V", disabled: true },
    { label: "重命名", icon: "rename", shortcut: "Ctrl+R", action: rename },
    "divider",
    { label: "克隆", icon: "clone", shortcut: "Ctrl+D", disabled: true },
    "divider",
    { label: "复制链接", icon: "copy-link", action: () => void copyFolderLink(folder.id) },
    "divider",
    {
      label: "排序依据",
      icon: "sort-by",
      submenu: [
        sortItem("当前层级 - 标题 A → Z", "levelAsc"),
        sortItem("当前层级 - 标题 Z → A", "levelDesc"),
        sortItem("全部 - 标题 A → Z", "allAsc"),
        sortItem("全部 - 标题 Z → A", "allDesc"),
      ],
    },
    {
      label: "展开/收起文件夹",
      icon: "expand-folder",
      disabled: !hasChildren,
      action: () => toggleOpen(folder.id),
    },
    {
      label: "展开/收起同阶层文件夹",
      icon: "expand-siblings",
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
      label: "展开/收起所有文件夹",
      icon: "expand-all",
      shortcut: "/",
      action: () => {
        if (treeExpansion.openSet.size > 0) collapseAll();
        else expandAll(content.folders);
      },
    },
    "divider",
    { label: "更改图标", icon: "edit-icon", disabled: true, submenu: [] },
    { label: "更改颜色", icon: "edit-color", disabled: true, submenu: [] },
    "divider",
    { label: "添加至其他资源库", icon: "add-to-library", submenuWidth: 180, submenu: [{ label: "（暂无其他资源库）", disabled: true }] },
    {
      label: "导出文件",
      icon: "export-file",
      submenu: [
        { label: "导出为素材包", icon: "import-pack", disabled: true },
        { label: "导出到本地磁盘", icon: "export-to-disk", action: () => void pickExportFolder(folder.id) },
      ],
    },
    "divider",
    { label: "删除文件夹", icon: "trash", shortcut: "Delete", action: () => deleteFolderFlow(folder.id, folder.name) },
  ];
}

// —— 文件夹卡片（FolderStrip）——
export function folderCardSpec(folder: { id: string; name: string }): CtxEntry[] {
  const selection = useSelectionStore();
  const parentId = findParentId(useContentStore().folders, folder.id) ?? null;
  return folderMenuEntries(folder, parentId, () => {
    selection.namingFolderCardId = folder.id;
  });
}

// —— 文件夹卡片空白处 ——
export function folderBlankSpec(): CtxEntry[] {
  const content = useContentStore();
  const current = content.sidebarSelected.startsWith("folder:") ? content.sidebarSelected.slice(7) : null;
  return [{ label: "新建文件夹", icon: "new-folder", action: () => void createAndName(current, null) }];
}

// —— 文件树（复用 05 文件夹稿；档案节点随档案业务落地后分流 04 稿，记 09 待办）——
export function treeItemSpec(node: FolderNode, parentId: string | null): CtxEntry[] {
  const selection = useSelectionStore();
  return folderMenuEntries(node, parentId, () => {
    selection.namingFolderId = node.id;
  });
}

// —— 素材卡片空白处（新稿无对应文档，保留 6.3 版；是否保留记 09 待办）——
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

// —— 07 稿 · 新增按钮（SidebarLeft +，宽 320 无搜索框；导入动作由调用方注入）——
// 新建文档组稿只有 prose 无 CSS（图标需系统检测），按通用 item 样式补、全占位；
// Illustrator / Word / Excel / 文本文档 无图标，留空记 09 待办。
export function addButtonSpec(pickImport: (kind: "files" | "folder") => void): CtxEntry[] {
  const selection = useSelectionStore();
  const hasClipboard = selection.clipboard !== null;
  return [
    { label: "作为链接粘贴", icon: "paste-as-link", shortcut: "Ctrl+V", disabled: !hasClipboard, action: () => void pasteClipboard() },
    { label: "作为链接粘贴并重命名", icon: "paste-as-link-rename", shortcut: "Ctrl+Shift+V", disabled: true },
    { label: "作为副本粘贴", icon: "paste-as-copy", shortcut: "Ctrl+Alt+V", disabled: !hasClipboard, action: () => void pasteDuplicate() },
    "divider",
    { label: "新建文件夹", icon: "new-folder", shortcut: "Ctrl+Shift+N", action: () => void createAndName(null, null) },
    { label: "新建档案", icon: "new-archive", shortcut: "Shift+N", disabled: true },
    "divider",
    { label: "导入本地文件", icon: "import-file", action: () => pickImport("files") },
    { label: "导入本地文件夹", icon: "import-folder", action: () => pickImport("folder") },
    { label: "导入素材包", icon: "import-pack", disabled: true },
    { label: "档案链接管理", icon: "linked-file-manage", shortcut: "Ctrl+J", disabled: true },
    { label: "添加文件/档案链接", icon: "add-file-link", shortcut: "Ctrl+L", action: () => pickFolderForLink() },
    "divider",
    {
      label: "扫描重复文件",
      icon: "scan-duplicates",
      submenu: [
        { label: "当前目录", disabled: true },
        { label: "全部", disabled: true },
        { label: "所选文件", disabled: true },
      ],
    },
    { label: "扫描文件/档案链接", icon: "scan-links", disabled: true },
    "divider",
    { label: "Animate 文档", icon: "animate-on", disabled: true },
    { label: "Photoshop 文档", icon: "photoshop-on", disabled: true },
    { label: "After Effects 文档", icon: "ae-on", disabled: true },
    { label: "Illustrator 文档", disabled: true },
    "divider",
    { label: "MarkDown 文档", icon: "open-markdown-editor", disabled: true },
    { label: "文本文档", disabled: true },
    { label: "Word 文档", disabled: true },
    { label: "Excel 文档", disabled: true },
  ];
}

// —— 网格右键统一调度（6.3，自 AssetGrid 拆出，规范第 3 条行数控制）——
// 素材卡片 = 素材菜单（回收站视口返回 "recycle" 由调用方出 RecycleCardMenu）；
// grid-header 空白 = 文件夹卡片空白菜单；其余空白 = 素材空白菜单；
// 文件夹卡片 / 按钮 / 输入框上不处理（"pass"——文件夹卡片自带菜单）。
// 1.3：素材 / 文件夹卡片菜单按稿出 320 宽 + 搜索框。
export function openGridContextMenu(e: MouseEvent, items: PlacementView[]): "recycle" | "done" | "pass" {
  const t = e.target as HTMLElement;
  const selection = useSelectionStore();
  const content = useContentStore();
  const card = t.closest(".asset-card, .list-row");
  const key = card?.getAttribute("data-key");
  if (key) {
    e.preventDefault();
    if (!selection.isSelected(key)) selection.selectOnly(key);
    if (content.isRecycleView) return "recycle";
    const p = items.find((it) => `${it.assetId}|${it.folderId}` === key);
    if (p) openContextMenu(e, assetCardSpec(p), undefined, { width: 320, searchable: true });
    return "done";
  }
  if (t.closest(".folder-card, button, input")) return "pass";
  e.preventDefault();
  openContextMenu(e, t.closest(".grid-header") ? folderBlankSpec() : assetBlankSpec());
  return "done";
}

// 文件夹卡片右键（FolderStrip 调用）：05 稿 320 宽 + 搜索框
export function openFolderCardMenu(e: MouseEvent, folder: { id: string; name: string }): void {
  openContextMenu(e, folderCardSpec(folder), undefined, { width: 320, searchable: true });
}
