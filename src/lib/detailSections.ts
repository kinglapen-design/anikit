// 详情面板数据层（7.1 二十三轮 tab 化重构）：DTO 与工具函数保留；旧「固定 7 区块注册表 +
// 排序/显隐配置」（2026-08-15 定案）随三 tab 化废除——新区块结构 = pinned 缩略图/封面区块（恒顶）
// + 基础信息 / 链接关系 / 提示词 三 tab（分段控制器切换）；「副本」区块废除、笔记区块待补稿接回。
// 链接关系 tab 五区块 = 唯一可排序组（2026-08-31 用户定：只有带标题行的链接 tab 区块可拖，
// 面板内拖标题行重排，DetailSections 旧 secDrag 范式移植；显隐后续接「信息查看管理」面板）。
import type { ArchiveDetailView, FolderNode, PlacementView } from "../stores/content";

/** 链接关系 tab 区块注册表（顺序 = 稿序：链接 / 音频 / 关系 / 缩略图 / 污染文件） */
export const DETAIL_LINK_BLOCKS = ["links", "audio", "relations", "thumb", "pollution"] as const;
export type DetailLinkBlockId = (typeof DETAIL_LINK_BLOCKS)[number];

/** 链接 tab 区块标题 */
export const DETAIL_LINK_BLOCK_TITLES: Record<DetailLinkBlockId, string> = {
  links: "链接",
  audio: "音频",
  relations: "关系",
  thumb: "缩略图",
  pollution: "污染文件",
};

/** 持久化顺序与注册表兜底合并：未知 id 忽略、重复去重、缺 id 按注册表相对序补位（显隐字段废除） */
export function mergeDetailLinkOrder(saved: unknown): string[] {
  const known = new Set<string>(DETAIL_LINK_BLOCKS);
  const raw = Array.isArray(saved) ? saved.filter((id): id is string => typeof id === "string" && known.has(id)) : [];
  const deduped = [...new Set(raw)];
  for (const id of DETAIL_LINK_BLOCKS) {
    if (!deduped.includes(id)) deduped.push(id);
  }
  return deduped;
}

// —— 后端 DTO（commands/archive.rs，camelCase，07_接口文档）——

/** 链接区块一条：哪个区块的哪个文件夹、显示什么名称（未分类 folderId = null、path = ["未分类"]） */
export interface PlacementLink {
  folderId: string | null;
  blockName: string;
  path: string[];
  name: string;
  origin: boolean;
}

/** 归属档案一条 */
export interface ArchiveRef {
  folderId: string;
  name: string;
}

/** 缩略图区块一条（8.1 伴生缩略图，2026-09-23 接业务）：assetId + 显示名 + 引用数
 * （users 仅调用侧卡片有 = 该伴生图被引用总数；被调用侧行恒 null 不显示） */
export interface ThumbLinkRef {
  assetId: string;
  name: string;
  users: number | null;
}

/** 资产详情（get_asset_detail 只读命令） */
export interface AssetDetail {
  assetId: string;
  originName: string;
  ext: string;
  size: number;
  width: number | null;
  height: number | null;
  duration: number | null;
  importedAt: string; // RFC3339 本地时间
  modifiedAt: string;
  tags: string[]; // 属性词
  descriptions: { title: string; content: string }[]; // 描述（6.1）
  colors: string[]; // 主色（6.1 颜色解析）：#RRGGBB 按占比降序，最多存 10，bar 显示前 7
  links: string[];
  note: string;
  placements: PlacementLink[];
  archives: ArchiveRef[];
  /** 缩略图区块·调用侧（8.1，2026-09-23 接业务）：本资产 thumb_asset_id 指向的 hidden 伴生资产；
   *  无绑定 / 目标非 hidden（还原途中）= null */
  thumbAsset: ThumbLinkRef | null;
  /** 缩略图区块·被调用侧：本资产是 hidden 伴生缩略图时，引用它的资产清单（按显示名排序） */
  thumbUsedBy: ThumbLinkRef[];
}

/** 详情面板展示目标：选中文件卡片 / 选中档案卡片 / 选中普通文件夹（7.1 二十三轮文件夹详情形态）/
 *  快速访问视口（未选中文件时显示当前视口信息，同日用户定；quick = 全部/未分类等快速访问） */
export type DetailTarget =
  | { kind: "file"; placement: PlacementView; detail: AssetDetail }
  | { kind: "archive"; node: FolderNode; data: ArchiveDetailView }
  | { kind: "folder"; node: FolderNode }
  | { kind: "quick"; viewKey: string; name: string };

/** 档案无记录 / 未拉取时的默认空数据（与后端 ArchiveData::new + 零统计同形） */
export function emptyArchiveData(folderId: string): ArchiveDetailView {
  return {
    folderId,
    badge: null,
    avatar: null,
    props: [],
    descriptions: [],
    links: [],
    createdAt: "",
    fileCount: 0,
    totalSize: 0,
    coverPath: null,
  };
}

/** 基础信息日期格式（与既有基础信息稿一致）：RFC3339 → YYYY/MM/DD HH:mm；解析失败原样返回 */
export function formatDetailDate(s: string): string {
  const d = new Date(s);
  if (Number.isNaN(d.getTime())) return s;
  const p = (n: number): string => String(n).padStart(2, "0");
  return `${d.getFullYear()}/${p(d.getMonth() + 1)}/${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`;
}

/** 递归查文件夹节点（DetailPanel 档案判定 / SectionLinks 档案目录后缀共用） */
export function findFolderNode(nodes: FolderNode[], id: string): FolderNode | null {
  for (const n of nodes) {
    if (n.id === id) return n;
    const hit = findFolderNode(n.children, id);
    if (hit) return hit;
  }
  return null;
}
