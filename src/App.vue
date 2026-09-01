<script setup lang="ts">
// 主窗口（7.1 十七轮 · 主窗口骨架轮贴稿）：transparent:false + WebView2 背景透明
// （063 六轮方案一定稿：tao transparent 的 blur-behind hack 会让圆角月牙缝成透底洞）+
// 运行时按系统版本云母/实色双态（fx-mica = 半透明-1 底透云母；基态 = 实色-1 回退）；
// 窗口描边分模式：实色 = 前端自绘 1px（::before）；云母 = 零绘制、DWM 染画独存
// （六轮实测定稿，踩坑 063）；圆角 8，最大化去圆角与描边；
// resize 判定在可视边缘内 8px（startResizeDragging）。
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
import AppIcon from "./components/AppIcon.vue";
import PreviewWindow from "./components/PreviewWindow.vue";
import SetupWizard from "./components/SetupWizard.vue";
import CreateLibraryModal from "./components/CreateLibraryModal.vue";
import AppModal from "./components/AppModal.vue";
import AppButton from "./components/AppButton.vue";
import AppCheckbox from "./components/AppCheckbox.vue";
import ImportDialog from "./components/ImportDialog.vue";
import SequenceDialog from "./components/SequenceDialog.vue";
import FailedListDialog from "./components/FailedListDialog.vue";
import DuplicateDialog from "./components/DuplicateDialog.vue";
import DupScanDialog from "./components/DupScanDialog.vue";
import LinkManageDialog from "./components/LinkManageDialog.vue";
import RecycleConfirmDialog from "./components/RecycleConfirmDialog.vue";
import BlockManageDialog from "./components/BlockManageDialog.vue";
import GroupManageDialog from "./components/GroupManageDialog.vue";
import GroupEditDialog from "./components/GroupEditDialog.vue";
import PropsDialog from "./components/PropsDialog.vue";
import DescEditDialog from "./components/DescEditDialog.vue";
import ContextMenu from "./components/ContextMenu.vue";
import AppConfirm from "./components/AppConfirm.vue";
import OrphanInfosAlert from "./components/OrphanInfosAlert.vue";
import MovePanel from "./components/MovePanel.vue";
import BatchRenameDialog from "./components/BatchRenameDialog.vue";
import ToastStack from "./components/ToastStack.vue";
import { useUiStore } from "./stores/ui";
import { useContentStore } from "./stores/content";
import { usePickerStore } from "./stores/picker";
import { useSelectionStore } from "./stores/selection";
import { usePreviewStore } from "./stores/preview";
import { wizardState, reopenLastSession, createLibraryModalState, deleteLibraryState, confirmDeleteLibrary, initOrphanInfosListener } from "./lib/libraryOps";
import { dupScanState } from "./lib/dupScanOps";
import { useGlobalKeys } from "./lib/useGlobalKeys";
import { useSidebarAnim } from "./lib/useSidebarAnim";
import { useWindowStatePersistence } from "./lib/windowState";

const ui = useUiStore();
const content = useContentStore();
const picker = usePickerStore();
const selection = useSelectionStore();
const preview = usePreviewStore();

// 侧栏 / dock 宿主引用：收展动画引擎（useSidebarAnim）逐帧写宽与灌宽的 DOM 锚点
const sidebarLeftRef = ref<InstanceType<typeof SidebarLeft>>();
const sidebarRightRef = ref<InstanceType<typeof SidebarRight>>();
const dockHostRef = ref<InstanceType<typeof DockHost>>();

const maximized = ref(false);
const appWindow = getCurrentWindow();
const webview = getCurrentWebviewWindow();

