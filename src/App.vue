<script setup lang="ts">
// 主窗口（7.1 十七轮 · 主窗口骨架轮贴稿）：transparent:false + WebView2 背景透明
// （063 六轮方案一定稿：tao transparent 的 blur-behind hack 会让圆角月牙缝成透底洞）+
// 窗口背景 = 实色-1 --n-g-a1-solid 单态（2026-10-01 弃云母，踩坑 071 四轮；
// fx-mica 双态机制 2026-10-02 已整层拆除）；
// 窗口描边分模式：实色 = 前端自绘 1px（::before）；云母 = 零绘制、**DWM 默认半透明边框独存**
// （2026-09-06 六轮用户改定：DWM 原生默认即半透明全窗交 DWM——原 #333 实心染色撤销；踩坑 063）；
// 圆角 8，最大化去圆角与描边；
// resize 判定在可视边缘内 8px（startResizeDragging）。
// 导入弹窗宿主：菜单 / 拖拽填 content.importDraft，ImportDialog 确认后刷新内容区。
// 全局键盘 / 侧栏收展动画 / 窗口几何持久化已拆 lib（useGlobalKeys / useSidebarAnim /
// windowState，2026-08-12，规范第 3 条行数控制）。
// 子窗口 query 分发（2026-09-06 插件窗轮）：?preview={assetKey} = 预览子窗口；
// ?plugin={id} = 插件详情子窗口（AnimatePluginWindow / McpPluginWindow；未知 id = 错误提示帧）——
// 子窗口只渲染各自面板（无三栏无弹窗），通知条锚点两子窗口同挂。
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
// @tauri-apps/api 2.11 的 window.d.ts 只 declare 未 export ResizeDirection（vue-tsc 落地发现，2026-08-12）——
// 本地按包内定义复刻（startResizeDragging 参数类型）
type ResizeDirection = "East" | "North" | "NorthEast" | "NorthWest" | "South" | "SouthEast" | "SouthWest" | "West";
import SidebarLeft from "./components/SidebarLeft.vue";
import SidebarRight from "./components/SidebarRight.vue";
import DockHost from "./components/DockHost.vue";
import SplitBar from "./components/SplitBar.vue";
import AppIcon from "./components/AppIcon.vue";
import PreviewWindow from "./components/PreviewWindow.vue";
import AnimatePluginWindow from "./components/plugins/AnimatePluginWindow.vue";
import McpPluginWindow from "./components/plugins/McpPluginWindow.vue";
import PluginListPanel from "./components/plugins/PluginListPanel.vue";
import SetupWizard from "./components/SetupWizard.vue";
import CreateLibraryModal from "./components/CreateLibraryModal.vue";
import MergeLibraryModal from "./components/MergeLibraryModal.vue";
import Checkbox from "./components/buttons/Checkbox.vue";
import ImportDialog from "./components/ImportDialog.vue";
import SequenceDialog from "./components/SequenceDialog.vue";
import FailedListDialog from "./components/FailedListDialog.vue";
import DuplicateDialog from "./components/DuplicateDialog.vue";
import DupScanDialog from "./components/DupScanDialog.vue";
import LinkManageDialog from "./components/LinkManageDialog.vue";
import RecycleConfirmDialog from "./components/RecycleConfirmDialog.vue";
import BlockManageDialog from "./components/BlockManageDialog.vue";
import InfoManageDialog from "./components/InfoManageDialog.vue";
import WsManageDialog from "./components/WsManageDialog.vue";
import GroupManageDialog from "./components/GroupManageDialog.vue";
import GroupEditDialog from "./components/GroupEditDialog.vue";
import PropsDialog from "./components/PropsDialog.vue";
import DescEditDialog from "./components/DescEditDialog.vue";
import ContextMenu from "./components/ContextMenu.vue";
import AppConfirm from "./components/AppConfirm.vue";
import ConfirmModal from "./components/modals/ConfirmModal.vue";
import OrphanInfosAlert from "./components/OrphanInfosAlert.vue";
import MovePanel from "./components/MovePanel.vue";
import DestTreeDialog from "./components/DestTreeDialog.vue";
import BatchRenameDialog from "./components/BatchRenameDialog.vue";
import ToastStack from "./components/ToastStack.vue";
import { useUiStore } from "./stores/ui";
import { useContentStore } from "./stores/content";
import { usePickerStore } from "./stores/picker";
import { useSelectionStore } from "./stores/selection";
import { useToastStore } from "./stores/toast";
import { useTabsStore } from "./stores/tabs";
import { useWsStore } from "./stores/ws";
import { initPreviewListeners } from "./stores/preview";
import { wizardState, reopenLastSession, createLibraryModalState, mergeLibraryModalState, deleteLibraryState, confirmDeleteLibrary, initOrphanInfosListener, renameLibraryState } from "./lib/libraryOps";
import { retargetLinkFlow } from "./lib/menuSpecs/workspace";
import { playAlert } from "./lib/sounds";
import { osFileDragHover, osFileDragHoverEnd, isOsDragOutActive } from "./lib/dragOperation";
import { closePluginPanel, pluginPanelState } from "./lib/pluginPanel";
import { dupScanState } from "./lib/dupScanOps";
import { useGlobalKeys } from "./lib/useGlobalKeys";
import { useSidebarAnim } from "./lib/useSidebarAnim";
import { useWindowStatePersistence } from "./lib/windowState";
import { initWsListeners } from "./lib/wsOps";

