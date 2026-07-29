// 快速访问视图 DTO（6.1 二轮，自 content store 拆出，规范第 3 条行数控制）：
// 后端 list_recent_assets / list_recycle 的返回形状（PlacementView 同构 flatten + 附加字段），
// 元素含 PlacementView 全部字段，AssetGrid 零改动复用。
import type { PlacementView } from "./content";

// 最近使用条目（后端按时间倒序）
export interface RecentAssetView extends PlacementView {
  viewedAt: string;
}

// 回收站条目（folderId 恒 "recycle"、origin 恒 false——key 与素材域天然独立）
export interface RecycleEntryView extends PlacementView {
  deletedAt: string;
  placementCount: number;
}
