// 右键菜单框架（6.3，2026-07-29 用户设计 `00_修改与需求/05_右键菜单`）：
// 数据驱动——六种上下文各自产出 CtxEntry 清单（未实现项 disabled 置灰占位，图标一律空），
// 全局单一宿主（App.vue 一个 <ContextMenu>），任何组件 openContextMenu 即可弹出。
// 与 AppMenu/MenuItem 的关系：渲染复用这两个组件（视觉口径不变），本模块只加
// 数据驱动与一级级联子菜单（flyout，原 AppMenu 不支持）。
import { reactive } from "vue";

// 菜单条目：action 缺失或 disabled = 置灰占位；submenu = 一级级联（hover 展开）；
// checkable/checked 用于布局方式、排序等状态项；icon = 左侧 18px 图标（1.2 起，39_标签页菜单带图标）
export interface CtxItem {
  label: string;
  icon?: string;
  iconClass?: string; // 图标附加类（如 rot-90 旋转，1.2 标签页菜单"上下分屏"用）
  shortcut?: string;
  disabled?: boolean;
  checkable?: boolean;
  checked?: boolean;
  submenu?: CtxEntry[];
  action?: () => void;
}
// "divider" = 组分隔线（设计稿的分组，组标题不出文字）
export type CtxEntry = CtxItem | "divider";

// 全局菜单状态（单一宿主）：非空 = 打开。
// onClose（2026-08-10）：关闭回调，executed = 执行了动作（true）还是取消（false，点外 / Esc）——
// 用于"右键未选中项先单选，菜单取消时恢复原选择"（FolderTree 树 item 右键）
export const menuState = reactive({
  x: 0,
  y: 0,
  entries: null as CtxEntry[] | null,
  onClose: null as ((executed: boolean) => void) | null,
});

export function openContextMenu(
  e: { clientX: number; clientY: number },
  entries: CtxEntry[],
  onClose?: (executed: boolean) => void,
): void {
  menuState.x = e.clientX;
  menuState.y = e.clientY;
  menuState.entries = entries;
  menuState.onClose = onClose ?? null;
}

export function closeContextMenu(executed = false): void {
  const cb = menuState.onClose;
  menuState.entries = null;
  menuState.onClose = null;
  cb?.(executed);
}
