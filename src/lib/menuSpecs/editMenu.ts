// 菜单栏「编辑」按钮 · 编辑菜单 spec（2026-09-06 接业务轮；稿 = 用户当场供稿截图，同日会话）：
// 六组 = 撤销 Ctrl+Z / 重做 Ctrl+Shift+Z │ 剪切 Ctrl+X / 复制 Ctrl+C /
// 作为文件/档案链接粘贴 Ctrl+V / 作为副本粘贴 Ctrl+Shift+V │ 重命名 F2 │
// 创建副本 / 克隆（同键 Ctrl+D，按选中域分流显示——素材域出「创建副本」、文件夹·档案域出「克隆」，
// 2026-09-06 用户定）│ 全选 Ctrl+A / 清除选择 Esc │ 从文件夹中移除 Shift+Delete / 丢到回收站 Delete。
// 全项 = useGlobalKeys 既有快捷键的菜单面（分流 / 禁用同口径）；稿上「从文件夹中删除链接」为笔误——
// 删除链接语义已不存在（03 稿链接分流项 2026-08-18 废除），一律「从文件夹中移除」（2026-09-06 用户定）。
// 回收站视口只读——编辑类全 disabled（appMenu 整理同口径；回收站的删除入口是右键菜单「彻底删除」）；
// 选择模式（picker.active）下「编辑」钮禁开（全编辑类，速查 §12.1 白名单——调用方闸，同「菜单」钮）。
import { useContentStore } from "../../stores/content";
import { useSelectionStore } from "../../stores/selection";
import { useUndoStore } from "../../stores/undo";
import type { CtxEntry } from "../contextMenus";
import { cloneFolderFlow, deleteFoldersFlow, findParentId } from "../menuOps";
import {
  copySelection,
  cutFolders,
  deletePlacements,
  duplicateSelected,
  pasteClipboard,
  pasteDuplicate,
} from "../placementOps";

// 重命名（F2 同径）：文件夹域单选 = 卡片/树命名态分流（条带可见走卡片，否则树）；素材域单选 = 就地重命名
function renameFlow(): void {
  const content = useContentStore();
  const selection = useSelectionStore();
  if (selection.selectedFolders.size === 1) {
    const id = selection.orderedSelectedFolderIds()[0];
    const parent = findParentId(content.folders, id) ?? null;
    const cur = content.currentFolderId;
    if (cur !== null && parent === cur) selection.namingFolderCardId = id;
    else selection.namingFolderId = id;
    return;
  }
  if (selection.selectedFolders.size === 0 && selection.count === 1) {
    selection.renamingKey = selection.first;
  }
}

export function editMenuSpec(): CtxEntry[] {
  const content = useContentStore();
  const selection = useSelectionStore();
  const undo = useUndoStore();
  const folderIds = selection.orderedSelectedFolderIds();
  const hasFolders = folderIds.length > 0;
  const hasAssets = selection.count > 0;
  const recycle = content.isRecycleView;
  const canRename = !recycle && (selection.selectedFolders.size === 1 || (!hasFolders && selection.count === 1));
  return [
    { label: "撤销", shortcut: "Ctrl+Z", disabled: !undo.canUndo, action: () => void undo.undo() },
    { label: "重做", shortcut: "Ctrl+Shift+Z", disabled: !undo.canRedo, action: () => void undo.redo() },
    "divider",
    {
      label: "剪切",
      shortcut: "Ctrl+X",
      disabled: recycle || (!hasFolders && !hasAssets),
      action: () => {
        // 文件夹域 = 剪切文件夹（粘贴 = move_folders 护栏）；素材域 = 写内部剪贴板
        if (hasFolders) cutFolders(folderIds);
        else copySelection("cut");
      },
    },
    // 复制仅素材域（文件夹复制语义 = 克隆，走 Ctrl+D）
    { label: "复制", shortcut: "Ctrl+C", disabled: recycle || hasFolders || !hasAssets, action: () => copySelection("copy") },
    // 粘贴两项 = 内部剪贴板非空才可（空时 Ctrl+V 归系统剪贴板导入，不在本菜单语义内）；
    // 落点非法（快速访问视口）由 pasteClipboard 内部 toast 拦截，与快捷键同口径
    {
      label: "作为文件/档案链接粘贴",
      shortcut: "Ctrl+V",
      disabled: !selection.clipboard,
      action: () => void pasteClipboard(),
    },
    {
      label: "作为副本粘贴",
      shortcut: "Ctrl+Shift+V",
      disabled: !selection.clipboard,
      action: () => void pasteDuplicate(),
    },
    "divider",
    { label: "重命名", shortcut: "F2", disabled: !canRename, action: renameFlow },
    "divider",
    // 同键 Ctrl+D 分流显示（2026-09-06 用户定）：文件夹·档案域出「克隆」，其余出「创建副本」
    hasFolders
      ? { label: "克隆", shortcut: "Ctrl+D", action: () => cloneFolderFlow(folderIds[0]) }
      : { label: "创建副本", shortcut: "Ctrl+D", disabled: recycle || !hasAssets, action: () => void duplicateSelected() },
    "divider",
    { label: "全选", shortcut: "Ctrl+A", action: () => selection.selectAll() },
    {
      label: "清除选择",
      shortcut: "Esc",
      disabled: !hasAssets && !hasFolders && !content.selectedInboxName,
      action: () => selection.clear(),
    },
    "divider",
    {
      label: "从文件夹中移除",
      shortcut: "Shift+Delete",
      disabled: recycle || hasFolders || !hasAssets,
      action: () => void deletePlacements(selection.selectedRefs(), "uncategorized"),
    },
    {
      label: "丢到回收站",
      shortcut: "Delete",
      disabled: recycle || (!hasFolders && !hasAssets),
      action: () => {
        if (hasFolders) deleteFoldersFlow(folderIds);
        else void deletePlacements(selection.selectedRefs());
      },
    },
  ];
}
