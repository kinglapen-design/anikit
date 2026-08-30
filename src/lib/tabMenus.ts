// 标签页右键菜单（1.2，figma-css 39_右键菜单-标签页）：
// 关闭标签页（锁定时禁用）/ 锁定 / 新窗口打开 / 左右分屏 / 上下分屏。
// "新窗口打开"（2026-08-08 用户定案）：只有预览视图才允许新窗口打开——预览标签页化
// 属后续轮次，本轮标签页恒为文件视口，故该项恒置灰（设计稿口径：非预览视图禁用色）。
// 左右 / 上下分屏图标共用 split-screen（39 号稿两项同图标）。
import { openContextMenu, type CtxEntry } from "./contextMenus";
import { useTabsStore } from "../stores/tabs";

export function openTabContextMenu(e: MouseEvent, tabId: string): void {
  const tabs = useTabsStore();
  const tab = tabs.tabs.find((t) => t.id === tabId);
  if (!tab) return;
  const entries: CtxEntry[] = [
    {
      label: "关闭标签页",
      icon: "clear",
      disabled: tab.locked,
      action: () => tabs.closeTab(tabId),
    },
    // ≥2 个标签页时追加批量关闭（2026-08-08 用户定：关闭其他 = 排除右键目标项；锁定页跳过；
    // 文案统一带"页"字——标签和标签页是两个东西，防误解）
    ...(tabs.tabs.length >= 2
      ? ([
          { label: "关闭其他标签页", icon: "clear", action: () => tabs.closeOtherTabs(tabId) },
          { label: "关闭所有标签页", icon: "clear", action: () => tabs.closeAllTabs() },
        ] as CtxEntry[])
      : []),
    {
      label: tab.locked ? "取消锁定" : "锁定",
      icon: "pin-on",
      action: () => tabs.toggleLock(tabId),
    },
    { label: "新窗口打开", icon: "open-new-window", disabled: true },
    { label: "左右分屏", icon: "split-screen", action: () => tabs.splitTab(tabId, "right") },
    // 上下分屏：图标旋转 90°（语义对齐，2026-08-08 用户定）
    { label: "上下分屏", icon: "split-screen", iconClass: "rot-90", action: () => tabs.splitTab(tabId, "below") },
  ];
  openContextMenu(e, entries);
}
