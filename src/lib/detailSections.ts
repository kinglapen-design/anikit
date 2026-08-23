// 详情面板 8 区块体系（2026-08-15 用户定案，3.2 一轮）：文件与档案共用同一套固定区块——
// 自定义数据 / 基础信息 / 副本 / 链接区块 / 归属档案 / 关系 / 音频 / 提示词（顺序 = 22 稿）。
// 无内容区块 = 显示空数据（不隐藏，标题照常）；排序 + 显隐在 22 稿「编辑数据区块」面板操作，
// 配置持久化在 ui store（detailSections）；一轮全部区块只读（编辑态归二轮）。
import type { ArchiveDetailView, FolderNode, PlacementView } from "../stores/content";

export interface DetailSectionDef {
  id: string;
  title: string;
}

/** 固定 7 区块注册表（顺序 = 22 稿「编辑数据区块」面板行序）：
 *  2026-08-15 定案：缩略图区块（pinned 缩略图+颜色 bar+名称/链接输入框）独立永远置顶，
 *  不进注册表（不可隐藏/排序，DetailPanel 固定渲染）；"custom" 语义 = 纯自定义数据区块
 *  （带标题：属性 chips + 描述 + 编辑描述）；归属档案区块整体删除（存量配置 id 合并时忽略） */
export const DETAIL_SECTIONS: DetailSectionDef[] = [
  { id: "custom", title: "自定义数据" },
  { id: "info", title: "基础信息" },
  { id: "copies", title: "副本" },
  { id: "links", title: "链接区块" },
  { id: "relations", title: "关系" },
  { id: "audio", title: "音频" },
  { id: "prompt", title: "提示词" },
];

/** 区块标题（注册表查不到 = 原样返回 id 兜底） */
export function detailSectionTitle(id: string): string {
  return DETAIL_SECTIONS.find((s) => s.id === id)?.title ?? id;
}

/** 区块排序 / 显隐配置（ui store 持久化形态） */
export interface DetailSectionPref {
  id: string;
  visible: boolean;
}

/** 持久化配置与注册表兜底合并：配置里已删 / 未知 id 忽略；
 *  注册表新增区块按注册表相对顺序补位（插到它之后第一个已配置区块之前），默认可见 */
export function mergeDetailSections(saved: unknown): DetailSectionPref[] {
  const known = new Set(DETAIL_SECTIONS.map((s) => s.id));
  const result: DetailSectionPref[] = [];
  if (Array.isArray(saved)) {
    for (const it of saved) {
      if (!it || typeof it !== "object") continue;
      const { id, visible } = it as { id?: unknown; visible?: unknown };
      if (typeof id !== "string" || !known.has(id) || result.some((r) => r.id === id)) continue;
      result.push({ id, visible: visible !== false });
    }
  }
  for (let i = 0; i < DETAIL_SECTIONS.length; i++) {
    const def = DETAIL_SECTIONS[i];
    if (result.some((r) => r.id === def.id)) continue;
    let idx = result.length;
    for (let j = i + 1; j < DETAIL_SECTIONS.length; j++) {
      const p = result.findIndex((r) => r.id === DETAIL_SECTIONS[j].id);
      if (p >= 0) {
        idx = p;
        break;
      }
    }
    result.splice(idx, 0, { id: def.id, visible: true });
  }
  return result;
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
}

/** 详情面板展示目标：选中文件卡片 / 选中档案卡片（无选中 = null，空面板不渲染区块） */
export type DetailTarget =
  | { kind: "file"; placement: PlacementView; detail: AssetDetail }
  | { kind: "archive"; node: FolderNode; data: ArchiveDetailView };

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
