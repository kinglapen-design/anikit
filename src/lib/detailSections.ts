// 详情面板数据层：后端 DTO / 工具函数 + 三 tab 区块注册表。
// 区块结构 = pinned 缩略图/封面区块（恒顶，不进注册表）+ 基础信息 / 链接关系 / 自定义组件 三 tab
//（7.1 二十三轮 tab 化）；「副本」区块废除。
// **三 tab 语义分界（2026-10-12 用户定）**：基础信息 = 是什么（名称/链接/标签/描述/基本数据）；
// 链接关系 = 和谁连着；自定义组件 = 绑定选中对象的文件级生产/批注扩展（PRD 07 扩展点第 1 条
// 同日修订口径）——首批两张真卡片 = 提示词（自基础信息末位挪回，9-26 二轮口径修订）+
// 逐帧笔记（DetailNotes 摘出 2026-08-31 后接回换代，NotesSection + items/NoteItem）。
// 排序/显隐 = 信息查看管理面板（2026-09-26 重立，旧 22 稿 SectionEditPanel 后继）：三 tab 各一套
// { order, hidden }（ui.detailBlocks 持久化，mergeDetailBlocks 注册表兜底——未知 id 忽略、
// 缺 id 按注册表稿序补位）；排序双入口 = 管理面板箭头（全量序）+ 链接 tab 面板内拖标题行
//（可见序落点，2026-08-31 用户定）；旧 detailLinksOrder 单键配置由 loadSettings 迁移并入。
import type { ArchiveDetailView, FolderNode, PlacementView, PromptBinding } from "../stores/content";
import type { WsEntry } from "./wsOps";

/** 详情面板三 tab（分段控制器稿序：基础信息 / 链接关系 / 自定义组件——自定义组件 =
 *  文件级扩展卡片容器（2026-10-12 转正：提示词 + 逐帧笔记两张卡片装入，占位期结束） */
export const DETAIL_TABS = ["basic", "links", "custom"] as const;
export type DetailTabKey = (typeof DETAIL_TABS)[number];

/** 基础信息 tab 区块注册表（稿序：输入框 / 标签 / 详情 / 基本数据） */
export const DETAIL_BASIC_BLOCKS = ["fields", "tags", "descs", "baseData"] as const;

/** 链接关系 tab 区块注册表（稿序：链接 / 音频 / 关系 / 缩略图 / 污染文件） */
export const DETAIL_LINK_BLOCKS = ["links", "audio", "relations", "thumb", "pollution"] as const;
export type DetailLinkBlockId = (typeof DETAIL_LINK_BLOCKS)[number];

/** 自定义组件 tab 卡片注册表（2026-10-12：稿序 = 提示词 / 逐帧笔记；7.3 T3 detailTabSpecs
 *  注册表化后新卡片（直方图等）挂同一容器） */
export const DETAIL_CUSTOM_BLOCKS = ["prompt", "notes"] as const;

/** 各 tab 区块标题（信息查看管理面板行标题；基础信息四区块详情侧无标题行，按基础信息子稿命名） */
export const DETAIL_TAB_BLOCK_TITLES: Record<DetailTabKey, Record<string, string>> = {
  basic: { fields: "名称与链接", tags: "标签", descs: "详情", baseData: "基本数据" },
  links: { links: "链接", audio: "音频", relations: "关系", thumb: "缩略图", pollution: "污染文件" },
  custom: { prompt: "提示词", notes: "逐帧笔记" },
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
  custom: DETAIL_CUSTOM_BLOCKS,
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
 * （旧 ui.detailLinksOrder 迁移通道，loadSettings 传入；无旧配置 = 注册表稿序）。
 * 2026-10-12 提示词挪自定义组件 tab 迁移：旧 basic.hidden["prompt"]（2026-09-26 二轮挪入基础信息
 * 后被用户隐藏）与更早的独占「提示词」tab hidden 一并并入 custom.hidden——已隐藏提示词 =
 * 挪入自定义组件后保持隐藏；两处 order 不迁（basic 侧 mergeOrder 未知 id 自动丢弃，custom 按稿序补位） */
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
  const basicHidden = s.basic?.hidden;
  const promptTabHidden = s.prompt?.hidden;
  const legacyPromptHidden =
    (Array.isArray(basicHidden) && basicHidden.includes("prompt")) ||
    (Array.isArray(promptTabHidden) && promptTabHidden.includes("prompt"));
  if (legacyPromptHidden && !out.custom.hidden.includes("prompt")) {
    out.custom.hidden.push("prompt");
  }
  return out;
}

/** 详情渲染序 = 全量序过滤隐藏项（TabBasic / TabLinks / TabCustom 共用） */
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
  /** 提示词绑定（提示词轮 get_asset_detail 输出）：null/缺 = 未指定；assetId 非空 = 已指定；
   *  assetId null 且有 content = 已扫描（分析快照） */
  prompt?: PromptBinding | null;
}

/** 详情面板展示目标：选中文件卡片 / 选中档案卡片 / 选中普通文件夹（7.1 二十三轮文件夹详情形态）/
 *  快速访问视口（未选中文件时显示当前视口信息，同日用户定；quick = 全部/未分类等快速访问）/
 *  工作空间磁盘实况文件（2026-09-26：ws 合成项 assetId = ws: 假 id 禁发 assetId 命令，
 *  数据 = ws store 目录缓存条目 WsEntry + 所在目录绝对路径 dirPath；简化基础信息形态，
 *  不进三 tab / ui.detailBlocks 配置体系） */
export type DetailTarget =
  | { kind: "file"; placement: PlacementView; detail: AssetDetail }
  | { kind: "archive"; node: FolderNode; data: ArchiveDetailView }
  | { kind: "folder"; node: FolderNode }
  | { kind: "quick"; viewKey: string; name: string }
  | { kind: "ws"; entry: WsEntry; dirPath: string | null };

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
    prompt: null, // 提示词绑定（提示词轮）：未建档/未拉取 = 未指定
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
