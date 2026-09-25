// 详情面板数据层：后端 DTO / 工具函数 + 三 tab 区块注册表。
// 区块结构 = pinned 缩略图/封面区块（恒顶，不进注册表）+ 基础信息 / 链接关系 / 提示词 三 tab
//（7.1 二十三轮 tab 化）；「副本」区块废除、笔记区块待补稿接回。
// 排序/显隐 = 信息查看管理面板（2026-09-26 重立，旧 22 稿 SectionEditPanel 后继）：三 tab 各一套
// { order, hidden }（ui.detailBlocks 持久化，mergeDetailBlocks 注册表兜底——未知 id 忽略、
// 缺 id 按注册表稿序补位）；排序双入口 = 管理面板箭头（全量序）+ 链接 tab 面板内拖标题行
//（可见序落点，2026-08-31 用户定）；旧 detailLinksOrder 单键配置由 loadSettings 迁移并入。
import type { ArchiveDetailView, FolderNode, PlacementView } from "../stores/content";

/** 详情面板三 tab（分段控制器稿序：基础信息 / 链接关系 / 提示词） */
export const DETAIL_TABS = ["basic", "links", "prompt"] as const;
export type DetailTabKey = (typeof DETAIL_TABS)[number];

/** 基础信息 tab 区块注册表（稿序：输入框 / 标签 / 详情 / 基本数据） */
export const DETAIL_BASIC_BLOCKS = ["fields", "tags", "descs", "baseData"] as const;

/** 链接关系 tab 区块注册表（稿序：链接 / 音频 / 关系 / 缩略图 / 污染文件） */
export const DETAIL_LINK_BLOCKS = ["links", "audio", "relations", "thumb", "pollution"] as const;
export type DetailLinkBlockId = (typeof DETAIL_LINK_BLOCKS)[number];

/** 提示词 tab 区块注册表（单区块——排序无意义，仅显隐） */
export const DETAIL_PROMPT_BLOCKS = ["prompt"] as const;

/** 各 tab 区块标题（信息查看管理面板行标题；基础信息四区块详情侧无标题行，按基础信息子稿命名） */
export const DETAIL_TAB_BLOCK_TITLES: Record<DetailTabKey, Record<string, string>> = {
  basic: { fields: "名称与链接", tags: "标签", descs: "详情", baseData: "基本数据" },
  links: { links: "链接", audio: "音频", relations: "关系", thumb: "缩略图", pollution: "污染文件" },
  prompt: { prompt: "提示词" },
};

/** 每 tab 区块配置：order = 全量顺序（含隐藏项，管理面板行序）；hidden = 隐藏清单（详情渲染过滤） */
export interface DetailBlocksConfig {
  order: string[];
  hidden: string[];
}
export type DetailBlocksState = Record<DetailTabKey, DetailBlocksConfig>;

const DETAIL_TAB_BLOCKS: Record<DetailTabKey, readonly string[]> = {
  basic: DETAIL_BASIC_BLOCKS,
  links: DETAIL_LINK_BLOCKS,
  prompt: DETAIL_PROMPT_BLOCKS,
};

/** 注册表兜底合并顺序：未知 id 忽略、重复去重、缺 id 按注册表稿序补位 */
function mergeOrder(saved: unknown, registry: readonly string[]): string[] {
  const known = new Set(registry);
  const raw = Array.isArray(saved)
    ? saved.filter((id): id is string => typeof id === "string" && known.has(id))
    : [];
  const deduped = [...new Set(raw)];
  for (const id of registry) {
    if (!deduped.includes(id)) deduped.push(id);
  }
  return deduped;
}

/** 注册表兜底合并隐藏清单：未知 id 忽略、去重 */
function mergeHidden(saved: unknown, registry: readonly string[]): string[] {
  const known = new Set(registry);
  const raw = Array.isArray(saved)
    ? saved.filter((id): id is string => typeof id === "string" && known.has(id))
    : [];
  return [...new Set(raw)];
}

/** 默认配置（全注册表稿序、无隐藏） */
export function defaultDetailBlocks(): DetailBlocksState {
  return mergeDetailBlocks(null);
}

/** 持久化配置兜底合并：三 tab 各 { order, hidden }；links tab 缺 order 时回退 legacyLinksOrder
 * （旧 ui.detailLinksOrder 迁移通道，loadSettings 传入；无旧配置 = 注册表稿序） */
export function mergeDetailBlocks(saved: unknown, legacyLinksOrder?: unknown): DetailBlocksState {
  const s = (saved !== null && typeof saved === "object" ? saved : {}) as Record<
    string,
    { order?: unknown; hidden?: unknown }
  >;
  const out = {} as DetailBlocksState;
  for (const tab of DETAIL_TABS) {
    const registry = DETAIL_TAB_BLOCKS[tab];
    const orderSeed = s[tab]?.order ?? (tab === "links" ? legacyLinksOrder : undefined);
    out[tab] = { order: mergeOrder(orderSeed, registry), hidden: mergeHidden(s[tab]?.hidden, registry) };
  }
  return out;
}

/** 详情渲染序 = 全量序过滤隐藏项（TabBasic / TabLinks / TabPrompt 共用） */
export function visibleDetailBlocks(cfg: DetailBlocksConfig): string[] {
  return cfg.order.filter((id) => !cfg.hidden.includes(id));
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
