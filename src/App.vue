<script setup lang="ts">
// 主窗口（03_UI 布局/01）：C1 底 / 圆角 8 / padding 0 / 间距 0 / 外边框 2px border-1。
// 全透明无边框窗口：最大化时去圆角与边框；resize 判定在可视边缘内 8px（startResizeDragging）。
// 导入弹窗宿主：菜单 / 拖拽填 content.importDraft，ImportDialog 确认后刷新内容区。
// 全局键盘 / 侧栏收展动画 / 窗口几何持久化已拆 lib（useGlobalKeys / useSidebarAnim /
// windowState，2026-08-12，规范第 3 条行数控制）。
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
import IconButton from "./components/IconButton.vue";
import WindowControls from "./components/WindowControls.vue";
import PreviewWindow from "./components/PreviewWindow.vue";
import SetupWizard from "./components/SetupWizard.vue";
import CreateLibraryModal from "./components/CreateLibraryModal.vue";
import AppModal from "./components/AppModal.vue";
import AppButton from "./components/AppButton.vue";
import ImportDialog from "./components/ImportDialog.vue";
import SequenceDialog from "./components/SequenceDialog.vue";
import ImportResultDialog from "./components/ImportResultDialog.vue";
import DuplicateDialog from "./components/DuplicateDialog.vue";
import LinkDialog from "./components/LinkDialog.vue";
import RecycleConfirmDialog from "./components/RecycleConfirmDialog.vue";
import ContextMenu from "./components/ContextMenu.vue";
import AppConfirm from "./components/AppConfirm.vue";
import FolderPickerDialog from "./components/FolderPickerDialog.vue";
import ToastStack from "./components/ToastStack.vue";
import { useUiStore } from "./stores/ui";
import { useContentStore } from "./stores/content";
import { useSelectionStore } from "./stores/selection";
import { usePreviewStore } from "./stores/preview";
import { ensureDocApps } from "./lib/docApps";
import { wizardState, reopenLastSession, createLibraryModalState, deleteLibraryState, confirmDeleteLibrary } from "./lib/libraryOps";
import { useGlobalKeys } from "./lib/useGlobalKeys";
import { useSidebarAnim } from "./lib/useSidebarAnim";
import { useWindowStatePersistence } from "./lib/windowState";

const ui = useUiStore();
const content = useContentStore();
const selection = useSelectionStore();
const preview = usePreviewStore();

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

