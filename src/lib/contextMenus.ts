// 右键菜单框架（6.3 初版；1.3 一轮 2026-08-11 按新稿 02_布局 03/04/05/07 升级）：
// 数据驱动——各上下文产出 CtxEntry 清单（设计项全列，未实现的一律 disabled 置灰占位），
// 全局单一宿主（App.vue 一个 <ContextMenu>），任何组件 openContextMenu 即可弹出。
// 与 AppMenu/MenuItem 的关系：渲染复用这两个组件（视觉口径不变），本模块只加
// 数据驱动、一级级联子菜单（flyout）、定宽与搜索框开关（opts）。
import { reactive } from "vue";

// 菜单条目：action 缺失或 disabled = 置灰占位；submenu = 一级级联（hover 展开）；
// checkable/checked 用于布局方式、排序等状态项；icon = 左侧 18px 图标（1.2 起，39_标签页菜单带图标）；
// submenuWidth = 二级菜单定宽（1.3：03§4=180 / 05§3=230 / §4=235 等稿定宽，缺省 175）
export interface CtxItem {
  label: string;
  icon?: string;
  iconClass?: string; // 图标附加类（如 rot-90 旋转，1.2 标签页菜单"上下分屏"用）
  shortcut?: string;
  disabled?: boolean;
  checkable?: boolean;
  checked?: boolean;
  submenu?: CtxEntry[];
  submenuWidth?: number;
  action?: () => void;
}
// "divider" = 组分隔线（设计稿的分组，组标题不出文字）
export type CtxEntry = CtxItem | "divider";

// 全局菜单状态（单一宿主）：非空 = 打开。
// onClose（2026-08-10）：关闭回调，executed = 执行了动作（true）还是取消（false，点外 / Esc）——
// 用于"右键未选中项先单选，菜单取消时恢复原选择"（FolderTree 树 item 右键）
// width/searchable（1.3）：菜单定宽（稿 320/300/212…，缺省 175）与是否带搜索框（03/04/05 稿）
export const menuState = reactive({
  x: 0,
  y: 0,
  entries: null as CtxEntry[] | null,
  width: null as number | null,
  searchable: false,
  onClose: null as ((executed: boolean) => void) | null,
});

export function openContextMenu(
  e: { clientX: number; clientY: number },
  entries: CtxEntry[],
  onClose?: (executed: boolean) => void,
  opts?: { width?: number; searchable?: boolean },
): void {
  menuState.x = e.clientX;
  menuState.y = e.clientY;
  menuState.entries = entries;
  menuState.width = opts?.width ?? null;
  menuState.searchable = opts?.searchable ?? false;
  menuState.onClose = onClose ?? null;
}

export function closeContextMenu(executed = false): void {
  const cb = menuState.onClose;
  menuState.entries = null;
  menuState.width = null;
  menuState.searchable = false;
  menuState.onClose = null;
  cb?.(executed);
}