const ui = useUiStore();
const content = useContentStore();
const picker = usePickerStore();
const selection = useSelectionStore();
const toast = useToastStore();
const tabs = useTabsStore();
const ws = useWsStore();

// 断链链接目录判定（断链修复弹窗三触发源共用——选中 watch / openView 守卫 / 幽灵回退）：
// 父目录缓存条目 = 链接门且目标不可达
function isBrokenWsLink(rel: string): boolean {
  const l = ws.linkState(rel);
  return !!l?.isLink && !l.linkOk;
}

// 断链修复弹窗动作（2026-10-02 用户定）：主钮 = 重新定位（系统目录选择器重挂同 rel）；
// 次钮 = 删除链接（只删门进系统回收站，弹窗本身即提示不再二次确认）；取消 = 先不管
async function onBrokenLinkRetarget(): Promise<void> {
  const p = selection.brokenLinkPrompt;
  selection.brokenLinkPrompt = null;
  if (p) await retargetLinkFlow(p.rel);
}
function onBrokenLinkRemove(): void {
  const p = selection.brokenLinkPrompt;
  selection.brokenLinkPrompt = null;
  if (p) void ws.removeLink(p.rel).catch((e: unknown) => toast.notify({ message: String(e), icon: "fail" }));
}

// 侧栏 / dock 宿主引用：收展动画引擎（useSidebarAnim）逐帧写宽与灌宽的 DOM 锚点
const sidebarLeftRef = ref<InstanceType<typeof SidebarLeft>>();
const sidebarRightRef = ref<InstanceType<typeof SidebarRight>>();
const dockHostRef = ref<InstanceType<typeof DockHost>>();

const maximized = ref(false);
const appWindow = getCurrentWindow();
const webview = getCurrentWebviewWindow();

// 启动时未打开任何素材库 → 强制弹初次打开向导（38_向导稿，2.1；状态在 libraryOps.wizardState）
const wizardOpen = () => wizardState.open;

// 独立预览窗口（4.3 新窗口打开）：URL ?preview={assetKey} 进入，只渲染预览布局（无三栏、无导入弹窗）
const previewParam = new URLSearchParams(window.location.search).get("preview");
// 插件详情子窗口（2026-09-06 插件窗轮）：URL ?plugin={id} 进入（open_plugin_window 后端命令单例）
const pluginParam = new URLSearchParams(window.location.search).get("plugin");

// 全局键盘（lib/useGlobalKeys）：剪贴板导入 / 撤销重做 / 选择 / 操作快捷键；
// 子窗口（预览 / 插件详情）不挂主窗口快捷键（previewParam 闸复用——语义 = 子窗口参数）
useGlobalKeys({ previewParam: previewParam ?? pluginParam, needLibrarySetup: wizardOpen });
// 侧栏收展动画 rAF 引擎 + dockResizeObs 灌宽（lib/useSidebarAnim，机制注释随迁该文件）
useSidebarAnim({
  leftEl: () => sidebarLeftRef.value?.$el as HTMLElement | undefined,
  rightEl: () => sidebarRightRef.value?.$el as HTMLElement | undefined,
  dockEl: () => dockHostRef.value?.$el as HTMLElement | undefined,
});
// 主窗口几何持久化（lib/windowState；预览 / 插件详情子窗口不记录）
useWindowStatePersistence({ enabled: !previewParam && !pluginParam });

async function syncMaximized(): Promise<void> {
  maximized.value = await appWindow.isMaximized();
}

let resizeTimer: ReturnType<typeof setTimeout> | undefined;
function onResize(): void {
  // 拖动窗口期间高频触发，debounce 后再查窗口状态（IPC）
  clearTimeout(resizeTimer);
  resizeTimer = setTimeout(() => void syncMaximized(), 150);
}

// 向导完成后：关向导（标签页布局重置与全量刷新已收进 libraryOps.openLibrary/createLibrary）
function onWizardDone(): void {
  wizardState.open = false;
}

// 删除资源库确认 = 旧 09 稿 alert 侵入式语义：弹窗出现配提示音（旧 AppModal alert onMounted 口径，
// 随 ConfirmModal 换代收进 App 层——AppConfirm watch playAlert 同先例）
watch(
  () => deleteLibraryState.name,
  (n) => {
    if (n) playAlert();
  },
);

// 导入/导出失败清单 = alert error 侵入式语义：弹窗出现配提示音（ConfirmModal 不带音，
// 宿主 watch 补——§40 口径，与 deleteLibraryState 同先例）
watch([() => content.importFailed, () => content.exportFailed], ([i, e]) => {
  if (i || e) playAlert();
});

