// 菜单栏「视图」按钮 · 视图菜单 spec（2026-09-06 接业务轮；稿 = 用户当场供稿截图，同日会话）：
// 四组 = 大纲管理 / 分组管理 / 信息查看管理 │ 查看方式 ▸ / 标签页 ▸ │
// 显示侧边栏 / 显示信息面板 / 始终显示搜索框 │ 重置窗口（2026-09-07 用户定）。
//
// 口径：
// - 大纲管理 = 侧栏区块管理弹窗（30 稿 BlockManageDialog，ui.blockManageOpen——侧栏大纲的
//   分区块显隐排序管理，三组管理项对应三个区域：侧栏大纲 / 内容分组 / 右栏信息查看）；
// - 分组管理 = 33 稿 GroupManageDialog（ui.groupManageOpen + groupDialogFolder 锁当前文件夹视口，
//   ContentTitleBar 分组钮同径；非文件夹视口无目标 disabled）；
// - 信息查看管理 = 右栏底部同名入口同一业务（2026-09-26 接：InfoManageDialog 排序/显隐管理弹窗）；
// - 查看方式子菜单全项 = ui store 既有持久化字段的菜单面（layoutMode / sortBy / sortAsc /
//   timeDisplay / showBrief / briefMode / showExt / showBadge / showLinkBadge / showSubfolders /
//   autoPlayGif / autoPlayVideo——与工具栏 ViewOptionsMenu 同状态源，天然同步）；
// - 布局四项快捷键 Alt+1~4（稿标注，useGlobalKeys onViewKeys 同轮落地）；
// - 排序「自定义」= 存储序无升降语义——升/降序两项随 sortBy 实时禁用（3.5 定，disabled 函数化）；
// - 简介「标签」档占位禁用（卡片数据无 tags，ViewOptionsMenu 同口径）；
// - 状态翻转项一律 keepOpen（2026-08-12 用户定：可逆状态翻转类保持打开）+ checked/disabled
//   函数化（ContextMenu 渲染期求值，菜单开着对勾随状态实时刷新）；
// - 标签页子菜单 = submenu 函数实时生成（标签清单随开随关）：列表点击 = activateTab 切换激活；
//   关闭当前 = 锁定页禁用（closeTab 守卫同口径）；关闭所有/其他 = 单页禁用（tabMenus ≥2 口径，
//   主菜单恒列）；分屏两项 = splitTab right/below（39 稿）；
// - 显示侧边栏 / 显示信息面板 = leftCollapsed / rightCollapsed 反相；
// - 始终显示搜索框（2026-09-06 用户定）：关 = 悬浮栏搜索平时隐藏（Ctrl+F 唤出 / Esc 收回，
//   下沉/浮入动画），关闭动作 toast 告知 Ctrl+F 通道；
// - 选择模式（picker.active）：查看方式 / 搜索框放行（§12.1 白名单），管理弹窗 / 标签页 /
//   侧栏收展（左栏 = 挑选主通路恒在；右栏 = PickerPanel 整体替换中）禁用。
import { LogicalSize, getCurrentWindow } from "@tauri-apps/api/window";
import { useContentStore } from "../../stores/content";
import { usePickerStore } from "../../stores/picker";
import { useTabsStore } from "../../stores/tabs";
import { useToastStore } from "../../stores/toast";
import { useUiStore } from "../../stores/ui";
import type { CtxEntry, CtxItem } from "../contextMenus";
import { isDisplaySorted, type SortKey } from "../sortPlacements";

// 布局方式四档（稿序 = Alt+1~4；值 ↔ ui.layoutMode，ViewOptionsMenu layouts 同序）
const LAYOUTS: readonly { value: "justified" | "waterfall" | "grid" | "list"; label: string; shortcut: string }[] = [
  { value: "justified", label: "自适应", shortcut: "Alt+1" },
  { value: "waterfall", label: "瀑布流", shortcut: "Alt+2" },
  { value: "grid", label: "网格", shortcut: "Alt+3" },
  { value: "list", label: "列表", shortcut: "Alt+4" },
];

// 排序键 8 项（稿序；"" = 自定义存储序；ViewOptionsMenu sortKeys 同口径）
const SORT_KEYS: readonly { value: SortKey; label: string }[] = [
  { value: "", label: "自定义" },
  { value: "name", label: "名称" },
  { value: "size", label: "文件大小" },
  { value: "duration", label: "时长" },
  { value: "ext", label: "扩展名" },
  { value: "createdAt", label: "创建时间" },
  { value: "modifiedAt", label: "修改时间" },
  { value: "importedAt", label: "导入时间" },
];

