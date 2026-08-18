// 05 稿 · 文件夹右键菜单 spec（自 menuSpecs.ts 拆出，规范第 3 条行数控制）：
// 卡片与树节点共用主体（folderMenuEntries），差异 = 重命名入口 + 树专属段；05 稿一体不可拆。
import { useContentStore, type FolderNode } from "../../stores/content";
import { useSelectionStore } from "../../stores/selection";
import { useTabsStore } from "../../stores/tabs";
import { useUiStore } from "../../stores/ui";
import type { CtxEntry, CtxItem } from "../contextMenus";
import { menuState } from "../contextMenus";
import {
  convertToArchiveFlow,
  copyFolderLink,
  createAndName,
  createArchiveAndName,
  cutFoldersFlow,
  deleteFolderFlow,
  findParentId,
  moveFoldersFlow,
  pickExportFolder,
  pickFolderForLink,
  topLevelBlockId,
} from "../menuOps";
import { pasteFoldersInto } from "../placementOps";
import { archiveCardSpec } from "./archiveCard";
import { collapseAll, collapseLevel, expandAll, expandLevel, toggleOpen, treeExpansion } from "../treeExpansion";

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

// —— 05 稿 · 文件夹（卡片与树节点共用主体；差异：重命名入口 + 树专属段）——
// 排序依据带当前模式对勾 + "自定义"首项（2026-08-14 用户定，显示简介下拉同款；原"无对勾列防截断"
// 口径推翻，二级宽 184 → 226 容纳对勾列 + 长标签；keepOpen 期间经 menuState reactive 回写 checked）；
// 展开收起三项沿用 treeExpansion 业务；
// 排序依据/展开收起/更改图标/更改颜色 = 树节点专属（文件夹卡片不支持，用户定）；
// 更改图标/颜色 = 网格面板形态（2026-08-18，3.3 点亮：CtxItem.panel + ContextMenu flyout 分流渲染）。
function folderMenuEntries(folder: { id: string; name: string }, parentId: string | null, rename: () => void, treeOps: boolean): CtxEntry[] {
  const content = useContentStore();
  const selection = useSelectionStore();
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
    {
      label: "新建文件夹",
      icon: "new-folder",
      shortcut: "Ctrl+Shift+N",
      // 同级新建（2026-08-14 区块子系统）：目标为顶级文件夹时带上其所在区块，否则落错「目录」
      action: () =>
        void createAndName(parentId, folder.id, parentId === null ? topLevelBlockId(content.folders, folder.id) : null),
    },
    {
      label: "新建子文件夹",
      icon: "new-subfolder",
      shortcut: "Ctrl+Shift+Alt+N",
      action: () => void createAndName(folder.id, null),
    },
    {
      label: "新建档案",
      icon: "new-archive",
      shortcut: "Shift+N",
      // 同级新建档案（3.2 一轮，同新建文件夹口径）：目标为顶级时跟随其所在区块
      action: () =>
        void createArchiveAndName(parentId, folder.id, parentId === null ? topLevelBlockId(content.folders, folder.id) : null),
    },
    {
      label: "转换为档案",
      icon: "archive",
      shortcut: "菜单+N",
      // 两段式（3.2 一轮）：有子文件夹先弹警告确认，确认后剔除层级转换
      action: () => convertToArchiveFlow(folder.id),
    },
    "divider",
    { label: "移动文件夹", icon: "move-file", shortcut: "Ctrl+Y", action: () => moveFoldersFlow(folder.id) },
    { label: "添加文件/档案链接", icon: "add-file-link", shortcut: "Ctrl+L", action: () => pickFolderForLink() },
    "divider",
    // 剪切文件夹（3.4 点亮）：触发项在多选域内 → 剪切整个选中集（cutFoldersFlow 分流）
    { label: "剪切文件夹", icon: "cut-file", shortcut: "Ctrl+X", action: () => cutFoldersFlow(folder.id) },
    // 粘贴文件夹（3.4 点亮）：落点 = 被右键的本文件夹；剪贴板无文件夹时禁用
    {
      label: "粘贴文件夹",
      icon: "paste-folder",
      shortcut: "Ctrl+V",
      disabled: !(selection.clipboard?.folders?.length),
      action: () => void pasteFoldersInto(folder.id),
    },
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
    // 排序依据（2026-08-14 用户定：加"自定义"首项 + 当前模式对勾，显示简介下拉同款）——
    // 对勾口径：自定义 = treeSort custom；level* 仅锚定层级命中；all* 全局。
    // keepOpen 期间 spec 静态快照不刷新——action 里经 menuState（reactive）回写 checked
    const sortModes = [
      ["自定义", "custom"],
      ["当前层级 - 标题 A → Z", "levelAsc"],
      ["当前层级 - 标题 Z → A", "levelDesc"],
      ["全部 - 标题 A → Z", "allAsc"],
      ["全部 - 标题 Z → A", "allDesc"],
    ] as const;
    const isCurrentSort = (mode: (typeof sortModes)[number][1]): boolean =>
      mode === "custom"
        ? ui.treeSort === "custom"
        : ui.treeSort === mode && (mode.startsWith("all") || ui.treeSortAnchor === parentId);
    const syncSortChecks = (): void => {
      const entry = menuState.entries?.find((e): e is CtxItem => e !== "divider" && e.label === "排序依据");
      entry?.submenu?.forEach((it, i) => {
        if (it !== "divider") it.checked = isCurrentSort(sortModes[i][1]); // 二级无分隔线，索引对齐
      });
    };
    const sortItem = (label: string, mode: (typeof sortModes)[number][1]): CtxItem => ({
      label,
      keepOpen: true, // 状态切换类：用户可能连续试几种排序（2026-08-12 判定口径）
      checkable: true,
      checked: isCurrentSort(mode),
      action: () => {
        ui.treeSort = mode;
        ui.treeSortAnchor = mode === "custom" ? null : parentId;
        syncSortChecks();
      },
    });
    entries.push(
      "divider",
      {
        label: "排序依据",
        icon: "sort-by",
        // 对勾列 18px 加入后 184 稿宽会截断（原"无对勾列"口径 2026-08-14 被用户推翻）→ 226
        //（202 实测"当前层级 - 标题 A → Z"仍截断，探针 S7 走查补到 226）
        submenuWidth: 226,
        submenu: sortModes.map(([label, mode]) => sortItem(label, mode)),
      },
      {
        label: "展开/收起文件夹",
        icon: "expand-folder",
        disabled: !hasChildren,
        keepOpen: true, // 状态切换类
        action: () => toggleOpen(folder.id),
      },
      {
        label: "展开/收起同阶层文件夹",
        icon: "expand-siblings",
        keepOpen: true, // 状态切换类
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
        keepOpen: true, // 状态切换类
        action: () => {
          if (treeExpansion.openSet.size > 0) collapseAll();
          else expandAll(content.folders);
        },
      },
      "divider",
      // 更改图标/颜色网格面板（2026-08-18，3.3 点亮）：05 稿 §3=230（20 格）/ §4=235（10 格）；
      // 同步语义（用户定）：默认影响子级——链路一致的后代跟随，不一致的不动；
      // panel 形态由 ContextMenu flyout 渲染对应组件
      { label: "更改图标", icon: "edit-icon", submenuWidth: 230, panel: { kind: "folderIcon", folderId: folder.id } },
      { label: "更改颜色", icon: "edit-color", submenuWidth: 235, panel: { kind: "folderColor", folderId: folder.id } },
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

// —— 文件树（05 稿全量含排序/展开/图标/颜色；档案节点分流 04 稿档案菜单，2026-08-15 一轮）——
export function treeItemSpec(node: FolderNode, parentId: string | null): CtxEntry[] {
  const selection = useSelectionStore();
  // 档案节点：04 稿档案菜单；重命名入口换树命名态（卡片命名态 namingFolderCardId 在树上无效）
  if (node.kind === "archive") {
    return archiveCardSpec(node, () => {
      selection.namingFolderId = node.id;
    });
  }
  return folderMenuEntries(node, parentId, () => {
    selection.namingFolderId = node.id;
  }, true);
}