// 导入弹窗确认：关弹窗，先检测序列帧（命中追加 12_序列帧弹窗，4.1），导入走后台任务
//（Channel 流式，到一个显示一个，UI 不阻塞）
function onImportConfirm(opts: { structure: "flatten" | "keep"; source: "copy" | "move" }): void {
  const draft = content.importDraft;
  content.importDraft = null;
  if (draft) void content.probeSequences(draft, opts.structure, opts.source);
}

// 拖拽导入（PRD 05-01）：webview 拖放事件拿文件路径，落点在文件树 item 上则导入至该文件夹
async function onDrop(paths: string[], x: number, y: number): Promise<void> {
  if (wizardState.open || paths.length === 0 || content.importDraft) return;
  // 工作空间落点（2026-10-01 ws ↔ Explorer 互通，PRD 02-13：拖入一律复制——非破坏口径）：
  // 树行（data-folder-id = "ws:"+rel）/ 文件夹卡片（data-ws-folder = rel）/ 视口空白
  //（当前视口是 ws 且无其他落点）= 当前目录——命中即按路径复制进落点目录，不进库导入流
  //（ws 树行落点此前混进库导入流的隐患同修）；命中库树行 / 库卡片（非 ws: 值）不拦截，
  // 完全走既有库导入 / 拒入流
  const dropEl = document.elementFromPoint(x, y);
  const hitFolderId = dropEl?.closest("[data-folder-id]")?.getAttribute("data-folder-id") ?? null;
  const hitWsFolder = dropEl?.closest("[data-ws-folder]")?.getAttribute("data-ws-folder") ?? null;
  let wsDest: string | null = null;
  if (hitFolderId?.startsWith("ws:")) wsDest = hitFolderId.slice(3);
  else if (hitWsFolder !== null) wsDest = hitWsFolder;
  else if (hitFolderId === null && content.isWorkspaceView) wsDest = content.sidebarSelected.slice(3);
  if (wsDest !== null) {
    try {
      await ws.importPaths(paths, wsDest);
    } catch (e) {
      toast.notify({ message: String(e), icon: "fail" });
    }
    return;
  }
  // 只读视口拒入（2026-09-08 用户定，isNoImportView 统一判定；2026-09-09 收窄扩「全部」）：
  // 全部 / 最近使用 / 自动导入 / 缩略图 / 回收站不允许额外导入——拖入直接拒绝弹错误弹窗，
  // 走 classify / importDraft 前拦截（弹窗开关在 ui store：Ctrl+V 拒入同弹，lib 无权触达本地态）；
  // 未分类 = 候车区总入口放行（落点 root）
  if (content.isNoImportView) {
    ui.importRejectOpen = true;
    return;
  }
  try {
    const { kind } = await invoke<{ kind: "files" | "folder" | "mixed"; fileCount: number }>(
      "classify_paths",
      { paths },
    );
    const hit = document.elementFromPoint(x, y)?.closest("[data-folder-id]");
    const folderId = hit?.getAttribute("data-folder-id") ?? null;
    content.importDraft = {
      kind,
      paths,
      targetFolder: folderId ?? content.currentFolderId,
    };
  } catch (e) {
    console.error("拖拽导入失败：", e);
  }
}

let unlistenDragDrop: (() => void) | undefined;