// 全局键盘（lib/useGlobalKeys）：剪贴板导入 / 撤销重做 / 选择 / 操作快捷键
useGlobalKeys({ previewParam, needLibrarySetup: wizardOpen });
// 侧栏收展动画 rAF 引擎 + dockResizeObs 灌宽（lib/useSidebarAnim，机制注释随迁该文件）
useSidebarAnim({
  leftEl: () => sidebarLeftRef.value?.$el as HTMLElement | undefined,
  rightEl: () => sidebarRightRef.value?.$el as HTMLElement | undefined,
  dockEl: () => dockHostRef.value?.$el as HTMLElement | undefined,
});
// 主窗口几何持久化（lib/windowState；预览子窗口不记录）
useWindowStatePersistence({ enabled: !previewParam });

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
  void ensureDocApps(); // 新建文档软件检测（1.3：每次打开 UI 监测，07 新增菜单"新建文档"组驱动）
  window.addEventListener("resize", onResize);
  // 视图切换（6.1）：左栏选中项变化即清空素材选择
  watch(() => content.sidebarSelected, () => selection.clear());
  void content.initInboxListener(); // 收件箱自动入库（3.2），App 存活期注册一次
  void content.initFpsHealListener(); // 存量视频 fps 自愈通知（2026-07-28 复验反馈修复）
  void content.initRecentListener(); // 最近使用热刷新（2026-07-29 复验反馈修复）
  void preview.initPreviewListener(); // 预览就绪事件（PSD 高清 / 帧条渐进，4.3）
  void webview.onDragDropEvent((event) => {
    if (event.payload.type === "drop") {
      void onDrop(event.payload.paths, event.payload.position.x, event.payload.position.y);
    }
  }).then((fn) => {
    unlistenDragDrop = fn;
  });
  // 查询库状态：未打开时自动重开最近空间与库（2.1 空间化：reopenLastSession 三态——
  // ok 直接进；need-library 空间已开但无库，向导直进第二步；need-wizard 首次使用 / 重开失败，向导入口帧）。
  // 预览窗口不弹向导（PreviewWindow 自行拉数据）
  invoke<unknown>("get_current_library")
    .then(async (lib) => {
      if (lib !== null) {
        void content.refresh();
        return;
      }
      if (previewParam) return;
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
  <div class="window" :class="{ maximized }">
    <PreviewWindow v-if="previewParam" :asset-key="previewParam" />
    <!-- 预览子窗口的通知条挂点（6.0；主窗口挂在 DockHost 内） -->
    <ToastStack v-if="previewParam" />
    <template v-else>
      <!-- 侧栏收展（2026-08-08 定案 v6 锚点；2026-08-10 同步改 RO）：侧栏常驻挂载，收展宽度由
           App 层 rAF 驱动逐帧写入；dockview 网格由 dockResizeObs 在 RO 阶段以当帧最终宽同步
           （其自身 RO 回调被 rAF 包裹且持旧尺寸，同帧灌宽会被覆盖回上一帧）。
           锚点：收左栏 = 中间面板右缘钉死向左拉宽；收右栏反之。SplitBar 负 margin 零占位，
           v-if 瞬时不影响布局。左栏展开按钮在标签页行首（SidebarExpandButton），
           右栏"展开详情面板"在中间面板工具栏；窗口控制组为 App 层浮块（.win-controls-overlay），
           恒在窗口右上角、不参与收展 -->
      <SidebarLeft ref="sidebarLeftRef" />
      <SplitBar v-if="!ui.leftCollapsed" side="left" />
      <DockHost ref="dockHostRef" />
      <SplitBar v-if="!ui.rightCollapsed" side="right" />
      <SidebarRight ref="sidebarRightRef" />
      <div v-if="!previewParam" class="win-controls-overlay">
        <IconButton
          :name="ui.alwaysOnTop ? 'window-pin-off' : 'window-pin-on'"
          title="窗口置顶"
          @click="togglePin"
        />
        <WindowControls />
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
    <!-- 删除资源库确认（08 稿 item 删除×；alert 侵入式配提示音） -->
    <AppModal
      v-if="deleteLibraryState.name"
      variant="alert"
      alert-icon="warning"
      title="删除资源库"
      @close="deleteLibraryState.name = null"
    >
      <p class="del-lib-text">
        删除资源库「{{ deleteLibraryState.name }}」？库内文件夹、标签等数据将被删除且不可恢复；素材文件保留在素材库中。
      </p>
      <template #actions>
        <AppButton label="取消" @click="deleteLibraryState.name = null" />
        <AppButton label="删除" variant="primary" @click="confirmDeleteLibrary" />
      </template>
    </AppModal>
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
    <ImportResultDialog
      v-if="content.importFailed"
      :failed="content.importFailed"
      @close="content.importFailed = null"
    />
    <!-- hash 冲突逐个询问（导入完成后弹出；10_重复添加提示） -->
    <DuplicateDialog v-if="content.importConflicts && content.importConflicts.length > 0" />
    <!-- 关联 / 副本弹窗（6.1 C5）：Ctrl+V(copy) / Ctrl+拖拽落文件夹 / 右键"粘贴"触发 -->
    <LinkDialog v-if="selection.linkDraft" />
    <!-- 回收站彻底删除确认窗（6.1 二轮）：虚拟片段引用警示，不可恢复无撤销 -->
    <RecycleConfirmDialog v-if="selection.recyclePrompt" />
    <!-- 全局右键菜单宿主（6.3：lib/contextMenus 单一实例，六上下文共用） -->
    <ContextMenu />
    <!-- 通用确认弹窗（6.3：警示前置 / 删除文件夹 / 彻底删除二次确认） -->
    <AppConfirm v-if="selection.confirmPrompt" />
    <!-- 文件夹选择弹窗（6.3"添加至文件夹"） -->
    <FolderPickerDialog v-if="selection.folderPickPrompt" />
  </div>
</template>

<style scoped>
/* 删除资源库确认文案：正文常态色，限宽换行 */
.del-lib-text {
  margin: 0;
  max-width: 320px;
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-normal);
}
/* 不透明窗口 + 原生装饰（DWM 阴影）：容器贴满窗口；圆角 8 与 DWM 一致。
   外框 = DWM 1px 边框染 亮-选中 实色（lib.rs DWMWA_BORDER_COLOR，2026-08-08 用户定；
   前端不自绘——自绘与 DWM 叠加变粗难看，2026-07-23 实测） */
.window {
  position: relative;
  height: 100%;
  display: flex;
  gap: 0;
  padding: 0;
  background: var(--c1);
  overflow: hidden;
  border-radius: var(--radius-8);
}
.window.maximized {
  border-radius: 0;
}
/* 窗口控制浮块（2026-08-08 定案）：恒在窗口右上角（标签栏行 / 右栏标题栏同一位），
   侧栏收展时它不参与布局变化——从根上消除窗口控制的跳变 */
.win-controls-overlay {
  position: absolute;
  top: 0;
  right: 0;
  height: 46px;
  display: flex;
  align-items: center;
  gap: var(--space-4);
  padding: 0 var(--space-12);
  z-index: 60;
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
.resize-zone.e {
  top: 8px;
  bottom: 8px;
  right: 0;
  width: 8px;
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
