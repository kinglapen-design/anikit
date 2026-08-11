// 08 稿 · 切换资源库 + 14 稿 · 汉堡菜单 spec（自 menuSpecs.ts 拆出，规范第 3 条行数控制）：
// 汉堡"资源库"二级复用 librarySwitchSpec 同一内容，两 spec 同文件。
import { invoke } from "@tauri-apps/api/core";
import type { CtxEntry, CtxItem } from "../contextMenus";
import { requestLibrarySetup, switchLibrary, type LibraryEntry } from "../libraryOps";
import { pickImportPaths } from "../menuOps";
import { newDocGroupEntries } from "./addMenu";

// —— 08 稿 · 切换资源库（宽 300；14 汉堡"资源库"二级复用同一内容）——
// 库列表 = 最近列表动态生成（checkable 勾 = 当前库；点击切换 = libraryOps.switchLibrary）；
// 切换储存空间 / 工作空间（阶段九）、合并资源库（2.1）、清除缓存 占位 disabled；
// 库 item 的移动抓手 / 删除×（最近列表移除需后端命令）未做，记 09 待办。
export function librarySwitchSpec(entries: LibraryEntry[], currentPath: string | null): CtxEntry[] {
  const items: CtxEntry[] = entries.map((e) => ({
    label: e.name,
    icon: "library-duotone",
    checkable: true,
    checked: e.path === currentPath,
    action: e.path === currentPath ? undefined : () => void switchLibrary(e.path),
  }));
  return [
    { label: "切换储存空间", icon: "switch-space", disabled: true, submenu: [] },
    "divider",
    ...(items.length > 0 ? items : [{ label: "（无最近资源库）", disabled: true } as CtxItem]),
    "divider",
    { label: "新建资源库", icon: "library", action: () => requestLibrarySetup() },
    { label: "合并资源库", icon: "merge-libraries", disabled: true },
    { label: "工作空间", icon: "workspace", disabled: true, submenu: [] },
    "divider",
    { label: "清除缓存并重新加载", icon: "clear-cache-reload", disabled: true },
  ];
}

// —— 14 稿 · 汉堡菜单（宽 220，一级项稿无图标）——
// 资源库 ▸ = 08 切换资源库内容（用户定）；文件 ▸ = 新增菜单子集（导入文件/文件夹 + 新建文档组）；
// 查找 ▸ = 扫描两项（占位）；偏好设置 / 帮助 先不管（占位 disabled）；退出 = 完全结束进程（exit_app）。
export function hamburgerSpec(entries: LibraryEntry[], currentPath: string | null): CtxEntry[] {
  return [
    { label: "资源库", submenuWidth: 300, submenu: librarySwitchSpec(entries, currentPath) },
    {
      label: "文件",
      submenuWidth: 213,
      submenu: [
        { label: "导入本地文件", icon: "import-file", action: () => void pickImportPaths("files") },
        { label: "导入本地文件夹", icon: "import-folder", action: () => void pickImportPaths("folder") },
        "divider",
        ...newDocGroupEntries(),
      ],
    },
    {
      label: "查找",
      submenuWidth: 213,
      submenu: [
        { label: "扫描重复文件", icon: "scan-duplicates", disabled: true },
        { label: "扫描文件/档案链接", icon: "scan-links", disabled: true },
      ],
    },
    "divider",
    { label: "偏好设置", shortcut: "Ctrl+,", disabled: true },
    { label: "帮助", disabled: true, submenu: [] },
    { label: "退出", shortcut: "Alt+F4", action: () => void invoke("exit_app") },
  ];
}
