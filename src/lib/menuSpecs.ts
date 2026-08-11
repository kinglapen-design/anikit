// 上下文菜单 spec（1.3 一轮 2026-08-11 按新稿 `02_布局` 03/04/05/07 重写；旧 6.3 版基于 `05_右键菜单` 废止）：
// 数据驱动（lib/contextMenus 渲染），设计项全列 + 图标按稿配（稿里无图标的项不画蛇添足，2026-08-11 走查）——
// 未实现的一律 disabled 置灰占位（占位点亮机制）。分组用 "divider"（组标题不出文字，Eagle 风格）。
// 03/04/05 稿 = 宽 320 + 搜索框（openGridContextMenu / FolderStrip 调用处传 opts）；07 稿 = 宽 320 无搜索框。
// 二级菜单宽度按稿（submenuWidth）：链接位置 320 / 缩略图·导出 213 / 资源库·AI·复制数据 180 /
// 排序依据 184 / 扫描重复 96 / 更改封面 152（2026-08-11 二轮走查，原默认 175 文字截断）。
// 菜单快捷键标注按新稿（重命名 ctrl+R 等，显示小写化在 MenuItem 统一做）；
// 全局快捷键绑定另有一套，统一对齐记 09 待办。
import { invoke } from "@tauri-apps/api/core";
import { useContentStore, type FolderNode, type PlacementView } from "../stores/content";
import { useSelectionStore } from "../stores/selection";
import { useTabsStore } from "../stores/tabs";
import { useUiStore } from "../stores/ui";
import { usePreviewStore } from "../stores/preview";
import { openContextMenu, type CtxEntry, type CtxItem } from "./contextMenus";
import { docApp } from "./docApps";
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
  pickImportPaths,
  revealWithWarning,
} from "./menuOps";
import {
  applyClipboardThumb,
  clearRecycle,
  copySelection,
  createTextDocument,
  deletePlacements,
  duplicatePlacements,
  openWithDialog,
  pasteClipboard,
  pasteDuplicate,
  recycleSelectedAssetIds,
  refreshThumb,
  restoreRecycle,
} from "./placementOps";
import { collapseAll, collapseLevel, expandAll, expandLevel, toggleOpen, treeExpansion } from "./treeExpansion";
// MarkDown 文档图标（2026-08-11 用户供 md.webp，webp 非 svg 走 iconData 内联图）
import mdDocIcon from "../../assets/icons/01_icon-default/md.webp?url";

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
      submenuWidth: 320,
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
      submenuWidth: 213,
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
      submenuWidth: 213,
      submenu: [
        { label: "导出为素材包", icon: "library", disabled: true },
        {
          label: "导出到本地磁盘",
          icon: "export-to-disk",
          action: () => void pickExportAssets(selection.selectedRefs().map((r) => r.assetId)),
        },
        { label: "导出提示词为 .MD 文档", icon: "set-markdown-as-prompt", disabled: true },
      ],
    },
    "divider",
    {
      label: "AI 工具集",
      icon: "ai-tools",
      submenuWidth: 180,
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
      submenuWidth: 180,
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
// 「更改封面图」稿漏画二级箭头，按 10 稿补（二级两项，宽 152，占位）；
// 稿未画的二级（链接位置/资源库/导出/复制数据）宽度沿用 03 同项口径。
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
    { label: "打开档案链接所在位置", icon: "reveal-archive-link", submenuWidth: 320, submenu: [{ label: "（无）", disabled: true }] },
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
      submenuWidth: 152,
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
      submenuWidth: 213,
      submenu: [
        { label: "导出为素材包", icon: "library", disabled: true },
        { label: "导出到本地磁盘", icon: "export-to-disk", disabled: true },
        { label: "导出提示词为 .MD 文档", icon: "set-markdown-as-prompt", disabled: true },
      ],
    },
    "divider",
    { label: "打开提示词编辑器", icon: "open-prompt-editor", shortcut: "Ctrl+T", disabled: true },
    {
      label: "复制数据",
      icon: "copy-data",
      submenuWidth: 180,
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

// —— 05 稿 · 文件夹（卡片与树节点共用主体；差异：重命名入口 + 树专属段）——
// 排序依据项按稿无对勾列（checkable 的 18px 占位会把 184 宽稿值挤到截断，2026-08-11 三轮走查）；
// 展开收起三项沿用 treeExpansion 业务；
// 排序依据/展开收起/更改图标/更改颜色 = 树节点专属（文件夹卡片不支持，用户定）；
// 更改图标/颜色网格二级面板随第二轮或 3.3 落地（一级占位）。
function folderMenuEntries(folder: { id: string; name: string }, parentId: string | null, rename: () => void, treeOps: boolean): CtxEntry[] {
  const content = useContentStore();
  const entries: CtxEntry[] = [
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
  ];
  if (treeOps) {
    const ui = useUiStore();
    const node = findNode(content.folders, folder.id);
    const hasChildren = (node?.children.length ?? 0) > 0;
    const sortItem = (label: string, mode: typeof ui.treeSort): CtxItem => ({
      label,
      action: () => {
        ui.treeSort = mode;
        ui.treeSortAnchor = parentId;
      },
    });
    entries.push(
      "divider",
      {
        label: "排序依据",
        icon: "sort-by",
        submenuWidth: 184,
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
    );
  }
  entries.push(
    "divider",
    { label: "添加至其他资源库", icon: "add-to-library", submenuWidth: 180, submenu: [{ label: "（暂无其他资源库）", disabled: true }] },
    {
      label: "导出文件",
      icon: "export-file",
      submenuWidth: 213,
      submenu: [
        { label: "导出为素材包", icon: "library", disabled: true },
        { label: "导出到本地磁盘", icon: "export-to-disk", action: () => void pickExportFolder(folder.id) },
      ],
    },
    "divider",
    { label: "删除文件夹", icon: "trash", shortcut: "Delete", action: () => deleteFolderFlow(folder.id, folder.name) },
  );
  return entries;
}

// —— 文件夹卡片（FolderStrip；无排序/展开/图标/颜色——卡片不支持，用户 2026-08-11 定）——
export function folderCardSpec(folder: { id: string; name: string }): CtxEntry[] {
  const selection = useSelectionStore();
  const parentId = findParentId(useContentStore().folders, folder.id) ?? null;
  return folderMenuEntries(folder, parentId, () => {
    selection.namingFolderCardId = folder.id;
  }, false);
}

// —— 文件夹卡片空白处（废除，2026-08-11：空白右键统一走 07 新增菜单）——

// —— 文件树（05 稿全量含排序/展开/图标/颜色；档案节点随档案业务落地后分流 04 稿，记 09 待办）——
export function treeItemSpec(node: FolderNode, parentId: string | null): CtxEntry[] {
  const selection = useSelectionStore();
  return folderMenuEntries(node, parentId, () => {
    selection.namingFolderId = node.id;
  }, true);
}

// —— 素材卡片空白处（废除，2026-08-11：空白右键统一走 07 新增菜单；
// 布局方式/排序/显示简介/分组依据归 28 查看方式菜单 ViewOptionsMenu）——

// —— 07 稿 · 新增菜单（宽 320 无搜索框）——
// 三入口共用一套（2026-08-11 用户定）：侧栏"新增"按钮 / 网格空白处右键 / Ctrl+N（按钮位弹出）。
// 上下文规则：剪贴板无文件 → 三个粘贴项不出现（同 03 稿口径，2026-08-11 用户定）；
// 新建文档组 = 系统软件检测动态显隐（lib/docApps，每次打开 UI 监测）：已安装 → 带系统图标
//（exe 提取，iconData）显示，未安装 → 删除对应新建项；MarkDown 文档恒显示（纯文本无需软件）；
// 新建文档业务阶段三 3.1 接，本轮项仍 disabled 占位。
export function addButtonSpec(): CtxEntry[] {
  const selection = useSelectionStore();
  const hasClipboard = selection.clipboard !== null;
  const entries: CtxEntry[] = [];
  if (hasClipboard) {
    entries.push(
      { label: "作为链接粘贴", icon: "paste-as-link", shortcut: "Ctrl+V", action: () => void pasteClipboard() },
      { label: "作为链接粘贴并重命名", icon: "paste-as-link-rename", shortcut: "Ctrl+Shift+V", disabled: true },
      { label: "作为副本粘贴", icon: "paste-as-copy", shortcut: "Ctrl+Alt+V", action: () => void pasteDuplicate() },
      "divider",
    );
  }
  entries.push(
    { label: "新建文件夹", icon: "new-folder", shortcut: "Ctrl+Shift+N", action: () => void createAndName(null, null) },
    { label: "新建档案", icon: "new-archive", shortcut: "Shift+N", disabled: true },
    "divider",
    { label: "导入本地文件", icon: "import-file", action: () => void pickImportPaths("files") },
    { label: "导入本地文件夹", icon: "import-folder", action: () => void pickImportPaths("folder") },
    { label: "导入素材包", icon: "import-pack", disabled: true },
    { label: "档案链接管理", icon: "linked-file-manage", shortcut: "Ctrl+J", disabled: true },
    { label: "添加文件/档案链接", icon: "add-file-link", shortcut: "Ctrl+L", action: () => pickFolderForLink() },
    "divider",
    {
      label: "扫描重复文件",
      icon: "scan-duplicates",
      submenuWidth: 96,
      submenu: [
        { label: "当前目录", disabled: true },
        { label: "全部", disabled: true },
        { label: "所选文件", disabled: true },
      ],
    },
    { label: "扫描文件/档案链接", icon: "scan-links", disabled: true },
  );
  // 新建文档组：已安装才出（图标 = 系统 exe 提取）；软件组与办公/文本组之间分组线按稿；
  // MarkDown / 文本文档已点亮（纯文本新建入库，2026-08-11 用户定），其余新建业务 3.1 接
  const docItem = (id: string, label: string, action?: () => void): CtxItem | null => {
    const app = docApp(id);
    if (!app?.installed) return null;
    return { label, iconData: app.icon ?? undefined, disabled: !action, action };
  };
  const softwareDocs = [
    docItem("animate", "Animate 文档"),
    docItem("photoshop", "Photoshop 文档"),
    docItem("afterEffects", "After Effects 文档"),
    docItem("illustrator", "Illustrator 文档"),
  ].filter((it): it is CtxItem => it !== null);
  const officeDocs = [
    docItem("text", "文本文档", () => void createTextDocument("txt")),
    docItem("word", "Word 文档"),
    docItem("excel", "Excel 文档"),
  ].filter((it): it is CtxItem => it !== null);
  const docGroup: CtxEntry[] = [];
  if (softwareDocs.length > 0) docGroup.push(...softwareDocs, "divider");
  docGroup.push({ label: "MarkDown 文档", iconData: mdDocIcon, action: () => void createTextDocument("md") });
  if (officeDocs.length > 0) docGroup.push(...officeDocs);
  entries.push("divider", ...docGroup);
  return entries;
}

// —— 13 稿 · 回收站（宽 148）——
// 空白 = 还原全部文件 / 清空回收站（空时禁用）；item 右键首行加「还原文件」（稿注单项入口，
// 语义统一为"文件"不区分文档，2026-08-11 用户定）。
// 彻底删除不入菜单（按稿；Del 快捷键与两段式确认业务保留在 placementOps）。
export function recycleBlankSpec(): CtxEntry[] {
  const content = useContentStore();
  return [
    {
      label: "还原全部文件",
      icon: "restore-all",
      disabled: content.recycleEntries.length === 0,
      action: () => void restoreRecycle([...new Set(content.recycleEntries.map((e) => e.assetId))]),
    },
    {
      label: "清空回收站",
      icon: "empty-trash",
      disabled: content.recycleEntries.length === 0,
      action: () => void clearRecycle(),
    },
  ];
}

export function recycleCardSpec(): CtxEntry[] {
  return [
    { label: "还原文件", icon: "restore-all", action: () => void restoreRecycle(recycleSelectedAssetIds()) },
    "divider",
    ...recycleBlankSpec(),
  ];
}

// —— 网格右键统一调度（6.3，自 AssetGrid 拆出，规范第 3 条行数控制）——
// 素材卡片 = 素材菜单（回收站视口 = 13 稿回收站菜单，1.3 归并全局宿主，RecycleCardMenu 局部菜单废除）；
// 空白处（含 grid-header）= 07 新增菜单（2026-08-11 用户定：空白右键/新增按钮/Ctrl+N 三入口共用，
// 旧 6.3 空白菜单 assetBlankSpec/folderBlankSpec 废除——布局方式等归 28 查看方式菜单）；
// 回收站视口空白 = 13 稿空白菜单；文件夹卡片 / 按钮 / 输入框上不处理（"pass"——文件夹卡片自带菜单）。
// 1.3：素材 / 文件夹卡片菜单按稿出 320 宽 + 搜索框。
export function openGridContextMenu(e: MouseEvent, items: PlacementView[]): "done" | "pass" {
  const t = e.target as HTMLElement;
  const selection = useSelectionStore();
  const content = useContentStore();
  const card = t.closest(".asset-card, .list-row");
  const key = card?.getAttribute("data-key");
  if (key) {
    e.preventDefault();
    if (!selection.isSelected(key)) selection.selectOnly(key);
    // 回收站分流前置（2026-08-11 修复 item 右键调不出：键解析失败也不依赖 items 反查，
    // 与 6.1 旧代码同构——回收站菜单不读卡片数据）
    if (content.isRecycleView) {
      openContextMenu(e, recycleCardSpec(), undefined, { width: 148 });
      return "done";
    }
    const p = items.find((it) => `${it.assetId}|${it.folderId}` === key);
    if (p) openContextMenu(e, assetCardSpec(p), undefined, { width: 320, searchable: true });
    return "done";
  }
  if (t.closest(".folder-card, button, input")) return "pass";
  e.preventDefault();
  if (content.isRecycleView) openContextMenu(e, recycleBlankSpec(), undefined, { width: 148 });
  else openContextMenu(e, addButtonSpec(), undefined, { width: 320 });
  return "done";
}

// 文件夹卡片右键（FolderStrip 调用）：05 稿 320 宽 + 搜索框
export function openFolderCardMenu(e: MouseEvent, folder: { id: string; name: string }): void {
  openContextMenu(e, folderCardSpec(folder), undefined, { width: 320, searchable: true });
}