// 窗口背景效果模式（7.1 十七轮 · 主窗口骨架轮）：Win11 22H2+ = mica（云母透底），
// 其余 = solid（实色-1 --n-g-a1-solid 回退）。基态样式即 solid 观感，查询返回 mica
// 才单向切换——防首帧半透明闪。主窗口与预览子窗口同此一处（.window 为共同根）。
const fxMode = ref<"solid" | "mica">("solid");

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
  // 查询窗口背景效果模式（返回 mica 才切半透明底；失败保持 solid 实色）
  void invoke<string>("get_window_fx_mode")
    .then((m) => {
      if (m === "mica") fxMode.value = "mica";
    })
    .catch(() => {});
  void ui.initSettings(); // 全局 UI 设置加载 + 变更防抖写回（2026-07-28 持久化）
  window.addEventListener("resize", onResize);
  // 视图切换（6.1）：左栏选中项变化即清空素材选择；
  // 3.5 新增区同口径归位（2026-08-19 用户定：切视图再回来 = 新增项并入排序流）
  watch(() => content.sidebarSelected, () => {
    selection.clear();
    content.clearArrivals();
  });
  void content.initInboxListener(); // 收件箱自动入库（3.2），App 存活期注册一次
  void content.initFpsHealListener(); // 存量视频 fps 自愈通知（2026-07-28 复验反馈修复）
  void content.initRecentListener(); // 最近使用热刷新（2026-07-29 复验反馈修复）
  void content.initCepListener(); // CEP 导出入库热刷新（8.1：WS 接收入库完成 → 全量对齐）
  void initOrphanInfosListener(); // 无主资产警告（2026-09-02）——须在开库（reopenLastSession）前注册
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
  <div class="window" :class="{ maximized, picking: picker.active, 'fx-mica': fxMode === 'mica' }">
    <PreviewWindow v-if="previewParam" :asset-key="previewParam" />
    <!-- 预览子窗口的通知条挂点（6.0；主窗口挂在 DockHost 内） -->
    <!-- 预览子窗口的通知条挂点（6.0；主窗口通知族挂 DockHost 内统一锚点） -->
    <div v-if="previewParam" class="notice-anchor">
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
           4 钮 58×46 满格 hit 区连续排（无容器底无间隙），图标 18 亮色/二级，
           hover 亮色/半透明-3 底、关闭 hover 状态/禁止底；置顶 = pin-on / 置顶中 = pin-off。
           关闭由 Rust 侧 CloseRequested 拦截转托盘（原 WindowControls 口径） -->
      <div v-if="!previewParam" class="win-controls-overlay">
        <button class="wc-btn" v-tip="'窗口置顶'" @click="togglePin">
          <AppIcon :name="ui.alwaysOnTop ? 'pin-off' : 'pin-on'" :size="18" />
        </button>
        <button class="wc-btn" v-tip="'最小化'" @click="appWindow.minimize()">
          <AppIcon name="minimize" :size="18" />
        </button>
        <button class="wc-btn" v-tip="'最大化'" @click="appWindow.toggleMaximize()">
          <AppIcon name="maximize" :size="18" />
        </button>
        <button class="wc-btn close" v-tip="'关闭'" @click="appWindow.close()">
          <AppIcon name="close-large" :size="18" />
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
    <!-- 删除资源库确认（08 稿 item 删除×；09 稿 alert 侵入式配提示音；孤儿素材复选清理） -->
    <AppModal
      v-if="deleteLibraryState.name"
      variant="alert"
      alert-icon="warning"
      :title="`删除资源库「${deleteLibraryState.name}」`"
      @close="deleteLibraryState.name = null"
    >
      <p class="del-lib-text">库内文件夹、标签等数据将被删除且不可恢复。</p>
      <AppCheckbox
        v-if="deleteLibraryState.orphans > 0"
        v-model="deleteLibraryState.includeOrphans"
        class="del-lib-check"
        :label="`同时删除 ${deleteLibraryState.orphans} 个仅该库引用的素材文件`"
      />
      <p v-else class="del-lib-text">素材文件保留在素材库中。</p>
      <template #actions>
        <AppButton label="删除" variant="primary" @click="confirmDeleteLibrary" />
        <AppButton label="取消" @click="deleteLibraryState.name = null" />
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
    <!-- 批量重命名弹窗（3.4，37 稿）：多选「重命名」/ Ctrl+R 触发 -->
    <BatchRenameDialog v-if="selection.renameDraft" />
    <!-- 属性标签弹窗（6.1，32 稿）/ 编辑描述弹窗（31 稿）：详情面板自定义数据区块入口，
         文件与档案同做；须在 AppConfirm 之前——池内 × 删除弹确认窗，后渲染者在上 -->
    <PropsDialog v-if="ui.propsEdit" />
    <DescEditDialog v-if="ui.descEdit" />
    <!-- 全局右键菜单宿主（6.3：lib/contextMenus 单一实例，六上下文共用） -->
    <ContextMenu />
    <!-- 通用确认弹窗（6.3：警示前置 / 删除文件夹 / 彻底删除二次确认） -->
    <AppConfirm v-if="selection.confirmPrompt" />
    <!-- 无主资产警告弹窗（2026-09-02 开库自检，资产污染修复配套；内部 v-if 驱动） -->
    <OrphanInfosAlert />
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
/* 孤儿素材复选框：与警示正文拉开 12（2026-08-13 用户：挨太近注意不到） */
.del-lib-check {
  margin-top: var(--space-12);
}
/* 主窗口壳（7.1 十七轮 · 主窗口骨架轮贴稿，布局/主窗口/主窗口.md）：transparent:false +
   WebView2 背景透明（backgroundColor:#00000000，Mica 由 DWM 在表面下合成，不需要 tao
   transparent——其 blur-behind hack 会让圆角月牙缝 alpha=0 透底，063 六轮方案一）+
   运行时按系统版本云母/实色双态——基态 = 实色-1 --n-g-a1-solid 回退底，fx-mica = 全透明
   （云母直呈）；大阴影 = DWM shadow（tauri.conf shadow:true）。
   圆角分模式（2026-08-30 实机 BUG 后改定）：实色 = 前端自裁 8（最大化归零）；云母 = DWM
   系统圆角独裁、前端 radius 0 不裁——双层裁切的 AA 路径差在四角透出云母成缝隙（踩坑 063）。
   窗口边框分模式同色（通用/六级实心 --n-g6，2026-08-30 用户定）：云母 = DWM 染画独存
   （lib.rs DWMWA_BORDER_COLOR #333333），前端 CSS 零绘制——六轮实测（win-border-measure）：
   CSS 线与 DWM 线的并集左右/角弧必错位分离（四边 3vs2、角缝），五轮遮罩环反成分缝
   之源，纯 DWM = 唯一均匀解（踩坑 063）；实色 = 前端 ::before 自绘（Win10 DWM 无边框
   可画）。选择模式进出 = set_main_border_picking 切染品牌色（§19）。 */
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
.window.fx-mica {
  /* 云母模式 = 窗口底全透明，DWM 云母直接呈现（2026-08-30 用户定：稿 93% 是 figma
     模拟手法，工程里云母上不叠色；实色区由各面板自画——内容容器/右栏 g1 卡片） */
  background: transparent;
  /* 圆角交 DWM 系统圆角独裁（前端再裁 radius 8 会与 DWM 裁切路径 AA 差在四角透缝）；
     实色模式 DWM 无圆角裁切，仍基态前端自裁 8 */
  border-radius: 0;
}
.window.maximized {
  border-radius: 0;
}
/* 1px 窗口描边（伪元素 inset 0 自绘不占空间）——**仅实色回退模式自绘**（通用/六级实心
   --n-g6 + 圆角 8 随壳自裁，Win10 DWM 无边框可画）；云母模式零绘制（边框主权 100% 交
   DWM，见下）；z-54 = 三栏内容之上、picking 外框（55）之下；最大化贴屏边无意义隐藏 */
