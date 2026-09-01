// 04 稿 · 档案卡片右键菜单 spec（自 menuSpecs.ts 拆出，规范第 3 条行数控制）：菜单项按新稿纯文字口径（2026-09-01）。
import { useContentStore, type FolderNode } from "../../stores/content";
import { usePickerStore } from "../../stores/picker";
import { useSelectionStore } from "../../stores/selection";
import { useTabsStore } from "../../stores/tabs";
import type { CtxEntry, CtxItem } from "../contextMenus";
import { menuState } from "../contextMenus";
import { FOLDER_SWATCHES, toNewTint } from "../customColors";
import {
  copyFolderLink,
  copyName,
  createAndName,
  createArchiveAndName,
  cutFoldersFlow,
  deleteFolderFlow,
  dissolveArchiveFlow,
  enterLinkPicker,
  findParentId,
  moveFoldersFlow,
  topLevelBlockId,
} from "../menuOps";
import { setFoldersColor } from "../placementOps";

// 档案节点的 FolderNode 查找（更改颜色当前色回显）：档案 = Folder kind 标记，色值在同一张 folders 表
function findNode(nodes: FolderNode[], id: string): FolderNode | null {
  for (const n of nodes) {
    if (n.id === id) return n;
    const hit = findNode(n.children, id);
    if (hit) return hit;
  }
  return null;
}

// —— 04 稿 · 档案卡片 ——
// 2026-08-15 一轮挂载：档案卡片（ArchiveStrip）右键 + 树档案节点（folder.ts treeItemSpec 分流）共用；
// rename（树节点分流用）：树档案节点的重命名入口走树命名态 namingFolderId，
// 缺省 = 卡片命名态 namingFolderCardId。
// 「更改封面图」稿漏画二级箭头，按 10 稿补（二级两项，占位）。
export function archiveCardSpec(archive: { id: string; name: string }, rename?: () => void): CtxEntry[] {
  // 选择模式（6.2 一轮，PRD 03_数据链接 04）：右键仅留查看类操作，编辑类不出现
  if (usePickerStore().active) {
    return [
      {
        label: "打开档案",
        action: () => {
          useTabsStore().openView(`folder:${archive.id}`);
        },
      },
      {
        label: "新标签页打开",
        action: () => {
          useTabsStore().openInNewTab(`folder:${archive.id}`);
        },
      },
      "divider",
      {
        label: "复制数据",
        submenu: [
          { label: "复制链接", action: () => void copyFolderLink(archive.id) },
          { label: "复制名称", action: () => void copyName(archive.name) },
        ],
      },
    ];
  }
  const content = useContentStore();
  const selection = useSelectionStore();
  const parentId = findParentId(content.folders, archive.id) ?? null;
  return [
    {
      label: "打开档案",
      shortcut: "Enter",
      action: () => {
        useTabsStore().openView(`folder:${archive.id}`);
      },
    },
    {
      label: "新标签页打开",
      shortcut: "Ctrl+Enter",
      // 恒新开标签页（2026-08-20 修复：原误用 openView——未锁定页只替换视口，没开新页）
      action: () => {
        useTabsStore().openInNewTab(`folder:${archive.id}`);
      },
    },
    {
      label: "新建档案",
      shortcut: "Shift+N",
      // 同级新建档案（3.2 一轮点亮，同新建文件夹口径）：目标为顶级时跟随其所在区块
      action: () =>
        void createArchiveAndName(parentId, archive.id, parentId === null ? topLevelBlockId(content.folders, archive.id) : null),
    },
    {
      label: "解散档案为文件夹",
      // 摘 kind 标记 + 清档案数据记录（3.2 一轮点亮，后端幂等）
      action: () => dissolveArchiveFlow(archive.id),
    },
    {
      label: "新建文件夹",
      shortcut: "Ctrl+Shift+N",
      // 同级新建（2026-08-14 区块子系统）：目标为顶级时跟随其所在区块，否则落错「目录」
      action: () =>
        void createAndName(parentId, archive.id, parentId === null ? topLevelBlockId(content.folders, archive.id) : null),
    },
    {
      label: "新建子文件夹",
      shortcut: "Ctrl+Shift+Alt+N",
      action: () => void createAndName(archive.id, null),
    },
    "divider",
    { label: "移动档案", shortcut: "Ctrl+Y", action: () => moveFoldersFlow(archive.id) },
    // 档案链接管理：随档案多位置立项（2026-08-24 用户定——角色档案整体链接到其他项目，数据模型级改动）
    { label: "档案链接管理", shortcut: "Ctrl+J", disabled: true },
    { label: "添加文件/档案链接", shortcut: "Ctrl+L", action: () => enterLinkPicker({ id: archive.id, name: archive.name }) },
    { label: "打开档案链接所在位置", submenu: [{ label: "（无）", disabled: true }] },
    "divider",
    { label: "剪切档案", shortcut: "Ctrl+X", action: () => cutFoldersFlow(archive.id) },
    { label: "复制档案", shortcut: "Ctrl+C", disabled: true },
    { label: "作为链接粘贴", shortcut: "Ctrl+V", disabled: true },
    { label: "作为副本粘贴", shortcut: "Ctrl+Shift+V", disabled: true },
    {
      label: "重命名",
      shortcut: "F2",
      action: () => {
        if (rename) rename(); // 树档案节点：树命名态（folder.ts treeItemSpec 分流传入）
        else selection.namingFolderCardId = archive.id;
      },
    },
    "divider",
    { label: "更改封面图",
      submenu: [
        { label: "从本地磁盘中选择", disabled: true },
        { label: "从资源库中选择", disabled: true },
      ],
    },
    // 更改颜色（2026-09-01 接入，同树文件夹口径）：内嵌色块行点色即生效不关菜单 + 多选域内批量；
    // 生效处 = 树档案节点圆点/图标 tint（Folder.color；档案徽标色是 archives 表另一套，不动）
    {
      label: "更改颜色",
      swatches: {
        colors: FOLDER_SWATCHES,
        current: toNewTint(findNode(content.folders, archive.id)?.color ?? undefined) || null,
        onPick: (tint) => {
          const ids =
            selection.selectedFolders.size > 1 && selection.selectedFolders.has(archive.id)
              ? selection.orderedSelectedFolderIds()
              : [archive.id];
          void setFoldersColor(ids, tint);
          const e = menuState.entries?.find((x): x is CtxItem => x !== "divider" && x.label === "更改颜色");
          if (e?.swatches) e.swatches.current = tint;
        },
      },
    },
    "divider",
    { label: "添加至其他资源库", submenu: [{ label: "（暂无其他资源库）", disabled: true }] },
    {
      label: "导出文件",
      submenu: [
        { label: "导出为素材包", disabled: true },
        { label: "导出到本地磁盘", disabled: true },
        { label: "导出提示词为 .MD 文档", disabled: true },
      ],
    },
    "divider",
    { label: "打开提示词编辑器", shortcut: "Ctrl+T", disabled: true },
    {
      label: "复制数据",
      submenu: [
        { label: "复制链接", action: () => void copyFolderLink(archive.id) },
        { label: "复制名称", action: () => void copyName(archive.name) },
      ],
    },
    "divider",
    { label: "置顶", disabled: true },
    "divider",
    { label: "从文件夹中移除档案链接", shortcut: "Shift+Delete", disabled: true },
    {
      label: "丢到回收站",
      shortcut: "Delete",
      action: () => deleteFolderFlow(archive.id, archive.name),
    },
  ];
}
