// 收件箱"工程文件"磁盘条目右键菜单 spec（1.3 归并全局宿主，自 InboxDiskSection 局部菜单迁入）：
// 四项沿用原局部菜单的顺序与文案（打开 / 重命名 / 删除 ─ 导入），占用条目禁用导入与重命名。
// 动作回调由组件注入（行内重命名等 UI 状态在组件侧）；菜单无定宽（原局部菜单同，用缺省）。
import type { CtxEntry } from "../contextMenus";

export interface InboxDiskMenuActions {
  onOpen: () => void;
  onRename: () => void;
  onDelete: () => void;
  onImport: () => void;
}

export function inboxDiskSpec(
  entry: { occupied: boolean },
  selected: { count: number; occupied: boolean },
  actions: InboxDiskMenuActions,
): CtxEntry[] {
  return [
    { label: "打开", action: actions.onOpen },
    { label: "重命名", disabled: entry.occupied, action: actions.onRename },
    { label: "删除", action: actions.onDelete },
    "divider",
    {
      label: selected.count > 1 ? `导入 ${selected.count} 项` : "导入",
      disabled: selected.occupied,
      action: actions.onImport,
    },
  ];
}
