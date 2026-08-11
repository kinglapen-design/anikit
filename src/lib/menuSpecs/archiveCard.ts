// 04 稿 · 档案卡片右键菜单 spec（自 menuSpecs.ts 拆出，规范第 3 条行数控制）：宽 320 + 搜索框。
import { useContentStore } from "../../stores/content";
import { useSelectionStore } from "../../stores/selection";
import { useTabsStore } from "../../stores/tabs";
import type { CtxEntry } from "../contextMenus";
import {
  copyFolderLink,
  copyName,
  createAndName,
  deleteFolderFlow,
  findParentId,
  pickFolderForLink,
} from "../menuOps";

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
