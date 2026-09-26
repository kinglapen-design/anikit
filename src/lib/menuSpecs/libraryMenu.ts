// 资源库菜单 spec（2026-09-07 自 appMenu 仓库▸资源库 二级提取共享）：
// 调用方 = 菜单栏「菜单」仓库▸资源库 二级（appMenu.repoEntries）+ 侧栏资源库行切换钮（LibraryView，一级菜单）。
// 库列表 = 纯对勾行（MenuItem checkable，2026-09-05 用户定稿口径）——08 稿 LibraryMenuItem 抓手排序 +
// 删除× 全库废除（2026-09-07 用户：资源库不至于多到需要排序），删除走同级「删除当前资源库」项。
import type { CtxEntry } from "../contextMenus";
import { currentLibrary, requestDeleteLibrary, requestLibrarySetup, requestMergeLibrary, requestRenameLibrary, switchLibrary } from "../libraryOps";

// 新建 / 删除当前 / 合并（2026-09-26 功能轮接通；二轮修订：全库展示 + 勾选 ≥2，目标 = 勾选集中
// libraryOrder 最前者）+ 库列表。对勾与禁用函数化求值（keepOpen 切库期间随 currentLibrary 实时）；
// 库项不配静态 action 快照——点击时动态判定，当前库跳过、其余按名切（keepOpen = 切库可切回，2026-08-12 用户定）
export function libraryMenuEntries(names: string[]): CtxEntry[] {
  const libRows: CtxEntry[] =
    names.length > 0
      ? names.map((name) => ({
          label: name,
          keepOpen: true,
          checkable: true,
          checked: () => name === currentLibrary.name,
          action: () => {
            if (name !== currentLibrary.name) void switchLibrary(name);
          },
        }))
      : [{ label: "（当前空间无资源库）", disabled: true }];
  return [
    { label: "新建资源库", action: () => requestLibrarySetup() },
    {
      label: "重命名当前资源库",
      disabled: () => !currentLibrary.name, // 无当前库不可点
      action: () => {
        if (currentLibrary.name) requestRenameLibrary(currentLibrary.name);
      },
    },
    {
      label: "删除当前资源库",
      disabled: () => !currentLibrary.name, // 无当前库不可点
      action: () => {
        if (currentLibrary.name) requestDeleteLibrary(currentLibrary.name);
      },
    },
    {
      label: "合并资源库",
      // 空间内不足两库不可点（2026-09-26 二轮：全库展示勾选 ≥2，目标 = 最上方勾选库，不再要求当前库）
      disabled: () => names.length < 2,
      action: () => void requestMergeLibrary(),
    },
    "divider",
    ...libRows,
  ];
}
