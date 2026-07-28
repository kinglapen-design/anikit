<script setup lang="ts">
// 主窗口（03_UI 布局/01）：C1 底 / 圆角 8 / padding 0 / 间距 0 / 外边框 2px border-1。
// 全透明无边框窗口：最大化时去圆角与边框；resize 判定在可视边缘内 8px（startResizeDragging）。
// 导入弹窗宿主：菜单 / 拖拽填 content.importDraft，ImportDialog 确认后刷新内容区。
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow, type ResizeDirection } from "@tauri-apps/api/window";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import SidebarLeft from "./components/SidebarLeft.vue";
import SidebarRight from "./components/SidebarRight.vue";
import ContentArea from "./components/ContentArea.vue";
import SplitBar from "./components/SplitBar.vue";
import PreviewWindow from "./components/PreviewWindow.vue";
import LibrarySetupModal from "./components/LibrarySetupModal.vue";
import ImportDialog from "./components/ImportDialog.vue";
import SequenceDialog from "./components/SequenceDialog.vue";
import ImportResultDialog from "./components/ImportResultDialog.vue";
import DuplicateDialog from "./components/DuplicateDialog.vue";
import LinkDialog from "./components/LinkDialog.vue";
import DeleteConfirmDialog from "./components/DeleteConfirmDialog.vue";
import ToastStack from "./components/ToastStack.vue";
import { useUiStore } from "./stores/ui";
import { useContentStore } from "./stores/content";
import { useSelectionStore } from "./stores/selection";
import { usePreviewStore } from "./stores/preview";
import { useUndoStore } from "./stores/undo";
import { copySelection, deletePlacements, pasteClipboard, pasteDuplicate } from "./lib/placementOps";

const ui = useUiStore();
const content = useContentStore();
const selection = useSelectionStore();
const preview = usePreviewStore();
const undo = useUndoStore();
const maximized = ref(false);
const appWindow = getCurrentWindow();
const webview = getCurrentWebviewWindow();

// 启动时未打开任何素材库 → 强制弹"创建/打开资源库"模态（PRD 01-1）
const needLibrarySetup = ref(false);

// 独立预览窗口（4.3 新窗口打开）：URL ?preview={assetKey} 进入，只渲染预览布局（无三栏、无导入弹窗）
const previewParam = new URLSearchParams(window.location.search).get("preview");

async function syncMaximized(): Promise<void> {
  maximized.value = await appWindow.isMaximized();
}

let resizeTimer: ReturnType<typeof setTimeout> | undefined;
function onResize(): void {
  // 拖动窗口期间高频触发，debounce 后再查窗口状态（IPC）
  clearTimeout(resizeTimer);
  resizeTimer = setTimeout(() => void syncMaximized(), 150);
}