onMounted(() => {
  void syncMaximized();
  void ui.initSettings(); // 全局 UI 设置加载 + 变更防抖写回（2026-07-28 持久化）
  window.addEventListener("resize", onResize);
  // 视图切换（6.1）：左栏选中项变化即清空素材选择；
  // 3.5 新增区同口径归位（2026-08-19 用户定：切视图再回来 = 新增项并入排序流）；
  // 顺带按 ws: 前缀归桶记录各模式停留视口（2026-09-19 用户定：库 / 工作空间切换各自恢复）；
  // 分段选择器跟随定位（2026-10-03 用户定：切标签页 / 分屏激活变化即把左栏定位到当前视口
  // 所属的库 / 工作空间视图——旧只记停留桶不同步模式，侧栏树与内容区各说各话）
  watch(() => content.sidebarSelected, (vk) => {
    selection.clear();
    content.clearArrivals();
    if (vk.startsWith("ws:")) {
      ui.lastWsView = vk;
      ui.leftMode = "workspace";
    } else {
      ui.lastLibView = vk;
      ui.leftMode = "library";
    }
  });
  // 幽灵视口自愈（2026-10-02 标签页 BUG 修复）：当前 ws 视口目录已不存在（lastWsView 跨空间
  // 重放的幽灵路径 / 外部删除）→ 回退到最近存活祖先；父级仍缺失 / 同断链则先爬开再回退
  //（断链父级会被 openView 守卫拦截，连环回退直至根；根缺失 = openView 同视口早退不成环，
  // root.exists 空态接管）。二轮补反馈（用户实机报「双击没有任何提示，以为 UI 卡住」）：
  // 断链 → 断链修复弹窗（缓存说没断实际断了的路径）；普通目录没了 → toast；顺带重扫父级翻正徽章
  watch(
    () => (content.sidebarSelected.startsWith("ws:") ? ws.missing[content.sidebarSelected.slice(3)] : undefined),
    (m) => {
      if (!m) return;
      const rel = content.sidebarSelected.slice(3);
      if (rel) {
        const link = ws.linkState(rel);
        if (link?.isLink) {
          if (selection.brokenLinkPrompt?.rel !== rel) selection.brokenLinkPrompt = { rel, name: rel.split("/").pop() ?? rel };
        } else {
          toast.notify({ message: "目录已不存在或已被移动，已回退到上级目录", icon: "fail" });
        }
      }
      void ws.refreshAround([rel]); // 重扫父级——断链条目陈旧挂链徽章随列举翻正（目标被删零事件场景）
      let parent = rel.includes("/") ? rel.slice(0, rel.lastIndexOf("/")) : "";
      while (parent && (ws.missing[parent] || isBrokenWsLink(parent))) {
        parent = parent.includes("/") ? parent.slice(0, parent.lastIndexOf("/")) : "";
      }
      tabs.openView(parent ? `ws:${parent}` : "ws:");
    },
  );
  // 选中断链即弹修复窗（2026-10-02 用户定）：单选且为断链链接目录 → brokenLinkPrompt
  //（导航拦截 openView 守卫 / 幽灵回退同弹一窗，三源按 rel 去重）
  watch(
    () => {
      if (selection.selected.size !== 1) return null;
      const key = selection.selected.values().next().value as string | undefined;
      if (!key?.startsWith("ws:")) return null;
      const rel = key.slice(3, key.indexOf("|") === -1 ? undefined : key.indexOf("|"));
      return isBrokenWsLink(rel) ? rel : null;
    },
    (rel) => {
      if (rel && selection.brokenLinkPrompt?.rel !== rel) selection.brokenLinkPrompt = { rel, name: rel.split("/").pop() ?? rel };
    },
  );
  void content.initInboxListener(); // 收件箱自动入库（3.2），App 存活期注册一次
  void content.initFpsHealListener(); // 存量视频 fps 自愈通知（2026-07-28 复验反馈修复）
  void content.initRecentListener(); // 最近使用热刷新（2026-07-29 复验反馈修复）
  void content.initCepListener(); // CEP 导出入库热刷新（8.1：WS 接收入库完成 → 全量对齐）
  void content.initMcpListener(); // MCP 写入热刷新（PRD 08-06：agent 写库 → 去抖全量对齐）
  initWsListeners(); // 工作空间磁盘实况（PRD 02 步骤 1）：fs-changed 去抖重扫 + 缩略图回填，幂等
  void initOrphanInfosListener(); // 无主资产警告（2026-09-02）——须在开库（reopenLastSession）前注册
  void initPreviewListeners(); // 预览就绪事件（PSD 高清 / 帧条渐进，4.3；扇出全部已建会话）
  void webview.onDragDropEvent((event) => {
    const p = event.payload;
    // 自身拖出窗口期（2026-10-01 走查：拖出错、未松手原路放回居然触发导入）——OLE DoDragDrop
    // 存续期间收到的 enter / over / drop 全是这条自己的拖：整体早退，OLE 侧报无放置 = 拖取消
    if (isOsDragOutActive()) return;
    // DragDropEvent 的 position 是**物理像素**（PhysicalPosition，webview.d.ts 类型实证），
    // elementFromPoint / getBoundingClientRect 是 CSS 逻辑像素——显示器缩放 >100% 时必须除
    // devicePixelRatio 换算，否则落点命中区整体偏移（踩坑 096：125% 缩放下侧边栏右侧误触发、
    // 视口右侧不触发；库导入树行落点同隐患同修）
    const scale = window.devicePixelRatio || 1;
    // OS 文件拖入悬停高亮（2026-10-01 走查：外部拖入无提示）——enter / over 实时解落点给
    // 树行 / 文件夹卡 / 视口高亮（dragOperation 同视觉槽）；leave / drop 清槽后走落盘流
    if (p.type === "enter" || p.type === "over") {
      osFileDragHover(p.position.x / scale, p.position.y / scale);
      return;
    }
    osFileDragHoverEnd();
    if (p.type === "drop") {
      void onDrop(p.paths, p.position.x / scale, p.position.y / scale);
    }
  }).then((fn) => {
    unlistenDragDrop = fn;
  });
  // 查询库状态：未打开时自动重开最近空间与库（2.1 空间化：reopenLastSession 三态——
  // ok 直接进；need-library 空间已开但无库，向导直进第二步；need-wizard 首次使用 / 重开失败，向导入口帧）。
  // 子窗口不弹向导（预览 = PreviewWindow 自行拉数据；插件详情 = 内容自拉，2026-09-06）
  invoke<unknown>("get_current_library")
    .then(async (lib) => {
      if (lib !== null) {
        void content.refresh();
        return;
      }
      if (previewParam || pluginParam) return;
      const r = await reopenLastSession();
      if (r === "ok") return;
      wizardState.open = true;
      wizardState.closable = false;
      wizardState.startStep = r === "need-library" ? 2 : 0;
    })
    .catch(() => {
      wizardState.open = true;
      wizardState.closable = false;
      wizardState.startStep = 0;
    });
});
onBeforeUnmount(() => {
  clearTimeout(resizeTimer);
  window.removeEventListener("resize", onResize);
  unlistenDragDrop?.();
});

