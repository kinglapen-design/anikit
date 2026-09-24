// 网格右键统一调度（6.3，自 AssetGrid 拆出；2026-08-12 自 menuSpecs.ts 再拆，规范第 3 条行数控制）：
// 素材卡片 = 素材菜单（回收站视口 = 13 稿回收站菜单，1.3 归并全局宿主，RecycleCardMenu 局部菜单废除；
// 虚拟视图 = 收窄集——2026-09-09 入口收窄，assetCardSpec 内分流）；
// 工作空间视口（PRD 02 步骤 1）= ws 专属最小集——卡片走 wsCardSpec（打开/资源管理器显示[占位置灰]/
// 刷新），空白走 wsBlankSpec（刷新）；只读底座，素材菜单的一切写操作项不挂；
// 空白处（含 grid-header）按视口分流（2026-09-05 走查「快速访问」节，旧"一律 07 新增菜单"废除）：
//   folder: 视口 = 07 新增菜单（addButtonSpec）；
//   未分类 = quickFullSpec（去新建文件夹/新建档案——快速访问无文件夹概念；2026-09-09 收窄：
//     候车区总入口，留导入 / 新建文档 / 粘贴）；
//   全部 / 最近使用 / 自动导入（quick:inbox）/ 缩略图 = scanOnlySpec（只给扫描两项——全部不是位置，
//     2026-09-09 起随虚拟视图同档）；
//   回收站 = 13 稿空白菜单；seq: 等其余视口照旧新增菜单；
// 文件夹卡片 / 档案卡片 / 按钮 / 输入框上不处理（"pass"——卡片自带菜单）。
// 1.3：菜单项按新稿纯文字口径（2026-09-01）。
// panelId = 发起右键的 panel（2026-09-25 per-panel 预览会话化：随 AssetGrid 注入会话透传，
// 「打开预览」开进属主 panel 自己的预览会话）
import { useContentStore, type PlacementView } from "../../stores/content";
import { usePickerStore } from "../../stores/picker";
import { useSelectionStore } from "../../stores/selection";
import { openContextMenu } from "../contextMenus";
import { assetCardSpec } from "./assetCard";
import { addButtonSpec, quickFullSpec, scanOnlySpec } from "./addMenu";
import { archiveCardSpec } from "./archiveCard";
import { folderCardSpec } from "./folder";
import { recycleBlankSpec, recycleCardSpec } from "./recycle";
import { wsBlankSpec, wsCardSpec } from "./workspace";

export function openGridContextMenu(e: MouseEvent, items: PlacementView[], panelId: string): "done" | "pass" {
  const t = e.target as HTMLElement;
  const selection = useSelectionStore();
  const content = useContentStore();
  const card = t.closest(".asset-card, .list-row");
  const key = card?.getAttribute("data-key");
  if (key) {
    e.preventDefault();
    // 选择模式（6.2）：不入选择域；素材卡出查看类精简集（spec 内分流），回收站卡无可出项；
    // 工作空间同不出（只读底座无挑选语义——选择模式入口全在库视口）
    if (usePickerStore().active) {
      if (content.isRecycleView || content.isWorkspaceView) return "done";
      const p = items.find((it) => `${it.assetId}|${it.folderId}` === key);
      if (p) openContextMenu(e, assetCardSpec(p, panelId), undefined);
      return "done";
    }
    // 工作空间（PRD 02 步骤 1）分流前置：ws 条目出最小只读集，不进素材菜单（写操作项一律不挂）
    if (content.isWorkspaceView) {
      const p = items.find((it) => `${it.assetId}|${it.folderId}` === key);
      if (p) openContextMenu(e, wsCardSpec(p, panelId), undefined);
      return "done";
    }
    if (!selection.isSelected(key)) selection.selectOnly(key);
    // 回收站分流前置（2026-08-11 修复 item 右键调不出：键解析失败也不依赖 items 反查——
    // recycleCardSpec 无参回退基础集）；2026-09-05 全量菜单起传条目 p（打开/导出/复制数据等要 assetId）
    if (content.isRecycleView) {
      const p = items.find((it) => `${it.assetId}|${it.folderId}` === key);
      openContextMenu(e, recycleCardSpec(p, panelId), undefined);
      return "done";
    }
    const p = items.find((it) => `${it.assetId}|${it.folderId}` === key);
    if (p) openContextMenu(e, assetCardSpec(p, panelId), undefined);
    return "done";
  }
  // .disk-card = 收件箱工程文件卡片（自带 inboxDiskSpec 菜单，InboxDiskSection @contextmenu.stop 已拦冒泡）
  if (t.closest(".folder-card, .archive-card, .disk-card, button, input")) return "pass";
  e.preventDefault();
  // 选择模式（6.2）：空白处新增菜单是编辑类，不出
  if (usePickerStore().active) return "done";
  const vk = content.sidebarSelected;
  if (vk.startsWith("ws:")) openContextMenu(e, wsBlankSpec(vk.slice(3)), undefined);
  else if (vk === "quick:recycle") openContextMenu(e, recycleBlankSpec(), undefined);
  else if (vk.startsWith("folder:")) openContextMenu(e, addButtonSpec(), undefined);
  else if (vk === "quick:uncategorized") openContextMenu(e, quickFullSpec(), undefined);
  else if (vk === "quick:all" || vk === "quick:recent" || vk === "quick:inbox" || vk === "quick:thumbnail") openContextMenu(e, scanOnlySpec(), undefined);
  else openContextMenu(e, addButtonSpec(), undefined);
  return "done";
}

// 文件夹卡片右键（FolderStrip 调用）：05 稿
export function openFolderCardMenu(e: MouseEvent, folder: { id: string; name: string }): void {
  openContextMenu(e, folderCardSpec(folder), undefined);
}

// 档案卡片右键（ArchiveStrip 调用，3.2 一轮）：04 稿
export function openArchiveCardMenu(e: MouseEvent, archive: { id: string; name: string }): void {
  openContextMenu(e, archiveCardSpec(archive), undefined);
}
