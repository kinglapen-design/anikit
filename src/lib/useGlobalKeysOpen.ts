// 打开类全局键（2026-10-15 快捷键全量点亮轮——设置面板速查表「打开预览 / 新标签页打开」
// 原仅菜单标注未绑键，本轮与菜单动作同径点亮）：
// Enter = 打开（素材 = 打开预览 previewSession.open；文件夹 / 档案 = openView 进入，与双击卡片同径
// ——folder/archiveCard「打开」菜单项 2026-09-23 删除时注明 Enter 快捷键本体不受影响）；
// Ctrl+Enter = 新标签页打开（素材 = openPreviewInNewTab 直进预览；文件夹 / 档案 = openInNewTab 同视口）。
// 库 / 回收站 / 工作空间三域同键：回收站菜单两项本就可点（recycleCardSpec）；ws 文件夹 = openView/openInNewTab
// `ws:` 视口（断链拦截走 openView 内建守卫），ws 文件 = 预览两径同库。
// 单选才响应（右键菜单恒单选语境同口径；多选不响应）；Shift+Enter = 默认应用打开归 onOperationKeys
// 不在此处理；选择模式（picker）与 Shift+Enter 同口径全禁（选择域恒空，双保险）。
import { onBeforeUnmount, onMounted } from "vue";
import { useContentStore } from "../stores/content";
import { usePickerStore } from "../stores/picker";
import { previewSession } from "../stores/preview";
import { useSelectionStore } from "../stores/selection";
import { useTabsStore } from "../stores/tabs";
import { useWsStore } from "../stores/ws";
import { globalKeysYield, type GlobalKeysOpts } from "./globalKeysGuard";

export function useOpenKeys(opts: GlobalKeysOpts): void {
  const content = useContentStore();
  const selection = useSelectionStore();
  const tabs = useTabsStore();

  function onOpenKeys(e: KeyboardEvent): void {
    if (e.key !== "Enter" || e.shiftKey || e.altKey) return; // Shift+Enter / Alt 组合不归本处理器
    const ctrl = e.ctrlKey || e.metaKey;
    if (globalKeysYield(e, opts)) return;
    if (usePickerStore().active) return;
    const panelId = tabs.activeTabId;
    if (!panelId) return;
    // 库两域互斥：文件夹域单选（含档案）/ 素材域单选；ws 文件与文件夹同在素材域（可混选——混选不响应）
    const folderSingle = selection.selectedFolders.size === 1 && selection.count === 0;
    const assetSingle = selection.selectedFolders.size === 0 && selection.count === 1;
    if (content.isWorkspaceView) {
      if (!assetSingle || !selection.first.startsWith("ws:")) return;
      const [assetId, folderId] = selection.first.split("|");
      const rel = assetId.slice(3);
      const ws = useWsStore();
      const entry = ws.entries((folderId ?? "").slice(3)).find((x) => `ws:${x.relPath}` === assetId);
      if (!entry) return;
      e.preventDefault();
      if (entry.isDir) {
        if (ctrl) tabs.openInNewTab(`ws:${rel}`);
        else tabs.openView(`ws:${rel}`);
      } else if (ctrl) {
        tabs.openPreviewInNewTab(panelId, selection.first);
      } else {
        void previewSession(panelId).open(selection.first);
      }
      return;
    }
    if (folderSingle) {
      const id = selection.orderedSelectedFolderIds()[0];
      if (!id) return;
      e.preventDefault();
      if (ctrl) tabs.openInNewTab(`folder:${id}`);
      else tabs.openView(`folder:${id}`);
      return;
    }
    if (assetSingle) {
      // 素材（含回收站条目——recycle 菜单同键可点）：key = selection.first 与菜单 assetKey 同构
      e.preventDefault();
      if (ctrl) tabs.openPreviewInNewTab(panelId, selection.first);
      else void previewSession(panelId).open(selection.first);
    }
  }

  onMounted(() => window.addEventListener("keydown", onOpenKeys));
  onBeforeUnmount(() => window.removeEventListener("keydown", onOpenKeys));
}