// 可视窗口边缘 resize 判定（透明阴影带外缘的 tao 物理判定保留作补充）
const resizeZones: { dir: ResizeDirection; cls: string }[] = [
  { dir: "North", cls: "n" },
  { dir: "South", cls: "s" },
  { dir: "East", cls: "e" },
  { dir: "West", cls: "w" },
  { dir: "NorthEast", cls: "ne" },
  { dir: "NorthWest", cls: "nw" },
  { dir: "SouthEast", cls: "se" },
  { dir: "SouthWest", cls: "sw" },
];

function startResize(dir: ResizeDirection, e: MouseEvent): void {
  if (e.button !== 0) return;
  e.preventDefault();
  void appWindow.startResizeDragging(dir);
}

// 窗口置顶（右上角浮块，2026-08-08 收展定案：窗口控制组恒在窗口右上角，不随侧栏收展换宿主）
async function togglePin(): Promise<void> {
  ui.alwaysOnTop = !ui.alwaysOnTop;
  await appWindow.setAlwaysOnTop(ui.alwaysOnTop);
}
</script>

<template>
  <div class="window" :class="{ maximized, picking: picker.active }">
    <PreviewWindow v-if="previewParam" :asset-key="previewParam" />
    <!-- 插件详情子窗口（?plugin={id} 分发，?preview= 先例）：只渲染插件面板（无三栏无弹窗）；
         未知 id = 错误提示帧 -->
    <AnimatePluginWindow v-else-if="pluginParam === 'animate'" />
    <McpPluginWindow v-else-if="pluginParam === 'mcp'" />
    <div v-else-if="pluginParam" class="plugin-unknown">未知插件：{{ pluginParam }}</div>
    <!-- 子窗口的通知条挂点（6.0；主窗口通知族挂 DockHost 内统一锚点；预览 / 插件详情同挂） -->
    <div v-if="previewParam || pluginParam" class="notice-anchor">
      <ToastStack />
    </div>
    <template v-else>
      <!-- 选择模式目的横幅已迁入 DockHost 通知族统一锚点（2026-08-29 用户定：横幅在上、通知条在下） -->
      <!-- 侧栏收展（2026-08-08 定案 v6 锚点；2026-08-10 同步改 RO）：侧栏常驻挂载，收展宽度由
           App 层 rAF 驱动逐帧写入；dockview 网格由 dockResizeObs 在 RO 阶段以当帧最终宽同步
           （其自身 RO 回调被 rAF 包裹且持旧尺寸，同帧灌宽会被覆盖回上一帧）。
           锚点：收左栏 = 中间面板右缘钉死向左拉宽；收右栏反之。SplitBar 负 margin 零占位，
           v-if 瞬时不影响布局。左栏展开按钮在标签页行首（SidebarExpandButton），
           右栏"展开详情面板"在中间面板工具栏；窗口控制组为 App 层浮块（.win-controls-overlay），
           恒在窗口右上角、不参与收展 -->
      <SidebarLeft ref="sidebarLeftRef" />
      <SplitBar v-if="!ui.leftCollapsed" side="left" />
      <!-- 档案二级侧边栏已迁入标签页视口（ContentArea 内，2026-08-15 修正：标签栏不被推挤） -->
      <DockHost ref="dockHostRef" />
      <SplitBar v-if="!ui.rightCollapsed" side="right" />
      <SidebarRight ref="sidebarRightRef" />
      <!-- 窗口控制浮块（2026-08-08 定案恒在右上角；7.1 二十三轮贴稿 布局/主窗口/右侧边栏-窗口控制：
           4 钮满格 hit 区连续排（无容器底无间隙）；2026-10-01 用户修订 58×38 / 图标 14），
           hover 亮色/半透明-3 底、关闭 hover 状态/禁止底；置顶 = pin-on / 置顶中 = pin-off。
           关闭由 Rust 侧 CloseRequested 拦截转托盘（原 WindowControls 口径） -->
      <div v-if="!previewParam" class="win-controls-overlay">
        <button class="wc-btn" v-tip="'窗口置顶'" @click="togglePin">
          <AppIcon :name="ui.alwaysOnTop ? 'pin-off' : 'pin-on'" :size="14" />
        </button>
        <button class="wc-btn" v-tip="'最小化'" @click="appWindow.minimize()">
          <AppIcon name="minimize" :size="14" />
        </button>
        <button class="wc-btn" v-tip="'最大化'" @click="appWindow.toggleMaximize()">
          <AppIcon name="maximize" :size="14" />
        </button>
        <button class="wc-btn close" v-tip="'关闭'" @click="appWindow.close()">
          <AppIcon name="close-large" :size="14" />
        </button>
      </div>
    </template>
    <template v-if="!maximized">
      <div
        v-for="z in resizeZones"
        :key="z.cls"
        class="resize-zone"
        :class="z.cls"
        @mousedown="startResize(z.dir, $event)"
      />
    </template>
    <SetupWizard
      v-if="wizardState.open"
      :closable="wizardState.closable"
      :start-step="wizardState.startStep"
      @done="onWizardDone"
      @cancel="wizardState.open = false"
    />
    <!-- 菜单"新建资源库"（16 稿独立弹窗） -->
    <CreateLibraryModal v-if="createLibraryModalState.open" @close="createLibraryModalState.open = false" />
    <!-- 菜单"重命名当前资源库"（复用创建弹窗改名模式，2026-09-26） -->
    <CreateLibraryModal
      v-if="renameLibraryState.open && renameLibraryState.from"
      :rename-from="renameLibraryState.from"
      @close="renameLibraryState.open = false"
    />
    <!-- 菜单"合并资源库"（2026-09-08 稿；2026-09-26 功能轮接通：勾选项并入当前库） -->
    <MergeLibraryModal
      v-if="mergeLibraryModalState.open"
      :items="mergeLibraryModalState.items"
      @close="mergeLibraryModalState.open = false"
    />
    <!-- 删除资源库确认（08 稿 item 删除×；09 稿 alert 侵入式配提示音——watch 播音见上；
         孤儿素材复选清理；2026-09-20 R2 换代 ConfirmModal warning：确认钮 = 状态/警告底） -->
    <ConfirmModal
      v-if="deleteLibraryState.name"
      kind="warning"
      :title="`删除资源库「${deleteLibraryState.name}」`"
      confirm-label="删除"
      @confirm="confirmDeleteLibrary"
      @cancel="deleteLibraryState.name = null"
    >
      <p class="del-lib-text">库内文件夹、标签等数据将被删除且不可恢复。</p>
      <Checkbox
        v-if="deleteLibraryState.orphans > 0"
        v-model="deleteLibraryState.includeOrphans"
        class="del-lib-check"
        :label="`同时删除 ${deleteLibraryState.orphans} 个仅该库引用的素材文件`"
      />
      <p v-else class="del-lib-text">素材文件保留在素材库中。</p>
    </ConfirmModal>
    <ImportDialog
      v-else-if="content.importDraft"
      :draft="content.importDraft"
      @confirm="onImportConfirm"
      @cancel="content.importDraft = null"
    />
    <!-- 序列帧弹窗（12_序列帧弹窗）：09 确认后检测到序列帧时追加；取消 = 放弃整个导入 -->
    <SequenceDialog
      v-if="content.sequencePrompt"
      @confirm="content.confirmSequenceImport"
      @cancel="content.sequencePrompt = null"
    />
    <FailedListDialog
      v-if="content.importFailed"
      title="导入未完成"
      :failed="content.importFailed"
      @close="content.importFailed = null"
      @retry="content.retryFailedImport()"
    />
    <FailedListDialog
      v-if="content.exportFailed"
      title="导出未完成"
      :failed="content.exportFailed"
      @close="content.exportFailed = null"
      @retry="content.retryFailedExport()"
    />
    <!-- hash 冲突逐个询问（导入完成后弹出；10_重复添加提示） -->
    <DuplicateDialog v-if="content.importConflicts && content.importConflicts.length > 0" />
    <!-- 扫描重复文件（2.7 二轮，35 稿 + 合并前确认）：汉堡"查找" / 新增菜单触发 -->
    <DupScanDialog v-if="dupScanState.groups" />
    <!-- 链接管理弹窗（17 稿，6.2 三轮）：文件右键 Ctrl+J / 详情链接区块添加钮触发 -->
    <LinkManageDialog v-if="selection.linkManageDraft" />
    <!-- 回收站彻底删除确认窗（6.1 二轮）：虚拟片段引用警示，不可恢复无撤销 -->
    <RecycleConfirmDialog v-if="selection.recyclePrompt" />
    <!-- 区块管理弹窗（2026-08-14 区块子系统，30 稿）：SidebarLeft section-manage 按钮触发 -->
    <BlockManageDialog v-if="ui.blockManageOpen" @close="ui.blockManageOpen = false" />
    <!-- 信息查看管理弹窗（2026-09-26 三 tab 化重立，旧 22 稿 SectionEditPanel 后继）：
         右栏底部「信息查看管理」钮 / 视图菜单同名项触发 -->
    <InfoManageDialog v-if="ui.infoManageOpen" @close="ui.infoManageOpen = false" />
    <WsManageDialog v-if="ui.wsManageOpen" @close="ui.wsManageOpen = false" />
    <!-- 分组弹窗（3.5 二轮）：33 稿分组管理（工具栏分组管理钮 / 12 稿菜单「编辑分组」）/
         34 稿新建分组（12 稿菜单「新建分组」）；目标文件夹 = 打开时锁定的 groupDialogFolder -->
    <GroupManageDialog
      v-if="ui.groupManageOpen && ui.groupDialogFolder"
      :folder-id="ui.groupDialogFolder"
      @close="ui.groupManageOpen = false"
    />
    <GroupEditDialog
      v-if="ui.groupEditOpen && ui.groupDialogFolder"
      :folder-id="ui.groupDialogFolder"
      @close="ui.groupEditOpen = false"
    />
    <!-- 移动面板（3.4，29 稿）：移动文件 / 文件夹 / 档案共用，菜单 / Ctrl+Y 触发；
         须在 AppConfirm 之前——面板点击落点后弹确认窗，同 z-index 后渲染者在上 -->
    <MovePanel v-if="selection.moveDraft" />
    <!-- 目的树选择器（2026-10-03 库 ↔ 工作空间互通）：「导出到工作空间 / 导入到库」共用 -->
    <DestTreeDialog v-if="selection.destTreeDraft" />
    <!-- 批量重命名弹窗（3.4，37 稿）：多选「重命名」/ Ctrl+R 触发 -->
    <BatchRenameDialog v-if="selection.renameDraft" />
    <!-- 属性标签弹窗（6.1，32 稿）/ 编辑描述弹窗（31 稿）：详情面板自定义数据区块入口，
         文件与档案同做；须在 AppConfirm 之前——池内 × 删除弹确认窗，后渲染者在上 -->
    <PropsDialog v-if="ui.propsEdit" />
    <DescEditDialog v-if="ui.descEdit" />
    <!-- 全局右键菜单宿主（6.3：lib/contextMenus 单一实例，六上下文共用） -->
    <!-- 插件列表面板宿主（2026-09-06 插件窗轮：lib/pluginPanel 单一实例，三入口共用——
         菜单栏「插件」钮 / 工具栏 plugin 钮 / 预览标题栏 plugin 钮；预览子窗口同可开） -->
    <PluginListPanel
      v-if="pluginPanelState.open"
      :x="pluginPanelState.x"
      :y="pluginPanelState.y"
      :trigger="pluginPanelState.trigger"
      @close="closePluginPanel"
    />
    <ContextMenu />
    <!-- 通用确认弹窗（6.3：警示前置 / 删除文件夹 / 彻底删除二次确认） -->
    <AppConfirm v-if="selection.confirmPrompt" />
    <!-- 断链修复弹窗（junction 方向 b 二轮，2026-10-02 用户定：选中断链链接目录即弹——
         主钮 重新定位… / 次钮 删除链接 / 取消；三触发源：单选 / 导航拦截 / 幽灵回退） -->
    <ConfirmModal
      v-if="selection.brokenLinkPrompt"
      title="链接已失效"
      kind="brand"
      confirm-label="重新定位…"
      secondary-label="删除链接"
      cancel-label="取消"
      @confirm="onBrokenLinkRetarget"
      @secondary="onBrokenLinkRemove"
      @cancel="selection.brokenLinkPrompt = null"
    >
      链接「{{ selection.brokenLinkPrompt.name }}」指向的目标文件夹不存在或已被移动。可以重新定位到新位置，或删除这个链接（只删链接本身，进系统回收站可找回）。
    </ConfirmModal>
    <!-- 只读视口导入拒入（2026-09-08：与右键收窄同口径；2026-09-09 收窄扩「全部」五视口）——
         新弹窗定稿件 ConfirmModal danger（状态/禁止钮），纯告知两钮同义关闭；拖拽与 Ctrl+V 共用 -->
    <ConfirmModal
      v-if="ui.importRejectOpen"
      title="当前视口不允许导入"
      kind="danger"
      confirm-label="知道了"
      @confirm="ui.importRejectOpen = false"
      @cancel="ui.importRejectOpen = false"
    >
      「全部」「最近使用」「自动导入」「缩略图」「回收站」视口不允许额外导入素材，请切换到文件夹视口或「未分类」后再导入。
    </ConfirmModal>
    <!-- 无主资产警告弹窗（2026-09-02 开库自检，资产污染修复配套；内部 v-if 驱动） -->
    <OrphanInfosAlert />
  </div>
