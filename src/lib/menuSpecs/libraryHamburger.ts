// 08 稿 · 切换资源库 + 14 稿 · 汉堡菜单 spec（自 menuSpecs.ts 拆出，规范第 3 条行数控制）：
// 汉堡"资源库"二级复用 librarySwitchSpec 同一内容，两 spec 同文件。
import { invoke } from "@tauri-apps/api/core";
import type { CtxEntry, CtxItem } from "../contextMenus";
import { requestLibrarySetup } from "../libraryOps";
import { pickImportPaths } from "../menuOps";
import { openScan } from "../dupScanOps";
import { newDocGroupEntries } from "./addMenu";

// —— 08 稿 · 切换资源库（14 汉堡"资源库"二级复用同一内容）——
// 库列表 = 当前储存空间内库动态生成（2.1 空间化：list_space_libraries；checkable 勾 = 当前库；
// 点击切换 = libraryOps.switchLibrary 按名开库）；切换储存空间 / 工作空间（阶段九）、
// 合并资源库（单独立项，先补 PRD）、清除缓存 占位 disabled。
export function librarySwitchSpec(names: string[], _currentName: string | null): CtxEntry[] {
  const items: CtxEntry[] = names.map((name) => ({
    label: name,
    library: true, // 点击行为在 ContextMenu 动态判定（不配 action——静态快照会让 keepOpen 期间"切回去"失效）
    keepOpen: true, // 切库是状态切换类：执行后菜单保持（用户可切过去看一眼切回来，2026-08-12 用户定）
    checkable: true,
  }));
  return [
    { label: "切换储存空间", disabled: true, submenu: [] },
    "divider",
    ...(items.length > 0 ? items : [{ label: "（当前空间无资源库）", disabled: true } as CtxItem]),
    "divider",
    { label: "新建资源库", action: () => requestLibrarySetup() },
    { label: "合并资源库", disabled: true },
    { label: "工作空间", disabled: true, submenu: [] },
    "divider",
    { label: "清除缓存并重新加载", disabled: true },
  ];
}

// —— 14 稿 · 汉堡菜单 ——
// 资源库 ▸ = 08 切换资源库内容（用户定）；文件 ▸ = 新增菜单子集（导入文件/文件夹 + 新建文档组）；
// 查找 ▸ = 扫描两项（占位）；偏好设置 / 帮助 先不管（占位 disabled）；退出 = 完全结束进程（exit_app）。
export function hamburgerSpec(names: string[], currentName: string | null): CtxEntry[] {
  return [
    { label: "资源库", submenu: librarySwitchSpec(names, currentName) },
    {
      label: "文件",
      submenu: [
        { label: "导入本地文件", action: () => void pickImportPaths("files") },
        { label: "导入本地文件夹", action: () => void pickImportPaths("folder") },
        "divider",
        ...newDocGroupEntries(),
      ],
    },
    {
      label: "查找",
      submenu: [
        // 扫描重复文件已点亮（2.7 二轮，35 稿）：汉堡入口 = 全部范围；扫描链接仍占位
        { label: "扫描重复文件", action: () => void openScan({ kind: "all" }) },
        { label: "扫描文件/档案链接", disabled: true },
      ],
    },
    "divider",
    { label: "偏好设置", shortcut: "Ctrl+,", disabled: true },
    { label: "帮助", disabled: true, submenu: [] },
    { label: "退出", shortcut: "Alt+F4", action: () => void invoke("exit_app") },
  ];
}
