// 全局键盘 composable（2026-08-12 自 App.vue 拆出，规范第 3 条行数控制）：
// window keydown 四处理器（剪贴板导入 / 撤销重做 / 选择 / 操作快捷键）+ 注册注销；
// 输入框豁免判定 isEditableTarget 为 App / SidebarLeft（Ctrl+N）/ AssetGrid（空格）共用。
import { onBeforeUnmount, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useContentStore } from "../stores/content";
import { usePreviewStore } from "../stores/preview";
import { useSelectionStore } from "../stores/selection";
import { useUndoStore } from "../stores/undo";
import {
  copySelection,
  cutFolders,
  deletePlacements,
  deleteRecycle,
  duplicateSelected,
  pasteClipboard,
  pasteDuplicate,
  recycleSelectedAssetIds,
} from "./placementOps";
import { cloneFolderFlow, deleteFoldersFlow, moveAssetsFlow, moveFoldersFlow } from "./menuOps";

// 输入框豁免判定：文本输入态（input / textarea / contenteditable）不拦截全局快捷键
export function isEditableTarget(t: HTMLElement): boolean {
  return t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable;
}

export function useGlobalKeys(opts: { previewParam: string | null; needLibrarySetup: () => boolean }): void {
  const content = useContentStore();
  const selection = useSelectionStore();
  const preview = usePreviewStore();
  const undo = useUndoStore();

  // 剪贴板导入（PRD 05-01；2026-07-27 扩：图片 + 文件列表，恒 Copy）；剪贴板无图无文件属常态，静默降级
  // 6.1 仲裁（2026-07-28 定案）：内部剪贴板非空时 Ctrl+V 归 onOperationKeys（链接粘贴 / 移动，
  // 2026-08-18 起链接粘贴静默不弹窗），本函数只处理"内部剪贴板为空"的系统剪贴板导入；
  // Ctrl+Shift+V = 粘贴为副本快捷键，不走导入
  async function onPaste(e: KeyboardEvent): Promise<void> {
    if (!(e.ctrlKey || e.metaKey) || e.key.toLowerCase() !== "v") return;
    if (e.shiftKey || selection.clipboard) return;
    if (isEditableTarget(e.target as HTMLElement)) return;
    if (opts.needLibrarySetup() || content.importDraft) return;
    try {
      await content.startImportClipboard(content.currentFolderId);
    } catch (err) {
      console.debug("剪贴板导入不可用：", err);
    }
  }

  // 撤销 / 重做（6.0）：Ctrl+Z / Ctrl+Shift+Z；输入框内不拦截（保留文本编辑自身的撤销）
  function onUndoRedo(e: KeyboardEvent): void {
    if (!(e.ctrlKey || e.metaKey) || e.key.toLowerCase() !== "z") return;
    if (isEditableTarget(e.target as HTMLElement)) return;
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
    if (isEditableTarget(e.target as HTMLElement)) return;
    if (opts.previewParam || preview.assetKey || opts.needLibrarySetup()) return;
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "a") {
      e.preventDefault();
      selection.selectAll();
      return;
    }
    // 让位判定读"Esc 按下时刻"快照（2026-08-16 用户拍板根治）：AppMenu capture 关菜单后
    // DOM 同步移除，bubble 再查 .app-menu 已失效（误清选择实锤）——capture 早序监听先打标，
    // 此处只读标记：本次 Esc 被菜单吃掉就不清选择；无菜单时维持原语义（清选择）
    if (e.key === "Escape" && !escHadOverlay) {
      selection.clear();
    }
  }

  // 操作快捷键（6.1 阶段 C，2026-07-28 用户定案，与 onSelectionKeys 同口径）：
  // F2 = 重命名当前单选素材（卡片就地重命名；多选不触发；文件夹重命名走卡片名称双击 /
  // 新建后命名态，2026-07-29 复验三轮 rename_folder 已接入）；
  // Del = 删除选中项（2026-07-29 用户修订：取消两段式询问，直接移入回收站，撤销可找回）；
  // Ctrl+C / Ctrl+X = 写内部剪贴板（仅素材）；Ctrl+V 仲裁（内部非空 → copy 静默链接粘贴 /
  // cut 移动；空 → onPaste 系统剪贴板导入）；Ctrl+Shift+V = 直接副本不弹窗（名称同源）；
  // Ctrl+D = 创建副本（2026-08-18 补绑）。
  // 回收站视口（6.1 二轮，2026-07-29）：F2 不触发（只读）；Del 分流彻底删除两段式
  //（delete_recycle，虚拟片段引用确认窗，不可恢复无撤销）——delete_placements 路径不再走。
  // 快速访问视图不作粘贴目标（toast 提示）；菜单 / 浮层 / 弹窗打开时让位
  function onOperationKeys(e: KeyboardEvent): void {
    if (isEditableTarget(e.target as HTMLElement)) return;
    if (opts.previewParam || preview.assetKey || opts.needLibrarySetup()) return;
    if (document.querySelector(".app-menu, .zoom-mask, .link-mask, .del-mask, .pick-mask")) return;
    if (e.key === "F2") {
      if (selection.count === 1 && !content.isRecycleView) {
        e.preventDefault();
        selection.renamingKey = selection.first;
      }
      return;
    }
    // Shift+Enter = 默认应用打开（6.3 右键菜单同款，2026-07-29 用户设计）
    if (e.key === "Enter" && e.shiftKey) {
      if (selection.count === 1 && !content.isRecycleView) {
        e.preventDefault();
        const p = content.visiblePlacements.find((v) => `${v.assetId}|${v.folderId}` === selection.first);
        if (p) {
          void invoke("open_with_default_app", { assetId: p.assetId }).then(() => content.refreshRecent());
        }
      }
      return;
    }
    if (e.key === "Delete") {
      // 文件夹域分流（3.4 批量删除，2026-08-18）：与素材域互斥，优先判定——
      // 原实现只读素材域，多选文件夹按 Delete 无反应（右键菜单也只删被右键的一个）
      if (selection.selectedFolders.size > 0) {
        e.preventDefault();
        deleteFoldersFlow(selection.orderedSelectedFolderIds());
        return;
      }
      if (selection.count > 0) {
        e.preventDefault();
        if (content.isRecycleView) {
          void deleteRecycle(recycleSelectedAssetIds()); // 回收站 = 彻底删除两段式
        } else {
          void deletePlacements(selection.selectedRefs());
        }
      }
      return;
    }
    if (!(e.ctrlKey || e.metaKey)) return;
    const k = e.key.toLowerCase();
    if (k === "c" || k === "x") {
      // 文件夹域（3.4 剪切粘贴）：Ctrl+X 剪切文件夹（= 移动，粘贴走 move_folders 护栏）；
      // Ctrl+C 对文件夹无语义（文件夹复制 = 克隆，走 Ctrl+D 克隆）
      if (selection.selectedFolders.size > 0) {
        if (k === "x") {
          e.preventDefault();
          cutFolders(selection.orderedSelectedFolderIds());
        }
        return;
      }
      if (selection.count > 0) {
        e.preventDefault();
        copySelection(k === "c" ? "copy" : "cut");
      }
      return;
    }
    // Ctrl+D = 创建副本（素材域）/ 克隆文件夹（文件夹域，3.4 四轮复制克隆）
    if (k === "d") {
      if (selection.selectedFolders.size > 0) {
        e.preventDefault();
        const first = selection.orderedSelectedFolderIds()[0];
        if (first) cloneFolderFlow(first); // 触发项在域内 → 批量分流整个选中集
        return;
      }
      if (selection.count > 0 && !content.isRecycleView) {
        e.preventDefault();
        void duplicateSelected();
      }
      return;
    }
    // Ctrl+Y = 移动面板（3.4，29 稿）：文件夹域 → 移动文件夹/档案；素材域 → 移动文件（回收站只读不开）
    if (k === "y") {
      if (selection.selectedFolders.size > 0) {
        e.preventDefault();
        const first = selection.orderedSelectedFolderIds()[0];
        if (first) moveFoldersFlow(first); // 触发项在域内 → 批量分流整个选中集
      } else if (selection.count > 0 && !content.isRecycleView) {
        e.preventDefault();
        moveAssetsFlow();
      }
      return;
    }
    // Ctrl+R = 重命名（3.4 补绑，菜单标注已久）：单选就地重命名 / 多选批量重命名弹窗（37 稿）
    if (k === "r") {
      if (selection.count > 0 && !content.isRecycleView) {
        e.preventDefault();
        if (selection.count > 1) selection.renameDraft = selection.selectedRefs();
        else selection.renamingKey = selection.first;
      }
      return;
    }
    // 粘贴两键（2026-08-18 定案）：Ctrl+V 链接 / Ctrl+Shift+V 副本；Ctrl+Alt+V 键位废除不响应
    if (k === "v" && selection.clipboard) {
      if (e.altKey) return;
      e.preventDefault();
      if (e.shiftKey) void pasteDuplicate();
      else void pasteClipboard();
    }
  }

  // Esc 按下时刻的浮层快照（2026-08-16 根治"Esc 关菜单误清选择"）：
  // capture 阶段早序记录（本监听随 App 挂载注册，恒早于任何菜单的 capture 关闭处理），
  // bubble 的 onSelectionKeys 只读此标记，不再查实时 DOM（菜单关闭后 DOM 已同步移除，
  // 实时查询必失效）。覆盖全部 AppMenu 系弹层（.app-menu）与悬停大图浮层（.zoom-mask）；
  // AppModal 不吃 Esc（既定口径），不参与。
  let escHadOverlay = false;
  function onEscCapture(e: KeyboardEvent): void {
    if (e.key === "Escape") escHadOverlay = !!document.querySelector(".app-menu, .zoom-mask");
  }

  onMounted(() => {
    window.addEventListener("keydown", onEscCapture, true);
    window.addEventListener("keydown", onPaste);
    window.addEventListener("keydown", onUndoRedo);
    window.addEventListener("keydown", onSelectionKeys);
    window.addEventListener("keydown", onOperationKeys);
  });
  onBeforeUnmount(() => {
    window.removeEventListener("keydown", onEscCapture, true);
    window.removeEventListener("keydown", onPaste);
    window.removeEventListener("keydown", onUndoRedo);
    window.removeEventListener("keydown", onSelectionKeys);
    window.removeEventListener("keydown", onOperationKeys);
  });
}
