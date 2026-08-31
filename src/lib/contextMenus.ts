// 右键菜单框架（6.3 初版；7.1 二十四轮 2026-09-01 按新稿 组件/上下文菜单 重写）：
// 数据驱动——各上下文产出 CtxEntry 清单（设计项全列，未实现的一律 disabled 置灰占位），
// 全局单一宿主（App.vue 一个 <ContextMenu>），任何组件 openContextMenu 即可弹出。
// 与 AppMenu/MenuItem 的关系：渲染复用这两个组件（视觉口径不变），本模块只加数据驱动与
// 一级级联子菜单（flyout）。菜单宽度 = 内容自适应（2026-09-01 二十五轮用户定，废 300 定宽：
// 名称→36px→右件最小间距，容器宽由最长 item 决定）。
// 2026-09-01 新稿废除：item 左侧图标位（icon/iconClass/iconData）、搜索区（searchable）、
// 各菜单定宽（width/submenuWidth）、孤立分割线元素（渲染层按 "divider" 切分组容器）。
import { reactive } from "vue";

// 菜单条目：action 缺失或 disabled = 置灰占位；submenu = 一级级联（hover 展开）；
// checkable/checked 用于布局方式、排序等状态项（对勾在右侧：常态隐 / hover 三级 / 选中一级）；
// trailingIcon = 标题右侧小图标 12（3.5 分组菜单「上移/下移」口径，功能保留）
export interface CtxItem {
  label: string;
  trailingIcon?: string; // 标题右侧小图标 12（chevron-small 等）
  trailingIconClass?: string; // 尾图标附加类（rot-180）
  shortcut?: string; // 快捷键纯文本（右侧，Fira Mono 12 三级色；原串显示如 "Ctrl+Enter"）
  disabled?: boolean;
  checkable?: boolean;
  checked?: boolean;
  library?: boolean; // 资源库条目（08 稿：抓手拖动排序 + 品牌徽章 + 右勾 + 删除×，LibraryMenuItem 渲染）
  keepOpen?: boolean; // 执行后不关菜单（状态切换/预览类：切库、排序依据等可反复尝试的操作；
  // 判定口径 2026-08-12 用户定：一次性任务类（新建/重命名/删除/导入，进入后续输入或任务流）必关，
  // 可逆状态翻转类（用户可能立即再做一次，如切过去看一眼切回来）保持打开
  submenu?: CtxEntry[];
  // 自定义网格面板（2026-08-18，3.3 / 05 稿 §3）：与 submenu 互斥——hover 时 flyout
  // 渲染对应面板组件（FolderIconPanel）而非条目列表
  panel?: { kind: "folderIcon"; folderId: string };
  // 内嵌色块行（2026-09-01 新稿「右键菜单-更改颜色」）：item 右侧 10 色块（常态-1 = 默认清色
  // + 九色-1 按 token 声明序），点击色块直接生效且**不关菜单**（用户可连续试色）；
  // 当前色块 1.5px 亮色/一级外环。整行（非色块区）点击无动作。
  swatches?: { colors: { tint: string | null; hex: string; label: string }[]; current: string | null; onPick: (tint: string | null) => void };
  action?: () => void;
}
// "divider" = 组分隔标记（数据层分组；渲染层切成组容器，组间 1px 通用/五级线，无孤立分割线元素）
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
// panel（2026-08-18）：整体面板菜单形态（一级内容 = 面板组件，无条目列表，见 openPanelMenu）
export const menuState = reactive({
  x: 0,
  y: 0,
  entries: null as CtxEntry[] | null,
  panel: null as PanelMenuState | null,
  onClose: null as ((executed: boolean) => void) | null,
});

export function openContextMenu(e: { clientX: number; clientY: number }, entries: CtxEntry[], onClose?: (executed: boolean) => void): void {
  menuState.x = e.clientX;
  menuState.y = e.clientY;
  menuState.entries = entries;
  menuState.panel = null;
  menuState.onClose = onClose ?? null;
}

// 整体面板菜单（2026-08-18，3.3）：一级内容直接 = 网格面板，无条目列表——
// 用于"点按钮弹选择器"场景（首个用例 = 区块管理行「默认图标」网格下拉，05 稿 §3 网格共用）；
// 树菜单二级网格面板走 CtxItem.panel + flyout 分支，与本机制不同（hover 级联 vs 直接弹出）
export function openPanelMenu(e: { clientX: number; clientY: number }, panel: PanelMenuState): void {
  menuState.x = e.clientX;
  menuState.y = e.clientY;
  menuState.entries = []; // 非空 = 打开；渲染分支以 panel 为准
  menuState.panel = panel;
  menuState.onClose = null;
}

export function closeContextMenu(executed = false): void {
  const cb = menuState.onClose;
  menuState.entries = null;
  menuState.panel = null;
  menuState.onClose = null;
  cb?.(executed);
}
