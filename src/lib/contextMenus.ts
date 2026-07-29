// 右键菜单框架（6.3，2026-07-29 用户设计 `00_修改与需求/05_右键菜单`）：
// 数据驱动——六种上下文各自产出 CtxEntry 清单（未实现项 disabled 置灰占位，图标一律空），
// 全局单一宿主（App.vue 一个 <ContextMenu>），任何组件 openContextMenu 即可弹出。
// 与 AppMenu/MenuItem 的关系：渲染复用这两个组件（视觉口径不变），本模块只加
// 数据驱动与一级级联子菜单（flyout，原 AppMenu 不支持）。
import { reactive } from "vue";

// 菜单条目：action 缺失或 disabled = 置灰占位；submenu = 一级级联（hover 展开）；
// checkable/checked 用于布局方式、排序等状态项
export interface CtxItem {
  label: string;
  shortcut?: string;
  disabled?: boolean;
  checkable?: boolean;
  checked?: boolean;
  submenu?: CtxEntry[];
  action?: () => void;
}
// "divider" = 组分隔线（设计稿的分组，组标题不出文字）
export type CtxEntry = CtxItem | "divider";

// 全局菜单状态（单一宿主）：非空 = 打开
export const menuState = reactive({
  x: 0,
  y: 0,
  entries: null as CtxEntry[] | null,
});

export function openContextMenu(e: { clientX: number; clientY: number }, entries: CtxEntry[]): void {
  menuState.x = e.clientX;
  menuState.y = e.clientY;
  menuState.entries = entries;
}

export function closeContextMenu(): void {
  menuState.entries = null;
}