</template>

<style scoped>
/* 未知插件 id 错误提示帧（?plugin= 分发兜底）：一句提示居中，正文档三级 */
.plugin-unknown {
  flex: 1;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l3);
  user-select: none;
}
/* 删除资源库确认文案：正文常态色，限宽换行 */
.del-lib-text {
  margin: 0;
  max-width: 320px;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l2);
}
/* 孤儿素材复选框：与警示正文拉开 12（2026-08-13 用户：挨太近注意不到） */
.del-lib-check {
  margin-top: var(--n-space-12);
}
/* 主窗口壳（7.1 十七轮 · 主窗口骨架轮贴稿，布局/主窗口/主窗口.md）：transparent:false +
   WebView2 不透明实底（backgroundColor:#1F1F1F）+ shadow:true（tauri.conf）。
   2026-10-01 终局转向（踩坑 071 四轮，用户定案「动画优先于云母」）：弃云母——撤
   set_effects(Mica) + 撤整窗玻璃（ExtendFrame(-1)，云母载体）+ WS_CAPTION 全程保留，
   换回 DWM 最大化/还原过渡动画（动画依附 caption；按钮画布 = 玻璃，玻璃撤了 DWM
   原生按钮画不出）。窗口底色 = 基态实色-1 --n-g-a1-solid 单态（fx-mica 双态机制
   2026-10-02 已整层拆除）。
   圆角 = 前端自裁 8（最大化归零），不透明底下与 DWM 系统圆角同径无双裁透缝问题。
   窗口边框 = DWM 默认半透明独存、前端零绘制、不显式染色（2026-10-02 用户定
   「DWM 默认的边框不是半透明的么」——09-06 六轮口径；撤玻璃后默认边框正常画，
   25H2「默认停画」只是玻璃场景特例；picking 切染品牌色 ↔ 恢复 COLOR_NONE，
   set_main_border_picking）。 */
