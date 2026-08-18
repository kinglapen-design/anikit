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
  iconData?: string; // 内联图片图标 data URL / 图片文件 URL（非 svg 图标用，如新建文档组 md.webp / txt.png，优先于 icon）
  shortcut?: string;
  disabled?: boolean;
  checkable?: boolean;
  checked?: boolean;
  library?: boolean; // 资源库条目（08 稿：抓手拖动排序 + 品牌徽章 + 右勾 + 删除×，LibraryMenuItem 渲染）
  keepOpen?: boolean; // 执行后不关菜单（状态切换/预览类：切库、排序依据等可反复尝试的操作；
  // 判定口径 2026-08-12 用户定：一次性任务类（新建/重命名/删除/导入，进入后续输入或任务流）必关，
  // 可逆状态翻转类（用户可能立即再做一次，如切过去看一眼切回来）保持打开
  submenu?: CtxEntry[];
  submenuWidth?: number;
  // 自定义网格面板（2026-08-18，3.3 / 05 稿 §3§4）：与 submenu 互斥——hover 时 flyout
  // 渲染对应面板组件（FolderIconPanel / FolderColorPanel）而非条目列表；宽度仍由 submenuWidth 给
  panel?: { kind: "folderIcon" | "folderColor"; folderId: string };
  action?: () => void;
}
// "divider" = 组分隔线（设计稿的分组，组标题不出文字）
export type CtxEntry = CtxItem | "divider";

// 整体面板菜单内容（openPanelMenu，2026-08-18）：kind 对应 ContextMenu 根菜单的面板渲染分支；
// iconPicker = 05 稿 §3 图标网格纯选择器（区块管理行「默认图标」下拉用，选中回调 + 关菜单）
export interface PanelMenuState {
  kind: "iconPicker";
  current: string | null;
  onPick: (name: string | null) => void;
}

// 全局菜单状态（单一宿主）：非空 = 打开。
// onClose（2026-08-10）：关闭回调，executed = 执行了动作（true）还是取消（false，点外 / Esc）——
// 用于"右键未选中项先单选，菜单取消时恢复原选择"（FolderTree 树 item 右键）
// width/searchable（1.3）：菜单定宽（稿 320/300/212…，缺省 175）与是否带搜索框（03/04/05 稿）
// panel（2026-08-18）：整体面板菜单形态（一级内容 = 面板组件，无条目列表，见 openPanelMenu）
export const menuState = reactive({
  x: 0,
  y: 0,
  entries: null as CtxEntry[] | null,
  width: null as number | null,
  searchable: false,
  panel: null as PanelMenuState | null,
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
  menuState.panel = null;
  menuState.onClose = onClose ?? null;
}

// 整体面板菜单（2026-08-18，3.3）：一级内容直接 = 网格面板，无条目列表、无搜索框——
// 用于"点按钮弹选择器"场景（首个用例 = 区块管理行「默认图标」网格下拉，05 稿 §3 网格共用）；
// 树菜单二级网格面板走 CtxItem.panel + flyout 分支，与本机制不同（hover 级联 vs 直接弹出）
export function openPanelMenu(e: { clientX: number; clientY: number }, panel: PanelMenuState, opts?: { width?: number }): void {
  menuState.x = e.clientX;
  menuState.y = e.clientY;
  menuState.entries = []; // 非空 = 打开；渲染分支以 panel 为准
  menuState.width = opts?.width ?? null;
  menuState.searchable = false;
  menuState.panel = panel;
  menuState.onClose = null;
}

export function closeContextMenu(executed = false): void {
  const cb = menuState.onClose;
  menuState.entries = null;
  menuState.width = null;
  menuState.searchable = false;
  menuState.panel = null;
  menuState.onClose = null;
  cb?.(executed);
}
