// 04 稿 · 档案卡片右键菜单 spec（自 menuSpecs.ts 拆出，规范第 3 条行数控制）：菜单项按新稿纯文字口径（2026-09-01）。
// 2026-09-05 走查「上下文菜单」档案节修订：
// - 「作为链接粘贴」移除（只有复制档案后才出现——复制档案业务随档案多位置立项另轮，届时按条件接回）；
// - 「作为副本粘贴」删除（档案走与文件夹一样的克隆，新加「克隆」Ctrl+D = clone_folders，档案记录随迁）；
// - 「导出文件」文案修正为「导出档案」，二级删「导出提示词为 .MD 文档」（占位废除）；
// - 「置顶」删除；「从文件夹中移除档案链接」删除（档案不存在分类问题——档案本身在树容器目录里，
//   至少也在根目录；档案进回收站语义 = 链接/原始档案独立进站，后端另轮统一立项，本轮「丢到回收站」行为照旧）；
// - 同时多选档案+文件夹时删除组分两项（删除文件夹 / 将档案丢到回收站，splitSelectedFolderKinds 分流）；
// - 「提示词」一级菜单（三态切换 = menuSpecs/promptMenu 共享构建器，提示词轮接业务）；
// - 复制数据组加「复制标签/复制描述」，复制后多出「粘贴标签/粘贴描述」（合并追加语义，多选批量）。
import { useContentStore, type FolderNode } from "../../stores/content";
import { usePickerStore } from "../../stores/picker";
import { useSelectionStore } from "../../stores/selection";
import { useTabsStore } from "../../stores/tabs";
import type { CtxEntry, CtxItem } from "../contextMenus";
import { menuState } from "../contextMenus";
import { FOLDER_SWATCHES, toNewTint } from "../customColors";
import { archivePromptEntries } from "./promptMenu";
import {
  copyArchiveDescriptions,
  copyArchiveTags,
  pasteDescriptions,
  pasteTags,
} from "../customDataOps";
import {
  copyFolderLink,
  copyName,
  createAndName,
  createArchiveAndName,
  cutFoldersFlow,
  deleteFolderFlow,
  deleteFoldersFlow,
  dissolveArchiveFlow,
  cloneFolderFlow,
  enterLinkPicker,
  findParentId,
  moveFoldersFlow,
  splitSelectedFolderKinds,
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

// 粘贴标签/描述的目标集（多选批量）：触发项在文件夹多选域内 → 域内全部档案；否则仅自身
function selectedArchiveIds(triggerId: string): string[] {
  const selection = useSelectionStore();
  if (!(selection.selectedFolders.size > 1 && selection.selectedFolders.has(triggerId))) return [triggerId];
  return selection
    .orderedSelectedFolderIds()
    .filter((id) => findNode(useContentStore().folders, id)?.kind === "archive");
}

// —— 04 稿 · 档案卡片 ——
// 2026-08-15 一轮挂载：档案卡片（ArchiveStrip）右键 + 树档案节点（folder.ts treeItemSpec 分流）共用；
// rename（树节点分流用）：树档案节点的重命名入口走树命名态 namingFolderId，
// 缺省 = 卡片命名态 namingFolderCardId。
// 「更改封面图」稿漏画二级箭头，按 10 稿补（二级两项，占位）。
// 「打开档案」首项删除（2026-09-23 用户定：左键选中即打开，菜单项冗余；文件夹侧「打开文件夹」同删）。
export function archiveCardSpec(archive: { id: string; name: string }, rename?: () => void): CtxEntry[] {
  // 选择模式（6.2 一轮，PRD 03_数据链接 04）：右键仅留查看类操作，编辑类不出现
  if (usePickerStore().active) {
    return [
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
  const entries: CtxEntry[] = [
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
    // 复制档案：占位 disabled——复制/作为链接粘贴业务随档案多位置立项另轮（2026-09-05 走查定）
    { label: "复制档案", shortcut: "Ctrl+C", disabled: true },
    // 粘贴标签/描述（2026-09-05 走查二轮修订：不放复制数据二级——复制后一级跟在复制项后，
    // 与粘贴文件项习惯一致；合并追加；多选域内触发 = 域内全部档案批量）
    ...(selection.copiedData.tags?.length
      ? [
          {
            label: "粘贴标签",
            action: () => void pasteTags({ assets: [], archives: selectedArchiveIds(archive.id) }),
          } as CtxEntry,
        ]
      : []),
    ...(selection.copiedData.descriptions?.length
      ? [
          {
            label: "粘贴描述",
            action: () => void pasteDescriptions({ assets: [], archives: selectedArchiveIds(archive.id) }),
          } as CtxEntry,
        ]
      : []),
    {
      label: "重命名",
      shortcut: "F2",
      action: () => {
        if (rename) rename(); // 树档案节点：树命名态（folder.ts treeItemSpec 分流传入）
        else selection.namingFolderCardId = archive.id;
      },
    },
    "divider",
    // 克隆（2026-09-05 走查：档案「作为副本粘贴」删除，走与文件夹一样的克隆）：
    // clone_folders 递归复制子树（结构 / 资产物理副本 / 档案记录），多选域内触发批量分流
    { label: "克隆", shortcut: "Ctrl+D", action: () => cloneFolderFlow(archive.id) },
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
      // 2026-09-05 走查修正：文案写错——档案菜单是「导出档案」；「导出提示词为 .MD 文档」占位删除
      label: "导出档案",
      submenu: [
        { label: "导出为素材包", disabled: true },
        { label: "导出到本地磁盘", disabled: true },
      ],
    },
    "divider",
    // 提示词（2026-09-05 走查重做，一级「提示词」+ 展开箭头；提示词轮接业务）：
    // 三态二级共享构建器 menuSpecs/promptMenu——档案无「分析提示词」（走查原文），动作单目标
    {
      label: "提示词",
      submenu: archivePromptEntries(archive),
    },
    {
      label: "复制数据",
      submenu: [
        { label: "复制链接", action: () => void copyFolderLink(archive.id) },
        { label: "复制名称", action: () => void copyName(archive.name) },
        "divider",
        // 复制标签/描述（2026-09-05 走查）：写 selection.copiedData 会话槽，复制后一级菜单多出对应粘贴项
        { label: "复制标签", action: () => copyArchiveTags(archive.id) },
        { label: "复制描述", action: () => copyArchiveDescriptions(archive.id) },
      ],
    },
    "divider",
  ];
  // 删除组：混选档案+文件夹时两项分开（2026-09-05 走查）；单选/同构多选照旧「丢到回收站」
  const mixed = splitSelectedFolderKinds(archive.id);
  if (mixed) {
    entries.push(
      { label: "删除文件夹", shortcut: "Delete", action: () => deleteFoldersFlow(mixed.folders) },
      { label: "将档案丢到回收站", action: () => deleteFoldersFlow(mixed.archives) },
    );
  } else {
    entries.push({
      label: "丢到回收站",
      shortcut: "Delete",
      action: () => deleteFolderFlow(archive.id, archive.name),
    });
  }
  return entries;
}