// 简介内容五档（稿序；标签档 = 占位禁用——卡片数据无 tags，ViewOptionsMenu 同口径）
const BRIEF_MODES: readonly { value: "size" | "fileSize" | "createdAt" | "modifiedAt" | "tags"; label: string; disabled?: boolean }[] = [
  { value: "size", label: "尺寸" },
  { value: "fileSize", label: "文件大小" },
  { value: "createdAt", label: "创建日期" },
  { value: "modifiedAt", label: "修改日期" },
  { value: "tags", label: "标签", disabled: true },
];

// 布尔开关项工厂（状态翻转类：checkable + 函数 checked + keepOpen + 翻转 action）
function toggleEntry(label: string, get: () => boolean, set: (v: boolean) => void, onOff?: () => void): CtxItem {
  return {
    label,
    checkable: true,
    checked: get,
    keepOpen: true,
    action: () => {
      const next = !get();
      set(next);
      if (!next) onOff?.();
    },
  };
}

// 「始终显示搜索框」关闭动作（2026-09-06 用户定）：toast 告知 Ctrl+F 唤出通道，防用户找不到
function toggleSearchBar(): void {
  const ui = useUiStore();
  ui.showSearchBar = !ui.showSearchBar;
  if (!ui.showSearchBar) {
    useToastStore().notify({ message: "搜索框已隐藏，按 Ctrl+F 可随时调出" });
  }
}

// 「重置窗口」（2026-09-07 用户定）：窗口与三栏回设计基准——1400×1100 = 左 254 + 中 880 + 右 266
//（布局/主窗口稿；右栏稿 264 与主窗口稿 1400 总宽差 2px，以用户定 266 收口，中栏 880 由总宽挤出）。
// 最大化先还原再定尺寸（最大化下 setSize 不生效）；收起的侧栏一并展开——三栏齐才达成 254/880/266
async function resetWindow(): Promise<void> {
  const ui = useUiStore();
  ui.leftWidth = 254;
  ui.rightWidth = 266;
  ui.leftCollapsed = false;
  ui.rightCollapsed = false;
  const win = getCurrentWindow();
  try {
    if (await win.isMaximized()) await win.unmaximize();
    await win.setSize(new LogicalSize(1400, 1100));
  } catch (e) {
    console.warn("重置窗口尺寸失败：", e);
  }
}

// —— 查看方式 ▸（全部 = ui store 既有字段的菜单面）——
function viewModeEntries(): CtxEntry[] {
  const ui = useUiStore();
  return [
    ...LAYOUTS.map(
      (m): CtxItem => ({
        label: m.label,
        shortcut: m.shortcut,
        checkable: true,
        checked: () => ui.layoutMode === m.value,
        keepOpen: true,
        action: () => {
          ui.layoutMode = m.value;
        },
      }),
    ),
    "divider",
    {
      label: "排序方式",
      submenu: [
        ...SORT_KEYS.map(
          (s): CtxItem => ({
            label: s.label,
            checkable: true,
            checked: () => ui.sortBy === s.value,
            keepOpen: true,
            action: () => {
              ui.sortBy = s.value;
            },
          }),
        ),
        "divider",
        // 自定义 = 存储序无升降语义——两项随 sortBy 实时禁用（3.5 定）
        {
          label: "升序",
          checkable: true,
          checked: () => ui.sortAsc,
          disabled: () => !isDisplaySorted(ui.sortBy),
          keepOpen: true,
          action: () => {
            ui.sortAsc = true;
          },
        },
        {
          label: "降序",
          checkable: true,
          checked: () => !ui.sortAsc,
          disabled: () => !isDisplaySorted(ui.sortBy),
          keepOpen: true,
          action: () => {
            ui.sortAsc = false;
          },
        },
      ],
    },
    {
      label: "时间显示格式",
      submenu: [
        {
          label: "帧数",
          checkable: true,
          checked: () => ui.timeDisplay === "frames",
          keepOpen: true,
          action: () => {
            ui.timeDisplay = "frames";
          },
        },
        {
          label: "时间码",
          checkable: true,
          checked: () => ui.timeDisplay === "timecode",
          keepOpen: true,
          action: () => {
            ui.timeDisplay = "timecode";
          },
        },
      ],
    },
    {
      label: "简介显示方式",
      submenu: [
        toggleEntry("显示简介", () => ui.showBrief, (v) => (ui.showBrief = v)),
        "divider",
        ...BRIEF_MODES.map(
          (b): CtxItem => ({
            label: b.label,
            checkable: true,
            checked: () => ui.briefMode === b.value,
            disabled: b.disabled,
            keepOpen: true,
            action: () => {
              ui.briefMode = b.value;
            },
          }),
        ),
      ],
    },
    "divider",
    toggleEntry("显示扩展名", () => ui.showExt, (v) => (ui.showExt = v)),
    toggleEntry("显示角标", () => ui.showBadge, (v) => (ui.showBadge = v)),
    toggleEntry("显示链接标识", () => ui.showLinkBadge, (v) => (ui.showLinkBadge = v)),
    toggleEntry("显示子文件夹内容", () => ui.showSubfolders, (v) => (ui.showSubfolders = v)),
    "divider",
    toggleEntry("自动播放动图", () => ui.autoPlayGif, (v) => (ui.autoPlayGif = v)),
    toggleEntry("自动播放视频预览", () => ui.autoPlayVideo, (v) => (ui.autoPlayVideo = v)),
  ];
}

