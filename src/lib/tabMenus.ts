// 标签页右键菜单（1.2，figma-css 39_右键菜单-标签页）：
// 关闭标签页（锁定时禁用）/ 锁定 / 新窗口打开 / 左右分屏 / 上下分屏。
// "新窗口打开"（2026-08-08 用户定案）：只有预览视图才允许新窗口打开——预览标签页化
// 属后续轮次，本轮标签页恒为文件视口，故该项恒置灰（设计稿口径：非预览视图禁用色）。
import { openContextMenu, type CtxEntry } from "./contextMenus";
import { useTabsStore } from "../stores/tabs";

export function openTabContextMenu(e: MouseEvent, tabId: string): void {
  const tabs = useTabsStore();
  const tab = tabs.tabs.find((t) => t.id === tabId);
  if (!tab) return;
  const entries: CtxEntry[] = [
    {
      label: "关闭标签页",
      disabled: tab.locked,
      action: () => tabs.closeTab(tabId),
    },
    // ≥2 个标签页时追加批量关闭（2026-08-08 用户定：关闭其他 = 排除右键目标项；锁定页跳过；
    // 文案统一带"页"字——标签和标签页是两个东西，防误解）
    ...(tabs.tabs.length >= 2
      ? ([
          { label: "关闭其他标签页", action: () => tabs.closeOtherTabs(tabId) },
          { label: "关闭所有标签页", action: () => tabs.closeAllTabs() },
        ] as CtxEntry[])
      : []),
    {
      label: tab.locked ? "取消锁定" : "锁定",
      action: () => tabs.toggleLock(tabId),
    },
    { label: "新窗口打开", disabled: true },
    { label: "左右分屏", action: () => tabs.splitTab(tabId, "right") },
    { label: "上下分屏", action: () => tabs.splitTab(tabId, "below") },
  ];
  openContextMenu(e, entries);
}