.window::before {
  content: "";
  position: absolute;
  inset: 0;
  border: 1px solid var(--n-g6);
  border-radius: var(--n-radius-8);
  pointer-events: none;
  z-index: 54;
}
/* 云母模式 CSS 描边零绘制（2026-08-30 六轮定稿，win-border-measure 实机像素测量推翻
   五轮遮罩环）：实测 @150%——DWM 线 = 2 物理 px 四边均匀；Chromium 把 CSS 1px border
   吸附成物理 1px 锐线（宽度吸附设备像素，无半透列），但 CSS 线与 DWM 线的并集左右
   不对称（左/上边视口缘贴 DWM 线内缘 → 并集 3px；右/下边视口缘伸入 DWM 线区 1px →
   CSS 被盖，并集 2px）= 四边 3 vs 2 粗细不一；角部 CSS 弧（半径 7 逻辑 = 10.5 物理）
   与 DWM 裁切弧（10.3 物理）异径分离 → 两弧间露内容色缝（用户实机"圆弧变细/缝隙"
   真身）。任何 CSS 线都逃不掉这套系统错位，唯一均匀解 = 纯 DWM 独画（"别人的完美
   边框"即此路径）；角弧 AA 过渡带 = Windows 原生通病（搜索面板同病），接受。
   maximized 时 DWM 不裁角不画边，同零画（合并入选择器列表）。 */
.window.fx-mica::before,
.window.maximized.fx-mica::before {
  border: 0;
  box-shadow: none;
}
.window.maximized::before {
  border: 0;
  border-radius: 0;
  box-shadow: none;
}
/* 选择模式常态描边让位（实色模式由 ::after 品牌框独存；云母模式 ::before 恒零画） */
.window.picking::before {
  border: 0;
}
/* 选择模式整圈品牌色外边框（6.2 一轮，PRD 03_数据链接 04 四重标识之一；2026-08-30 用户
   改定 = 品牌色-常态 --n-brand，原 品牌色-边框线 --brand-border 废除）——**仅实色模式自绘**；
   云母模式不画：品牌框 = DWM 切染品牌线独存（六轮定稿：CSS 任何线/环与 DWM 线并集
   必错位分离，高亮品牌色下尤显——纯 DWM 染色线 = 原生观感）。
   伪元素不遮交互；z-index 55：盖三栏内容（win-controls 浮块 60 与弹窗仍在其上） */
.window.picking::after {
  content: "";
  position: absolute;
  inset: 0;
  border: var(--border-strong) solid var(--n-brand);
  border-radius: var(--radius-8);
  pointer-events: none;
  z-index: 55;
}
.window.fx-mica.picking::after {
  border: 0; /* 云母模式 ::after 零画：品牌框 = DWM 切染品牌线独存 */
}
.window.maximized.picking::after {
  border-radius: 0;
}
/* 窗口控制浮块（2026-08-08 定案）：恒在窗口右上角（标签栏行 / 右栏标题栏同一位），
   侧栏收展时它不参与布局变化——从根上消除窗口控制的跳变；
   7.1 二十三轮贴稿：浮块无 padding/gap，四钮 58×46 连续排（右栏-窗口控制稿） */
.win-controls-overlay {
  position: absolute;
  top: 0;
  right: 0;
  height: 46px;
  display: flex;
  z-index: 60;
}
/* 窗口控制钮（稿）：58×46 满格 hit 区、无容器底；图标 18 亮色/二级；
   hover 亮色/半透明-3 底、关闭 hover 状态/禁止底（稿 1_hover 节）；过渡 150ms 通用档 */
.wc-btn {
  width: 58px;
  height: 46px;
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