.window {
  position: relative;
  height: 100%;
  display: flex;
  gap: 0;
  padding: 0;
  background: var(--n-g-a1-solid);
  overflow: hidden;
  border-radius: var(--n-radius-8);
}
.window.maximized {
  border-radius: 0;
}
/* 窗口边框与 picking 品牌框 = DWM 染色线独存，前端零绘制（2026-10-02 用户定「边框和
   投影交给 DWM 就行」——10-01 弃云母时按 25H2「默认边框停画」（玻璃场景实证）错撤染色
   改前端自绘，撤玻璃后 DWM 恢复画默认边框，与 ::before 自绘叠成 2px 双层被实机抓出；
   DWM 线与 CSS 线并集必错位双层（四边 3vs2、角弧异径出缝），踩坑 063——纯 DWM = 唯一
   均匀解。最大化 DWM 本不画边框，无需前端处理。
   历史自绘实现（::before 1px --n-g6 / ::after 1px --n-brand）与云母期注释随本变更删除。 */
/* 窗口控制浮块（2026-08-08 定案）：恒在窗口右上角（标签栏行 / 右栏标题栏同一位），
   侧栏收展时它不参与布局变化——从根上消除窗口控制的跳变；
   7.1 二十三轮贴稿：浮块无 padding/gap，四钮连续排（右栏-窗口控制稿）；
   2026-10-01 用户修订：58×46 → 58×38、图标 18 → 14（标题栏降占位） */