// 库进入打开状态后：关模态 + 拉真实数据
async function onLibraryReady(): Promise<void> {
  needLibrarySetup.value = false;
  await content.refresh();
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
  if (needLibrarySetup.value || paths.length === 0 || content.importDraft) return;
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

// 剪贴板导入（PRD 05-01；2026-07-27 扩：图片 + 文件列表，恒 Copy）；剪贴板无图无文件属常态，静默降级
// 6.1 仲裁（2026-07-28 定案）：内部剪贴板非空时 Ctrl+V 归 onOperationKeys（关联弹窗 / 移动），
// 本函数只处理"内部剪贴板为空"的系统剪贴板导入；Ctrl+Shift+V = 粘贴为副本快捷键，不走导入
async function onPaste(e: KeyboardEvent): Promise<void> {
  if (!(e.ctrlKey || e.metaKey) || e.key.toLowerCase() !== "v") return;
  if (e.shiftKey || selection.clipboard) return;
  const t = e.target as HTMLElement;
  if (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable) return;
  if (needLibrarySetup.value || content.importDraft) return;
  try {
    await content.startImportClipboard(content.currentFolderId);
  } catch (err) {
    console.debug("剪贴板导入不可用：", err);
  }
}

// 撤销 / 重做（6.0）：Ctrl+Z / Ctrl+Shift+Z；输入框内不拦截（保留文本编辑自身的撤销）
function onUndoRedo(e: KeyboardEvent): void {
  if (!(e.ctrlKey || e.metaKey) || e.key.toLowerCase() !== "z") return;
  const t = e.target as HTMLElement;
  if (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable) return;
  e.preventDefault();
  if (e.shiftKey) {
    void undo.redo();
  } else {
    void undo.undo();
  }
}

// 选择快捷键（6.1，2026-07-28 用户定案）：Ctrl+A 全选素材卡片 / Esc 清空选择。
// 与 onPaste / onUndoRedo 同口径：输入框内不拦截；预览打开 / 预览子窗口不抢（PreviewPanel 自行处理 Esc 等）；
// 菜单（AppMenu）/ 悬停大图浮层（HoverZoomOverlay）打开时让位——两者各自捕获 Esc 关闭，此刻不清选择
function onSelectionKeys(e: KeyboardEvent): void {
  const t = e.target as HTMLElement;
  if (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable) return;
  if (previewParam || preview.assetKey || needLibrarySetup.value) return;
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "a") {
    e.preventDefault();
    selection.selectAll();
    return;
  }
  if (e.key === "Escape" && !document.querySelector(".app-menu, .zoom-mask")) {
    selection.clear();
  }
}

// 操作快捷键（6.1 阶段 C，2026-07-28 用户定案，与 onSelectionKeys 同口径）：
// F2 = 重命名当前单选项（卡片就地重命名；多选不触发；文件夹重命名不在本轮——后端无 rename_folder）；
// Del = 删除选中项（两段式：先干跑，needConfirm 弹询问窗「移入回收站 / 保留为未分类」）；
// Ctrl+C / Ctrl+X = 写内部剪贴板（仅素材）；Ctrl+V 仲裁（内部非空 → copy 弹关联弹窗 / cut 移动；
// 空 → onPaste 系统剪贴板导入）；Ctrl+Shift+V = 直接副本不弹窗（名称同源）。
// 快速访问视图不作粘贴目标（toast 提示）；菜单 / 浮层 / 弹窗打开时让位
function onOperationKeys(e: KeyboardEvent): void {
  const t = e.target as HTMLElement;
  if (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable) return;
  if (previewParam || preview.assetKey || needLibrarySetup.value) return;
  if (document.querySelector(".app-menu, .zoom-mask, .link-mask, .del-mask")) return;
  if (e.key === "F2") {
    if (selection.count === 1) {
      e.preventDefault();
      selection.renamingKey = selection.first;
    }
    return;
  }
  if (e.key === "Delete") {
    if (selection.count > 0) {
      e.preventDefault();
      void deletePlacements(selection.selectedRefs());
    }
    return;
  }
  if (!(e.ctrlKey || e.metaKey)) return;
  const k = e.key.toLowerCase();
  if (k === "c" || k === "x") {
    if (selection.count > 0) {
      e.preventDefault();
      copySelection(k === "c" ? "copy" : "cut");
    }
    return;
  }
  if (k === "v" && selection.clipboard) {
    e.preventDefault();
    if (e.shiftKey) void pasteDuplicate();
    else void pasteClipboard();
  }
}

let unlistenDragDrop: (() => void) | undefined;
let unlistenMoved: (() => void) | undefined;
let unlistenResized: (() => void) | undefined;

// 主窗口几何持久化（2026-07-28 用户定："用户设置要持久化，包括窗口大小，位置"）：
// resize/move 高频事件防抖 500ms 整体写回全局 config（settings.window 子树）；
// 仅主窗口记录（预览子窗口不记录不恢复）；最大化时后端只刷标志位、几何保留还原尺寸
let winStateTimer: ReturnType<typeof setTimeout> | undefined;
function scheduleSaveWindowState(): void {
  clearTimeout(winStateTimer);
  winStateTimer = setTimeout(() => void saveWindowState(), 500);
}
async function saveWindowState(): Promise<void> {
  try {
    const [pos, size, maximized] = await Promise.all([
      appWindow.outerPosition(),
      appWindow.outerSize(),
      appWindow.isMaximized(),
    ]);
    await invoke("save_window_state", {
      state: { x: pos.x, y: pos.y, width: size.width, height: size.height, maximized },
    });
  } catch (e) {
    console.warn("窗口几何写回失败（静默降级）：", e);
  }
}

