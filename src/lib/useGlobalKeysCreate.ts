// 新建 / 整理类全局键（2026-10-15 快捷键全量点亮轮——速查表新建组与「展开或收起所有文件夹」
// 原仅菜单标注未绑键，本轮与 appMenu / folder 菜单动作同径点亮）：
// Ctrl+Shift+N = 新建文件夹（文件夹视口建到当前文件夹；其余位置视图 = 默认区块根级，appMenu 同径；
//   六个快速访问视口禁——isQuickView，快速访问无文件夹概念）；
// Ctrl+Shift+Alt+N = 新建子文件夹（当前文件夹，folderId null 禁）；
// Shift+N = 新建档案（默认区块根级；快速访问禁）；
// Alt+N = 转换为档案（当前文件夹视口且非档案——档案再转换无语义；两段式确认走 convertToArchiveFlow）；
// "/" = 展开/收起所有文件夹（folder 树节点菜单同径：有展开即全收、否则全展；查看类——选择模式放行，
//   与 onViewKeys 查看类白名单同口径；ws 树无此菜单不响应）。
// ws 视口：新建两键 = 当前目录新建（createAndRename——库文件夹视口内「新建文件夹 / 新建子文件夹」
// 同效口径的 ws 映射；ws 恒在目录视口含根 ""）；档案两键与 "/" 无 ws 语义不响应。
// 新建 / 转换 = 编辑类，选择模式（picker）全禁（onOperationKeys 同口径）。
import { onBeforeUnmount, onMounted } from "vue";
import { DEFAULT_BLOCK_ID, useContentStore } from "../stores/content";
import { usePickerStore } from "../stores/picker";
import { collapseAll, expandAll, openBlock, treeExpansion } from "./treeExpansion";
import { convertToArchiveFlow, createAndName, createArchiveAndName } from "./menuOps";
import { createAndRename } from "./menuSpecs/workspace";
import { globalKeysYield, type GlobalKeysOpts } from "./globalKeysGuard";

export function useCreateKeys(opts: GlobalKeysOpts): void {
  const content = useContentStore();

  function onCreateKeys(e: KeyboardEvent): void {
    // "/" = 展开/收起所有文件夹（查看类，picker 放行；shift+"/" = "?" 天然不命中）
    if (e.key === "/" && !e.ctrlKey && !e.metaKey && !e.altKey && !e.shiftKey) {
      if (globalKeysYield(e, opts)) return;
      if (content.isWorkspaceView) return;
      e.preventDefault();
      if (treeExpansion.openSet.size > 0) collapseAll();
      else expandAll(content.folders);
      return;
    }
    if (e.key.toLowerCase() !== "n") return;
    if (globalKeysYield(e, opts)) return;
    if (usePickerStore().active) return;
    const ctrl = e.ctrlKey || e.metaKey;
    // ws：Ctrl+Shift(+Alt)+N = 当前目录新建文件夹（先建后命名同菜单）；其余 N 组合无 ws 语义
    if (content.isWorkspaceView) {
      if (!ctrl || !e.shiftKey) return;
      e.preventDefault();
      void createAndRename(content.sidebarSelected.slice(3));
      return;
    }
    // Ctrl+Shift+Alt+N = 新建子文件夹（当前文件夹）
    if (ctrl && e.shiftKey && e.altKey) {
      const folderId = content.currentFolderId;
      if (!folderId) return;
      e.preventDefault();
      void createAndName(folderId, null);
      return;
    }
    // Ctrl+Shift+N = 新建文件夹（当前文件夹 / 默认区块根级）
    if (ctrl && e.shiftKey) {
      if (content.isQuickView) return;
      e.preventDefault();
      const cur = content.currentFolderId;
      if (cur !== null) void createAndName(cur, null);
      else {
        openBlock(DEFAULT_BLOCK_ID);
        void createAndName(null, null, DEFAULT_BLOCK_ID);
      }
      return;
    }
    if (ctrl) return; // 其余 Ctrl+N 组合不响应（旧 Ctrl+N 新增入口 7.1 已撤，不复活）
    // Shift+N = 新建档案（默认区块根级）
    if (e.shiftKey && !e.altKey) {
      if (content.isQuickView) return;
      e.preventDefault();
      openBlock(DEFAULT_BLOCK_ID);
      void createArchiveAndName(null, null, DEFAULT_BLOCK_ID);
      return;
    }
    // Alt+N = 转换为档案（当前文件夹且非档案）
    if (e.altKey && !e.shiftKey) {
      const folderId = content.currentFolderId;
      if (!folderId || content.archives[folderId]) return;
      e.preventDefault();
      convertToArchiveFlow(folderId);
    }
  }

  onMounted(() => window.addEventListener("keydown", onCreateKeys));
  onBeforeUnmount(() => window.removeEventListener("keydown", onCreateKeys));
}
