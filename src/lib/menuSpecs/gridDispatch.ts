// 网格右键统一调度（6.3，自 AssetGrid 拆出；2026-08-12 自 menuSpecs.ts 再拆，规范第 3 条行数控制）：
// 素材卡片 = 素材菜单（回收站视口 = 13 稿回收站菜单，1.3 归并全局宿主，RecycleCardMenu 局部菜单废除）；
// 空白处（含 grid-header）= 07 新增菜单（2026-08-11 用户定：空白右键/新增按钮/Ctrl+N 三入口共用，
// 旧 6.3 空白菜单 assetBlankSpec/folderBlankSpec 废除——布局方式等归 28 查看方式菜单）；
// 回收站视口空白 = 13 稿空白菜单；文件夹卡片 / 档案卡片 / 按钮 / 输入框上不处理（"pass"——卡片自带菜单）。
// 1.3：素材 / 文件夹卡片菜单按稿出 320 宽 + 搜索框。
import { useContentStore, type PlacementView } from "../../stores/content";
import { useSelectionStore } from "../../stores/selection";
import { openContextMenu } from "../contextMenus";
import { assetCardSpec } from "./assetCard";
import { addButtonSpec } from "./addMenu";
import { archiveCardSpec } from "./archiveCard";
import { folderCardSpec } from "./folder";
import { recycleBlankSpec, recycleCardSpec } from "./recycle";

export function openGridContextMenu(e: MouseEvent, items: PlacementView[]): "done" | "pass" {
  const t = e.target as HTMLElement;
  const selection = useSelectionStore();
  const content = useContentStore();
  const card = t.closest(".asset-card, .list-row");
  const key = card?.getAttribute("data-key");
  if (key) {
    e.preventDefault();
    if (!selection.isSelected(key)) selection.selectOnly(key);
    // 回收站分流前置（2026-08-11 修复 item 右键调不出：键解析失败也不依赖 items 反查，
    // 与 6.1 旧代码同构——回收站菜单不读卡片数据）
    if (content.isRecycleView) {
      openContextMenu(e, recycleCardSpec(), undefined, { width: 148 });
      return "done";
    }
    const p = items.find((it) => `${it.assetId}|${it.folderId}` === key);
    if (p) openContextMenu(e, assetCardSpec(p), undefined, { width: 320, searchable: true });
    return "done";
  }
  // .disk-card = 收件箱工程文件卡片（自带 inboxDiskSpec 菜单，InboxDiskSection @contextmenu.stop 已拦冒泡）
  if (t.closest(".folder-card, .archive-card, .disk-card, button, input")) return "pass";
  e.preventDefault();
  if (content.isRecycleView) openContextMenu(e, recycleBlankSpec(), undefined, { width: 148 });
  else openContextMenu(e, addButtonSpec(), undefined, { width: 320 });
  return "done";
}

// 文件夹卡片右键（FolderStrip 调用）：05 稿 320 宽 + 搜索框
export function openFolderCardMenu(e: MouseEvent, folder: { id: string; name: string }): void {
  openContextMenu(e, folderCardSpec(folder), undefined, { width: 320, searchable: true });
}

// 档案卡片右键（ArchiveStrip 调用，3.2 一轮）：04 稿 320 宽 + 搜索框
export function openArchiveCardMenu(e: MouseEvent, archive: { id: string; name: string }): void {
  openContextMenu(e, archiveCardSpec(archive), undefined, { width: 320, searchable: true });
}