onMounted(() => {
  void syncMaximized();
  void ui.initSettings(); // 全局 UI 设置加载 + 变更防抖写回（2026-07-28 持久化）
  window.addEventListener("resize", onResize);
  window.addEventListener("keydown", onPaste);
  window.addEventListener("keydown", onUndoRedo);
  window.addEventListener("keydown", onSelectionKeys);
  window.addEventListener("keydown", onOperationKeys);
  // 视图切换（6.1）：左栏选中项变化即清空素材选择
  watch(() => content.sidebarSelected, () => selection.clear());
  void content.initInboxListener(); // 收件箱自动入库（3.2），App 存活期注册一次
  void content.initFpsHealListener(); // 存量视频 fps 自愈通知（2026-07-28 复验反馈修复）
  void preview.initPreviewListener(); // 预览就绪事件（PSD 高清 / 帧条渐进，4.3）
  if (!previewParam) {
    // 主窗口几何监听（预览子窗口不记录）
    void appWindow.onMoved(() => scheduleSaveWindowState()).then((fn) => (unlistenMoved = fn));
    void appWindow.onResized(() => scheduleSaveWindowState()).then((fn) => (unlistenResized = fn));
  }
  void webview.onDragDropEvent((event) => {
    if (event.payload.type === "drop") {
      void onDrop(event.payload.paths, event.payload.position.x, event.payload.position.y);
    }
  }).then((fn) => {
    unlistenDragDrop = fn;
  });
  // 查询库状态失败按"未打开"处理，让用户能经模态重试（预览窗口不弹库模态，PreviewWindow 自行拉数据）
  invoke<unknown>("get_current_library")
    .then((lib) => {
      needLibrarySetup.value = lib === null && !previewParam;
      if (lib !== null) void content.refresh();
    })
    .catch(() => {
      needLibrarySetup.value = true;
    });
});
onBeforeUnmount(() => {
  clearTimeout(resizeTimer);
  clearTimeout(winStateTimer);
  window.removeEventListener("resize", onResize);
  window.removeEventListener("keydown", onPaste);
  window.removeEventListener("keydown", onUndoRedo);
  window.removeEventListener("keydown", onSelectionKeys);
  window.removeEventListener("keydown", onOperationKeys);
  unlistenDragDrop?.();
  unlistenMoved?.();
  unlistenResized?.();
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
</script>

<template>
  <div class="window" :class="{ maximized }">
    <PreviewWindow v-if="previewParam" :asset-key="previewParam" />
    <!-- 预览子窗口的通知条挂点（6.0；主窗口挂在 ContentArea 内） -->
    <ToastStack v-if="previewParam" />
    <template v-else>
      <SidebarLeft />
      <SplitBar v-if="!ui.leftCollapsed" side="left" />
      <ContentArea />
      <SplitBar v-if="!ui.rightCollapsed" side="right" />
      <SidebarRight />
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
    <LibrarySetupModal v-if="needLibrarySetup" @done="onLibraryReady" />
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
    <!-- 删除两段式询问窗（6.1 C1）：needConfirm 时弹出「移入回收站 / 保留为未分类」 -->
    <DeleteConfirmDialog v-if="selection.deletePrompt" />
  </div>
</template>

<style scoped>
/* 不透明窗口 + 原生装饰（DWM 阴影 / DWM 默认 1px 边框）：容器贴满窗口；
   圆角 8 与 DWM 一致。主窗口不自绘外边框（2026-07-23 实测：自绘 2px 与 DWM 1px
   重叠成 3px、四角圆角错开有空余，放弃——2px 外边框仅二级窗口 / 模态 / 菜单） */
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
  top: 8px;
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