// —— 标签页 ▸（submenu 函数 = hover 展开时实时生成，标签清单不滞留旧快照）——
function tabEntries(): CtxEntry[] {
  const tabs = useTabsStore();
  const content = useContentStore();
  const cur = tabs.activeTabId;
  const curTab = tabs.tabs.find((t) => t.id === cur);
  const single = tabs.tabs.length < 2;
  return [
    ...tabs.tabs.map(
      (t): CtxItem => ({
        label: content.viewTitle(t.viewKey),
        checkable: true,
        checked: t.id === cur,
        action: () => tabs.activateTab(t.id),
      }),
    ),
    "divider",
    { label: "锁定当前标签页", disabled: !curTab, action: () => cur && tabs.toggleLock(cur) },
    "divider",
    { label: "关闭当前标签页", disabled: !curTab || curTab.locked, action: () => cur && tabs.closeTab(cur) },
    { label: "关闭所有标签页", disabled: single, action: () => tabs.closeAllTabs() },
    { label: "关闭其他标签页", disabled: single || !cur, action: () => cur && tabs.closeOtherTabs(cur) },
    "divider",
    { label: "左右分屏", disabled: !curTab, action: () => cur && tabs.splitTab(cur, "right") },
    { label: "上下分屏", disabled: !curTab, action: () => cur && tabs.splitTab(cur, "below") },
  ];
}

export function viewMenuSpec(): CtxEntry[] {
  const ui = useUiStore();
  const content = useContentStore();
  const picking = usePickerStore().active;
  // 分组管理目标 = 当前文件夹视口（ContentTitleBar 分组钮同径）；非文件夹视口无目标
  const folderId = content.currentFolderId;
  return [
    { label: "大纲管理", disabled: picking, action: () => (ui.blockManageOpen = true) },
    {
      label: "分组管理",
      disabled: picking || folderId === null,
      action: () => {
        if (folderId === null) return;
        ui.groupDialogFolder = folderId;
        ui.groupManageOpen = true;
      },
    },
    // 信息查看管理 = 右栏底部同名入口同一业务（2026-09-26 接：排序/显隐管理弹窗，宿主 App.vue）
    { label: "信息查看管理", disabled: picking, action: () => (ui.infoManageOpen = true) },
    "divider",
    { label: "查看方式", submenu: viewModeEntries() },
    { label: "标签页", disabled: picking, submenu: tabEntries },
    "divider",
    // 选择模式下侧栏收展禁用（左栏 = 挑选主通路恒在；右栏 = PickerPanel 整体替换中）
    {
      ...toggleEntry("显示侧边栏", () => !ui.leftCollapsed, (v) => (ui.leftCollapsed = !v)),
      disabled: picking,
    },
    {
      ...toggleEntry("显示信息面板", () => !ui.rightCollapsed, (v) => (ui.rightCollapsed = !v)),
      disabled: picking,
    },
    { label: "始终显示搜索框", checkable: true, checked: () => ui.showSearchBar, keepOpen: true, action: toggleSearchBar },
    "divider",
    { label: "重置窗口", action: () => void resetWindow() },
  ];
}