.win-controls-overlay {
  position: absolute;
  top: 0;
  right: 0;
  height: 38px;
  display: flex;
  z-index: 60;
}
/* 窗口控制钮（稿；2026-10-01 用户修订：58×38——标题栏 46 → 38 降占位随行高，
   图标 18 → 14 同轮定）：满格 hit 区、无容器底；图标 14 亮色/二级；
   hover 亮色/半透明-3 底、关闭 hover 状态/禁止底（稿 1_hover 节）；过渡 150ms 通用档 */
.wc-btn {
  width: 58px;
  height: 38px;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  border: none;
  background: transparent;
  color: var(--n-l2);
  transition: background var(--n-dur-150) var(--n-ease-in-out);
}
.wc-btn:hover {
  background: var(--n-l-a3);
}
.wc-btn.close:hover {
  background: var(--n-danger);
}
/* 最右关闭钮贴窗口圆角（实色模式前端自裁 radius 8；最大化归零，同 .window 口径） */
.wc-btn.close {
  border-top-right-radius: var(--n-radius-8);
}
.window.maximized .wc-btn.close {
  border-top-right-radius: 0;
}
/* 可视边缘内 8px 的 resize 判定热区（startResizeDragging 手动发起） */
.resize-zone {
  position: absolute;
  z-index: 100;
}
.resize-zone.n {
  top: 0;
  left: 8px;
  right: 8px;
  height: 8px;
  cursor: n-resize;
}
.resize-zone.s {
  bottom: 0;
  left: 8px;
  right: 8px;
  height: 8px;
  cursor: s-resize;
}
/* 右缘热区收窄 8 → 3（2026-08-13 用户实机"详情面板滚轮无法选中"根因）：
   详情面板悬浮滚轮贴窗口右缘（与 8px 热区几乎全重叠）——鼠标从右缘外够滚轮时命中测试
   落在热区上，aside 的 mouseenter 不触发 → 滚轮保持透明 pointer-events:none →
   按下被热区抢走变窗口拉伸；收窄后滚轮左 3px 露出，hover 激活链恢复 */
.resize-zone.e {
  top: 8px;
  bottom: 8px;
  right: 0;
  width: 3px;
  cursor: e-resize;
}
.resize-zone.w {
  top: 0;
  bottom: 8px;
  left: 0;
  width: 8px;
  cursor: w-resize;
}
.resize-zone.ne {
  top: 0;
  right: 0;
  width: 8px;
  height: 8px;
  cursor: ne-resize;
}
.resize-zone.nw {
  top: 0;
  left: 0;
  width: 8px;
  height: 8px;
  cursor: nw-resize;
}
.resize-zone.se {
  bottom: 0;
  right: 0;
  width: 8px;
  height: 8px;
  cursor: se-resize;
}
.resize-zone.sw {
  bottom: 0;
  left: 0;
  width: 8px;
  height: 8px;
  cursor: sw-resize;
}
</style>
