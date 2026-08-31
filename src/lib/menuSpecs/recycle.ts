// 13 稿 · 回收站右键菜单 spec（自 menuSpecs.ts 拆出，规范第 3 条行数控制）：菜单项按新稿纯文字口径（2026-09-01）。
import { useContentStore } from "../../stores/content";
import type { CtxEntry } from "../contextMenus";
import { clearRecycle, recycleSelectedAssetIds, restoreRecycle } from "../placementOps";

// —— 13 稿 · 回收站 ——
// 空白 = 还原全部文件 / 清空回收站（空时禁用）；item 右键首行加「还原文件」（稿注单项入口，
// 语义统一为"文件"不区分文档，2026-08-11 用户定）。
// 彻底删除不入菜单（按稿；Del 快捷键与两段式确认业务保留在 placementOps）。
export function recycleBlankSpec(): CtxEntry[] {
  const content = useContentStore();
  return [
    {
      label: "还原全部文件",
      disabled: content.recycleEntries.length === 0,
      action: () => void restoreRecycle([...new Set(content.recycleEntries.map((e) => e.assetId))]),
    },
    {
      label: "清空回收站",
      disabled: content.recycleEntries.length === 0,
      action: () => void clearRecycle(),
    },
  ];
}

export function recycleCardSpec(): CtxEntry[] {
  return [
    { label: "还原文件", action: () => void restoreRecycle(recycleSelectedAssetIds()) },
    "divider",
    ...recycleBlankSpec(),
  ];
}
