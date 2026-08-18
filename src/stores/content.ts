import { markRaw } from "vue";
import { defineStore } from "pinia";
import { Channel, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { isStaticImageExt } from "../lib/formatBadge";
import { playFail, playSuccess } from "../lib/sounds";
import { useSelectionStore } from "./selection";
import { useToastStore } from "./toast";
import { useUiStore } from "./ui";
import type { RecentAssetView, RecycleEntryView } from "./quickViews";

// 与后端 commands/import.rs 输出一一对应（camelCase，07_接口文档）
export interface PlacementView {
  assetId: string;
  folderId: string;
  name: string;
  w: number | null; // fla 等无尺寸为 null，布局按 1:1 默认比例（4.2：无尺寸资产显示方形格式图标）
  h: number | null;
  ext: string;
  thumbPath: string | null; // 缩略图绝对路径，无则卡片显示占位底色
  thumbKind: "asset" | "formatIcon" | null; // 缩略图来源：格式图标为方形，卡片 contain 显示不裁切（4.2）
  origin: boolean;
  frameCount: number | null; // 序列帧帧数；动图（GIF/WEBP 动图）同字段（5.1）；皆非为 null
  animated: boolean; // 动图标记（5.1）：与序列帧区分——序列帧显示"序列帧"角标/后缀，动图显示紫角标
  duration: number | null; // 音视频 / 动图时长（秒，5.1）；无时长资源为 null——卡片标签显示 mm:ss
  bpm: number | null; // 音频 BPM（只读内嵌标签，5.1）；无标签为 null
  importedFrom: string | null; // 导入渠道标记；"inbox" 且未摘标（root）= 收件箱"已导入"持久显示
  noteCount?: number | null; // 逐帧笔记条数冗余（5.2 展示层：卡片笔记角标据此显隐）；无笔记为 null
  thumbFrame?: string | null; // 封面帧：序列帧/动图 = 帧名；视频 = `t:{秒}`（5.2 写入，5.3 悬停播放范围起点）
  fps?: number | null; // 帧率：序列帧 = 播放帧率；视频 = 素材自身帧率（5.3 帧化：帧号显示 / 逐帧步进按 1/fps）
}

// 最近使用 / 回收站视图 DTO（6.1 二轮）已拆至 stores/quickViews.ts（规范第 3 条行数控制），
// 此处 re-export 保既有引用点不动
export type { RecentAssetView, RecycleEntryView } from "./quickViews";

export interface FolderNode {
  id: string;
  name: string;
  count: number; // 该文件夹递归 Placement 总数（含全部子文件夹，2026-07-25 用户定）
  // 节点形态（2026-08-15 档案子系统一轮）："archive" = 档案；缺省 = 普通文件夹（所有层级都带）
  kind?: string | null;
  // 所属区块 id（2026-08-14 区块子系统）：仅顶级节点有值语义，null/缺省 = 默认区块「目录」
  block?: string | null;
  // 自定义图标名（assets/icons/05_custom-icon 文件干；顶级文件夹新建时继承区块默认图标）；
  // 全层级都带（2026-08-18，3.3 起子级也可持自定义图标），缺省 = 默认文件夹图标
  icon?: string | null;
  // 自定义颜色档名（2026-08-18，3.3：--custom-{色} token 名）；仅本节点持有——
  // 子级视觉跟随 = 该色底色版本（FolderTree tint 传递承担），缺省 = 未自定义
  color?: string | null;
  children: FolderNode[];
}

// 默认区块「目录」id（后端 core/models.rs DEFAULT_BLOCK_ID；禁删可改名）
export const DEFAULT_BLOCK_ID = "dir";

// 档案详细数据（2026-08-15 档案子系统一轮，后端 core/models.rs ArchiveData）：
// 一轮消费 = 档案卡片徽标（badge）；头像（avatar = 封面，随 22 稿编辑面板）与
// 属性 / 描述（props / descriptions，随详情面板区块体系）后续任务接入
export interface ArchiveBadge {
  text: string;
  color: string; // 徽标底色（自定义色值，芯片 background 直用）
}

export interface ArchiveAvatar {
  assetId: string;
  crop?: { x: number; y: number; scale: number } | null;
}

export interface ArchiveData {
  folderId: string;
  badge?: ArchiveBadge | null;
  avatar?: ArchiveAvatar | null;
  props: string[];
  descriptions: { title: string; content: string }[];
  links: string[]; // 链接（详情 pinned 链接输入框数据源，2026-08-15 任务 A 三项）
  createdAt: string; // 创建时间 RFC3339；旧数据 / 未建档 = 空串
}

/** 档案详情视图（get_archive_data，2026-08-15 任务 A 三项）：ArchiveData 全字段 + 统计 */
export interface ArchiveDetailView {
  folderId: string;
  badge?: ArchiveBadge | null;
  avatar?: ArchiveAvatar | null;
  props: string[];
  descriptions: { title: string; content: string }[];
  links: string[];
  createdAt: string;
  fileCount: number; // 档案内 Placement 数
  totalSize: number; // 档案内资产 size 合计（同资产去重）
}

// 区块视图（后端 commands/import.rs BlockView；list_folder_tree 按 order 排序输出）
export interface BlockView {
  id: string;
  name: string;
  icon: string | null; // 区块默认图标（自定义图标名），null = 无
}

// list_folder_tree 返回（2026-08-14 区块子系统 breaking：原直接返回 FolderNode[]）
export interface FolderTreeView {
  blocks: BlockView[];
  nodes: FolderNode[];
}

// 区块 + 其顶级文件夹节点（侧栏 / 文件夹选择弹窗分段渲染用）
export interface BlockSection {
  block: BlockView;
  nodes: FolderNode[];
}

export interface ImportDraft {
  kind: "files" | "folder" | "mixed";
  paths: string[];
  targetFolder: string | null; // null = 根目录
}

export interface FailedItem {
  path: string;
  reason: string;
}

// hash 冲突挂起项（后端 services/import.rs ConflictItem，携带双侧显示数据供 10_重复添加提示）
export interface ConflictItem {
  sourcePath: string; // 源文件绝对路径（序列帧冲突为首帧路径，弹窗预览首帧）
  sourceName: string; // 源文件名（含扩展名）；序列帧冲突为 标题.扩展名
  sourceSize: number;
  sourceW: number | null; // 图片宽高（非图片 null）
  sourceH: number | null;
  sourceThumbPath: string | null; // webview 直渲不了的格式（PSD/PSB/SVG 等）后端生成的预览缓存（cache/tmp），图片格式为 null 走直读源文件
  targetFolder: string; // 落点文件夹 ID（"root" 或 F 开头）
  targetFolderName: string; // 落点文件夹显示名（root → "根目录"）
  existingAssetId: string;
  existingName: string; // 已有资源显示名（origin Placement 名）
  existingW: number | null;
  existingH: number | null;
  existingSize: number;
  existingExt: string;
  existingThumbPath: string | null;
  existingFolderName: string; // 已有资源所在文件夹显示名
  sequence?: SequenceSource | null; // 序列帧组信息（原样回传给 resolve_conflicts，4.1）
  importedFrom?: string | null; // 本批次导入渠道（原样回传；"保留两个"新资产继承该标记进"已导入"视图）
}

// 序列帧组信息（4.1）：冲突项携带、决策时原样回传，"保留两个"按此重新打包
export interface SequenceSource {
  name: string; // 基础名（资产标题）
  ext: string;
  width: number; // 序号位宽
  frames: { path: string; number: number }[]; // 按序号升序
}

// detect_sequences 返回的组摘要（12_序列帧弹窗弹出依据）
export interface SequenceGroupInfo {
  name: string;
  ext: string;
  frameCount: number;
}

// 冲突决策（后端 commands resolve_conflicts 入参）
export interface ConflictResolution {
  sourcePath: string;
  existingAssetId: string;
  targetFolder: string;
  action: "useExisting" | "copy" | "link" | "skip";
  sequence?: SequenceSource | null; // 序列帧冲突原样回传（4.1）
  sourceThumbPath?: string | null; // 预览缓存原样回传（决策完成后后端删除，4.2）
  importedFrom?: string | null; // 批次渠道原样回传（copy 新资产继承该标记，2026-08-13）
}

// 导入任务事件（后端 services/import.rs ImportEvent，serde tag="type" camelCase）
export type ImportEvent =
  | { type: "started"; total: number }
  | { type: "folders" } // keep 模式层级已建，重拉文件树
  | { type: "asset"; placement: PlacementView } // 到一个显示一个
  | { type: "finished"; imported: number; conflicts: ConflictItem[]; failed: FailedItem[] };

// 导出任务事件（后端 services/asset_menu.rs ExportEvent，serde tag="type" camelCase）：
// 单位 = 文件（序列帧按帧计）；finished.error = 整体失败（目录不可写等），finished.failed = 失败清单
export type ExportEvent =
  | { type: "started"; total: number }
  | { type: "item" } // 每处理一个文件推一个（成败皆推，进度恒走完）
  | { type: "finished"; copied: number; failed: ExportFailedItem[]; error: string | null };

// 导出失败项（后端 ExportFailedItem）：path = 源文件（弹窗展示取文件名）；
// target = 首跑实际尝试的完整目标路径（重试原路覆盖用，原样回传）
export interface ExportFailedItem {
  path: string;
  target: string;
  reason: string;
}

// 收件箱磁盘实况条目（后端 services/inbox.rs InboxEntry；"工程文件"区块数据源，
// 磁盘为事实来源——不建档不写库，热刷新 = 全量重扫替换）
export interface InboxDiskEntry {
  name: string; // 顶层条目名（含扩展名）
  isDir: boolean;
  size: number; // 字节（文件夹递归；快层快照中目录先给 0，inbox-disk-details 事件补齐）
  modifiedAt: number; // Unix 毫秒
  autoImportable: boolean; // 白名单判定；false 的留在磁盘等手动导入
  occupied: boolean; // 被其他软件占用（占用条目禁止导入 / 重命名）
  iconPath: string | null; // 系统格式图标缓存绝对路径；null 时回退扩展名角标（目录恒 null）
}

// 流式追加节流：asset 事件先进缓冲，每 250ms 批量入列——
// 逐条 push 会让 10 万级列表每文件重排一次（布局 ~10ms×N 必卡），批量后重排 ≤4 次/秒
const FLUSH_MS = 250;
let assetBuffer: PlacementView[] = [];
let flushTimer: ReturnType<typeof setTimeout> | undefined;
// 流式导入前插锚点（2026-07-25 用户定：新导入排视口最前）：
// 本批已前插数——flush 时 splice 到锚点位并后移（批内保序，与后端 add_placement 锚点同语义；
// finished 后 refresh 全量对齐，目标文件夹段首位置以后端为准）
let importAnchor: number | null = null;

// 收件箱事件监听（3.2）：App 存活期注册一次，后端监听 inbox 目录自动入库
let unlistenInbox: (() => void) | undefined;
// 存量视频 fps 自愈事件监听（2026-07-28 复验反馈修复）：App 存活期注册一次
let unlistenFpsHeal: (() => void) | undefined;
// 最近使用变更事件监听（2026-07-29 复验反馈修复：原仅全量 refresh 拉取、要重启才更新）
let unlistenRecent: (() => void) | undefined;

// 视口滚动位置（会话级，2026-07-25 用户定）：进出预览 / 切换目录都恢复——
// 预览返回不跳顶（设缩略图后可直接确认），对比两个文件夹不用重新翻
const scrollPositions = new Map<string, number>();

// 递归查文件夹节点（stripFolders / 标题栏 crumbs 共用）
// 快速访问视口文案（1.2 多标签页：标签页标题 / 面包屑 / 本 store 视口计算共用）
// inbox = PRD 02-15"自动导入"（收件箱渠道候车区，2026-08-08 对齐新稿命名）
export const QUICK_LABELS: Record<string, string> = {
  all: "全部",
  uncategorized: "未分类",
  inbox: "自动导入",
  recent: "最近使用",
  thumbnail: "缩略图", // hidden 资产业务 5.x 接（PRD 02-15），当前空视图
  recycle: "回收站",
};

function findFolder(nodes: FolderNode[], id: string): FolderNode | null {
  for (const n of nodes) {
    if (n.id === id) return n;
    const hit = findFolder(n.children, id);
    if (hit) return hit;
  }
  return null;
}

export const useContentStore = defineStore("content", {
  state: () => ({
    // 10 万级：全量赋值一次完成（元素 markRaw 追加），数组保持可追踪以便流式 push 触发
    placements: [] as PlacementView[],
    folders: markRaw([]) as FolderNode[],
    // 区块表（2026-08-14 区块子系统）：list_folder_tree 一并返回，按 order 排序；空兜底默认区块
    blocks: markRaw([]) as BlockView[],
    // 快速访问"最近使用 / 回收站"（6.1 二轮，2026-07-29）：refresh 一并拉取，失败静默降级
    recentAssets: [] as RecentAssetView[],
    recycleEntries: [] as RecycleEntryView[],
    // 档案详细数据（2026-08-15 档案子系统一轮）：folderId → ArchiveData，refresh 一并拉取装配
    archives: {} as Record<string, ArchiveData>,
    // 左栏选中项：quick:all / quick:uncategorized… / folder:{id}
    sidebarSelected: "quick:all",
    // 选中互斥：素材卡片选中（selection store）与收件箱条目选中互斥。
    // 素材卡片选中键已迁 selection store（6.1 多选改造），selectedAssetKey 为下方派生 getter；
    // 文件夹卡片 / 树 item 多选同迁 selection store 文件夹选择域（2026-07-28 复验反馈 5）
    // 导入弹窗草稿：菜单 / 拖拽触发后填入，ImportDialog 读取；确认或取消后清空
    importDraft: null as ImportDraft | null,
    // 序列帧弹窗（12_序列帧弹窗，4.1）：09 确认后 detect_sequences 命中时填入；
    // 确认（打包/分散）或取消（放弃整个导入）后清空
    sequencePrompt: null as {
      draft: ImportDraft;
      structure: "flatten" | "keep";
      source: "copy" | "move";
    } | null,
    // 后台导入进度（11_导入进度条）；total=0 表示仍在扫描（总数未知）
    importProgress: null as {
      taskId: number | null;
      total: number;
      done: number;
      startedAt: number;
    } | null,
    // 后台导出进度（2026-08-14 用户定：复用导入进度条样式与位置，ImportProgressBar 双态）；
    // total=0 表示仍在清点（Started 未到）
    exportProgress: null as {
      taskId: number | null;
      total: number;
      done: number;
      startedAt: number;
    } | null,
    // 失败清单（FailedListDialog）与冲突挂起队列（DuplicateDialog 逐个询问）
    importFailed: null as FailedItem[] | null,
    // 导出失败清单（FailedListDialog，2026-08-14 用户定：导出也有失败弹窗 + 重试）
    exportFailed: null as ExportFailedItem[] | null,
    importConflicts: null as ConflictItem[] | null,
    // 收件箱磁盘实况（4.4"工程文件"区块）：磁盘为事实来源，不建档不写库；
    // inbox-fs-changed 事件（去抖 150ms）与手动操作后全量重扫替换
    inboxDisk: [] as InboxDiskEntry[],
    // 选中互斥第三态：磁盘实况条目选中名（与素材 / 文件夹选中（均在 selection store）互斥）
    selectedInboxName: "",
    // 原导入的来源模式（冲突"保留两个"决策继承；剪贴板导入恒 move）
    lastImportSource: "copy" as "copy" | "move",
    // 原导入的完整参数（导入失败弹窗"重试"按原参数重投失败项，2026-08-14 用户定）；
    // 仅 startImport 记录（剪贴板 / 收件箱手动导入不记，重试回退 keep + scatter + 当前目录）
    lastImportParams: null as {
      structure: "flatten" | "keep";
      sequenceMode: "pack" | "scatter";
      targetFolder: string | null;
    } | null,
  }),
  getters: {
    // 素材卡片选中键（6.1 多选改造，2026-07-28 用户定案）：转发 selection store 首项——
    // 详情面板 / 预览等单资产消费方原样不动；空串 = 无选中（语义同原 state 字段）
    selectedAssetKey(): string {
      return useSelectionStore().first;
    },
    // 按视口 key 计算资源列表（1.2 多标签页：每个标签页各自视口，原 visiblePlacements 逻辑参数化；
    // "最近使用 / 回收站"已接真实视图，6.1 二轮：元素为 PlacementView 超集，AssetGrid 零改动复用）
    placementsFor(state) {
      return (viewKey: string): PlacementView[] => {
        if (viewKey.startsWith("folder:")) {
          const id = viewKey.slice(7);
          // 递归显示（ui.showSubfolders，2026-08-10 接入）：开 = 视口含全部后代文件夹的 Placement；
          // 关 = 仅当前文件夹直接 Placement
          if (!useUiStore().showSubfolders) return state.placements.filter((p) => p.folderId === id);
          const ids = new Set<string>();
          const collect = (n: FolderNode): void => {
            ids.add(n.id);
            n.children.forEach(collect);
          };
          const root = findFolder(state.folders, id);
          if (root) collect(root);
          else ids.add(id); // 失效 viewKey（文件夹已删）：退化为直接匹配
          return state.placements.filter((p) => ids.has(p.folderId));
        }
        switch (viewKey) {
          // 2026-08-13 用户定案：快速访问不做内容限制——按实际 Placement 显示，
          // 链接不再隐藏（原 originOnly 资产级去重 + origin 过滤整体废除）
          case "quick:all": // 全部 = 递归显示所有文件（2026-07-22 用户定案）
            return state.placements;
          case "quick:uncategorized": // 未分类 = 未归入任何文件夹（root 即未分类）
            return state.placements.filter((p) => p.folderId === "root");
          case "quick:inbox": // 收件箱"已导入" = importedFrom=inbox 且未摘标的资产，持久显示
            // （2026-07-25 用户定：用户没分类走就要一直在）。不限 folderId=root——手动导入 Keep
            // 生成的层级也算未分类；摘标 = 清 importedFrom（归 6.3），清后自然消失
            return state.placements.filter((p) => p.importedFrom === "inbox");
          case "quick:recent": // 最近使用（后端已按时间倒序）
            return state.recentAssets;
          case "quick:recycle": // 回收站（folderId 恒 "recycle"，key 与素材域互不干扰）
            return state.recycleEntries;
          default:
            return [];
        }
      };
    },
    // 当前视口的资源列表（激活标签页视口；操作上下文统一走这个——交互发生时所在 panel 必为激活）
    visiblePlacements(state): PlacementView[] {
      return this.placementsFor(state.sidebarSelected);
    },
    // 视口标题（1.2 多标签页：标签页标题 / 面包屑共用）：folder → 实时文件夹名（重命名跟随）；
    // quick 五项 → 固定文案；工作空间/标签等占位视图 → "全部"兜底（调用方可另传 fallback）
    viewTitle(state) {
      return (viewKey: string): string => {
        if (viewKey.startsWith("folder:")) {
          return findFolder(state.folders, viewKey.slice(7))?.name ?? "文件夹";
        }
        return QUICK_LABELS[viewKey.slice(6)] ?? "全部";
      };
    },
    // 回收站视口判定（6.1 二轮，2026-07-29）：只读语义统一判断点——禁拖拽 / 禁重命名 / Del 分流彻底删除
    isRecycleView(state): boolean {
      return state.sidebarSelected === "quick:recycle";
    },
    // 快速访问 item 数量（4.4 查缺补漏）：全部 / 未分类 / 收件箱接真实统计；
    // 最近使用 / 回收站留 6.1，工作空间 / 标签 / 快捷入口占位视图不接（对应阶段未开工）；
    // 2026-08-13 起与视图同口径：按实际 Placement 计数（链接不再隐藏）
    allCount(state): number {
      return state.placements.length;
    },
    uncategorizedCount(state): number {
      return state.placements.filter((p) => p.folderId === "root").length;
    },
    inboxCount(state): number {
      return state.placements.filter((p) => p.importedFrom === "inbox").length;
    },
    // 文件夹卡片横排（03_中间面板）：仅文件夹视图显示直接子文件夹；
    // 快速访问视图一律不出文件夹区块（2026-07-22 用户定案；回收站保留文件夹随对应阶段另议）。
    // 1.2 多标签页：按视口 key 参数化（每个标签页各自视口）；
    // 档案（kind = archive，3.2 一轮）从文件夹区摘出——档案区独立区块（stripArchivesFor）
    stripFoldersFor(state) {
      return (viewKey: string): FolderNode[] => {
        if (viewKey.startsWith("folder:")) {
          return (findFolder(state.folders, viewKey.slice(7))?.children ?? []).filter(
            (n) => n.kind !== "archive",
          );
        }
        return [];
      };
    },
    // 档案区（3.2 一轮，PRD 02-03 中间面板三区：文件夹 / 档案 / 文件）：仅文件夹视图的直接子级档案；
    // 快速访问视图同文件夹区口径一律不出
    stripArchivesFor(state) {
      return (viewKey: string): FolderNode[] => {
        if (viewKey.startsWith("folder:")) {
          return (findFolder(state.folders, viewKey.slice(7))?.children ?? []).filter(
            (n) => n.kind === "archive",
          );
        }
        return [];
      };
    },
    // 激活视口的子级档案（操作/显示上下文跟随激活标签页）
    stripArchives(state): FolderNode[] {
      return this.stripArchivesFor(state.sidebarSelected);
    },
    // 激活视口的子文件夹（操作/显示上下文跟随激活标签页）
    stripFolders(state): FolderNode[] {
      return this.stripFoldersFor(state.sidebarSelected);
    },
    // 文件夹卡片缩略图（4.4）：每个文件夹的直接子项中，按 placements 顺序取第一个静态图片；
    // 排除序列帧/动图（frameCount）与 GIF/工程/矢量格式，右键自定义缩略图归 6.3
    folderThumbMap(state): Map<string, string> {
      const map = new Map<string, string>();
      for (const p of state.placements) {
        if (map.has(p.folderId)) continue;
        if (p.frameCount != null || p.thumbKind !== "asset" || !p.thumbPath) continue;
        if (!isStaticImageExt(p.ext)) continue;
        map.set(p.folderId, p.thumbPath);
      }
      return map;
    },
    // 按区块分组的顶级节点（2026-08-14 区块子系统）：block null/"dir"/失效引用均归默认区块「目录」；
    // 后端开库自检保证 blocks 非空且含 dir，空数组兜底合成默认区块（并行开发期降级）
    blockSections(state): BlockSection[] {
      const blocks: BlockView[] =
        state.blocks.length > 0 ? state.blocks : [{ id: DEFAULT_BLOCK_ID, name: "目录", icon: null }];
      const known = new Set(blocks.map((b) => b.id));
      const ofBlock = (n: FolderNode): string => {
        const b = n.block ?? DEFAULT_BLOCK_ID;
        return known.has(b) ? b : DEFAULT_BLOCK_ID;
      };
      return blocks.map((block) => ({ block, nodes: state.folders.filter((n) => ofBlock(n) === block.id) }));
    },
    // 当前视口对应的导入目标文件夹（菜单触发导入用；"全部"= 根目录 null）
    currentFolderId(state): string | null {
      return state.sidebarSelected.startsWith("folder:")
        ? state.sidebarSelected.slice(7)
        : null;
    },
    folderName: (state) => {
      return (id: string): string | null => findFolder(state.folders, id)?.name ?? null;
    },
  },
  actions: {
    // 视口滚动位置存取（key = sidebarSelected；会话级 Map，零响应式开销）
    saveScrollPos(key: string, pos: number): void {
      scrollPositions.set(key, pos);
    },
    getScrollPos(key: string): number | null {
      return scrollPositions.get(key) ?? null;
    },

    // 逐帧笔记条数冗余同步（5.2 展示层）：set_note 返回全量后由前端算条数更新——
    // 同一资产的全部 Placement（多文件夹链接）一并刷新，卡片角标 / 详情区块即时跟随
    setNoteCount(assetId: string, count: number): void {
      const v = count > 0 ? count : null;
      for (const p of this.placements) {
        if (p.assetId === assetId) p.noteCount = v;
      }
    },

    // 库打开后 / 导入完成后全量刷新（增量刷新随阶段 4 任务队列再做）
    async refresh(): Promise<void> {
      const [placements, tree] = await Promise.all([
        invoke<PlacementView[]>("list_placements", { folder: null }),
        invoke<FolderTreeView>("list_folder_tree"),
      ]);
      // 数组保持可追踪（流式 push 触发）；元素由 Vue 惰性代理，10 万级只代理可视区
      this.placements = placements;
      this.folders = markRaw(tree.nodes);
      this.blocks = markRaw(tree.blocks);
      // 最近使用 / 回收站 / 档案数据一并拉（6.1 二轮 / 3.2 一轮）：保持简单——操作后 refresh 即全量对齐
      await Promise.all([this.refreshRecent(), this.refreshRecycle(), this.refreshArchives()]);
      // 选择态剔除已不存在的 key（三域就位后统一 prune——回收站 key 存活以 recycleEntries 为准）
      useSelectionStore().prune();
    },
    // 快速访问"最近使用"（6.1 二轮）：失败静默降级——后端并行开发期命令未就绪不阻断主流程
    async refreshRecent(): Promise<void> {
      try {
        this.recentAssets = await invoke<RecentAssetView[]>("list_recent_assets");
      } catch (e) {
        console.warn("最近使用刷新失败（静默降级）：", e);
      }
    },
    // 回收站（6.1 二轮）：同 refreshRecent 静默降级口径
    async refreshRecycle(): Promise<void> {
      try {
        this.recycleEntries = await invoke<RecycleEntryView[]>("list_recycle");
      } catch (e) {
        console.warn("回收站刷新失败（静默降级）：", e);
      }
    },
    // 档案详细数据（3.2 一轮）：list_archives 装配成 folderId → ArchiveData map；
    // 同 refreshRecent 静默降级口径——徽标等是增强展示，不阻断主流程
    async refreshArchives(): Promise<void> {
      try {
        const list = await invoke<ArchiveData[]>("list_archives");
        const map: Record<string, ArchiveData> = {};
        for (const a of list) map[a.folderId] = a;
        this.archives = map;
      } catch (e) {
        console.warn("档案数据刷新失败（静默降级）：", e);
      }
    },
    async refreshFolders(): Promise<void> {
      const tree = await invoke<FolderTreeView>("list_folder_tree");
      this.folders = markRaw(tree.nodes);
      this.blocks = markRaw(tree.blocks);
    },

    // 09 导入弹窗确认后（4.1）：先经后端检测所选路径中的序列帧——
    // 命中则追加弹 12_序列帧弹窗（打包/分散），未命中按现状直接后台导入；
    // 检测异常不阻塞导入（按分散处理）
    async probeSequences(
      draft: ImportDraft,
      structure: "flatten" | "keep",
      source: "copy" | "move",
    ): Promise<void> {
      try {
        const groups = await invoke<SequenceGroupInfo[]>("detect_sequences", { paths: draft.paths });
        if (groups.length > 0) {
          this.sequencePrompt = { draft, structure, source };
          return;
        }
      } catch (e) {
        console.error("序列帧检测失败，按分散导入：", e);
      }
      await this.startImport(draft, structure, source, "scatter");
    },

    // 12_序列帧弹窗确认：打包导入预览（pack）/ 分散图片导入（scatter）；取消 = 放弃整个导入
    async confirmSequenceImport(mode: "pack" | "scatter"): Promise<void> {
      const prompt = this.sequencePrompt;
      this.sequencePrompt = null;
      if (prompt) await this.startImport(prompt.draft, prompt.structure, prompt.source, mode);
    },

    // 后台导入任务：命令立即返回 task_id，进度经 Channel 推送（异步不阻塞 UI）；
    // sequenceMode：序列帧打包（pack）或逐文件分散导入（scatter）
    async startImport(
      draft: ImportDraft,
      structure: "flatten" | "keep",
      source: "copy" | "move",
      sequenceMode: "pack" | "scatter",
    ): Promise<void> {
      this.lastImportSource = source;
      this.lastImportParams = { structure, sequenceMode, targetFolder: draft.targetFolder };
      this.importProgress = { taskId: null, total: 0, done: 0, startedAt: Date.now() };
      const onEvent = new Channel<ImportEvent>();
      onEvent.onmessage = (e) => this.onImportEvent(e);
      const taskId = await invoke<number>("import_paths", {
        paths: draft.paths,
        structure,
        source,
        targetFolder: draft.targetFolder,
        sequenceMode,
        onEvent,
      });
      if (this.importProgress && this.importProgress.taskId === null) {
        this.importProgress.taskId = taskId;
      }
    },
    async startImportClipboard(targetFolder: string | null): Promise<void> {
      this.lastImportSource = "copy"; // 剪贴板统一 Copy（2026-07-27 定：绝不移动用户原文件）
      this.importProgress = { taskId: null, total: 0, done: 0, startedAt: Date.now() };
      const onEvent = new Channel<ImportEvent>();
      onEvent.onmessage = (e) => this.onImportEvent(e);
      try {
        const taskId = await invoke<number>("import_clipboard", { targetFolder, onEvent });
        if (this.importProgress && this.importProgress.taskId === null) {
          this.importProgress.taskId = taskId;
        }
      } catch (e) {
        this.importProgress = null; // 失败复位进度条（剪贴板无图无文件等，防卡"扫描中"）
        throw e;
      }
    },

    // 取消导入（11_导入进度条）：进度条立即隐藏；后台把当前文件写完后真实停止（cancel 语义）
    async cancelImport(): Promise<void> {
      const p = this.importProgress;
      this.importProgress = null;
      if (p?.taskId != null) {
        await invoke("cancel_import", { taskId: p.taskId }).catch(() => {});
      }
    },

    // 取消导出（同导入口径）：进度条立即隐藏；后台把当前文件拷完后真实停止
    async cancelExport(): Promise<void> {
      const p = this.exportProgress;
      this.exportProgress = null;
      if (p?.taskId != null) {
        await invoke("cancel_export", { taskId: p.taskId }).catch(() => {});
      }
    },

    // 导出任务编排（placementOps 两入口与失败重试共用）：进度初值 + Channel 事件流 +
    // taskId 回填；起动失败复位进度条 + 失败 toast（防卡"清点中"）
    async runExportTask(start: (onEvent: Channel<ExportEvent>) => Promise<number>): Promise<void> {
      this.exportProgress = { taskId: null, total: 0, done: 0, startedAt: Date.now() };
      const onEvent = new Channel<ExportEvent>();
      onEvent.onmessage = (e) => this.onExportEvent(e);
      try {
        const taskId = await start(onEvent);
        if (this.exportProgress && this.exportProgress.taskId === null) {
          this.exportProgress.taskId = taskId;
        }
      } catch (e) {
        this.exportProgress = null;
        useToastStore().notify({ message: String(e), icon: "fail" });
      }
    },

    // 重试导出失败项（失败弹窗主按钮，2026-08-14 用户定）：failed 原样回传 export_retry——
    // 后端按首跑记录的 源→目标 原路覆盖，成功项不重导、不撞无关文件；再失败弹窗再现
    retryFailedExport(): void {
      const items = this.exportFailed ?? [];
      this.exportFailed = null;
      if (items.length === 0) return;
      void this.runExportTask((onEvent) => invoke<number>("export_retry", { items, onEvent }));
    },

    // 导出进度事件（placementOps 的 Channel 推入）：finished 收尾 toast / 失败弹窗——
    // 取消后的 finished 不再提示（进度条已隐藏，exportProgress 为 null 即取消标记）
    onExportEvent(e: ExportEvent): void {
      switch (e.type) {
        case "started":
          if (this.exportProgress) {
            this.exportProgress.total = e.total;
            this.exportProgress.startedAt = Date.now(); // ETA 从清点结束起算
          }
          break;
        case "item":
          if (this.exportProgress) this.exportProgress.done += 1;
          break;
        case "finished": {
          const wasCancelled = this.exportProgress === null;
          this.exportProgress = null;
          if (wasCancelled) break;
          if (e.error) {
            useToastStore().notify({ message: e.error, icon: "fail" });
          } else if (e.failed.length > 0) {
            this.exportFailed = e.failed; // 有失败项 → 失败弹窗（alert 模板自带提示音）
          } else {
            useToastStore().notify({ message: `已导出 ${e.copied} 个文件`, icon: "success", sound: true });
          }
          break;
        }
      }
    },

    // 冲突决策提交（10_重复添加提示）：逐项或"全部应用"批量，来源模式继承原导入
    async resolveConflicts(
      items: ConflictResolution[],
    ): Promise<{ resolved: number; failed: FailedItem[] }> {
      return await invoke("resolve_conflicts", { items, source: this.lastImportSource });
    },

    // 重试导入失败项（失败弹窗主按钮，2026-08-14 用户定）：按原导入参数重投失败路径；
    // 失败清单也可能来自冲突决策（DuplicateDialog），重投后撞 hash 会再走重复提示，语义自洽。
    // 无有效路径 = 纯关闭；剪贴板 / 收件箱渠道未记参数，回退 keep + scatter + 当前目录
    retryFailedImport(): void {
      const paths = (this.importFailed ?? []).map((f) => f.path).filter((p) => p.length > 0);
      this.importFailed = null;
      if (paths.length === 0) return;
      const params = this.lastImportParams ?? { structure: "keep" as const, sequenceMode: "scatter" as const, targetFolder: null };
      void this.startImport(
        { kind: "files", paths, targetFolder: params.targetFolder },
        params.structure,
        this.lastImportSource,
        params.sequenceMode,
      );
    },

    // 收件箱后台导入（PRD 05-01，3.2）：后端监听 inbox 目录自动入库，
    // 事件形状与导入 Channel 一致，复用 11_导入进度条 / 10_重复添加提示；
    // inbox-fs-changed（4.4）：磁盘实况热刷新信号，去抖 150ms 重扫快照
    async initInboxListener(): Promise<void> {
      if (unlistenInbox) return;
      unlistenInbox = await listen<ImportEvent>("inbox-import", (e) => {
        if (e.payload.type === "started" && !this.importProgress) {
          this.lastImportSource = "move"; // 收件箱恒 Move（过境：导入后即移走）
          this.importProgress = { taskId: null, total: 0, done: 0, startedAt: Date.now() };
        }
        this.onImportEvent(e.payload);
      });
      // 实况首次就位 + 监听热刷新（写入连发事件，去抖合并为一拍）
      void this.refreshInboxDisk();
      let fsTimer: ReturnType<typeof setTimeout> | undefined;
      await listen("inbox-fs-changed", () => {
        clearTimeout(fsTimer);
        fsTimer = setTimeout(() => void this.refreshInboxDisk(), 150);
      });
      // 慢层 details（递归大小 / 占用 / 格式图标）后台算好整体推送，全量替换快层快照：
      // 快层目录 size=0 / occupied=false 是已知过渡态，占用标记与目录大小以本事件合并后为准
      await listen<InboxDiskEntry[]>("inbox-disk-details", (e) => {
        this.inboxDisk = markRaw(e.payload);
      });
    },

    // 存量视频 fps 自愈通知（2026-07-28 复验反馈修复）：get_media_source 补探测写回后广播，
    // 同步该资产全部 Placement 的 fps（同 setNoteCount 多 Placement 口径）——
    // 卡片悬停胶囊（HoverVideoLayer 读 PlacementView.fps）与详情笔记即时正确，免重开窗口重拉；
    // 同一资产只自愈一次（写回后 metadata.fps 非空不再命中），天然不刷屏
    async initFpsHealListener(): Promise<void> {
      if (unlistenFpsHeal) return;
      unlistenFpsHeal = await listen<{ assetId: string; fps: number }>("video-fps-healed", (e) => {
        for (const p of this.placements) {
          if (p.assetId === e.payload.assetId) p.fps = e.payload.fps;
        }
      });
    },

    // 最近使用热刷新（2026-07-29 复验反馈：原仅全量 refresh 拉取、要重启才更新）——
    // 后端 open_with_default_app 记录成功广播 recent-changed，此处重拉视图与数量
    async initRecentListener(): Promise<void> {
      if (unlistenRecent) return;
      unlistenRecent = await listen("recent-changed", () => void this.refreshRecent());
    },

    // 拉取收件箱磁盘实况快层快照（4.4）：失败静默降级——实况是辅助展示，不阻断任何流程
    async refreshInboxDisk(): Promise<void> {
      try {
        this.inboxDisk = markRaw(await invoke<InboxDiskEntry[]>("list_inbox_snapshot"));
      } catch (e) {
        console.warn("收件箱磁盘实况刷新失败（静默降级）：", e);
      }
    },

    onImportEvent(e: ImportEvent): void {
      switch (e.type) {
        case "started":
          importAnchor = 0; // 新导入批次：从列表最前开始流入
          if (this.importProgress) {
            this.importProgress.total = e.total;
            this.importProgress.startedAt = Date.now(); // ETA 从扫描结束起算
          }
          break;
        case "folders":
          void this.refreshFolders();
          break;
        case "asset":
          assetBuffer.push(markRaw(e.placement));
          if (this.importProgress) this.importProgress.done += 1;
          if (!flushTimer) {
            flushTimer = setTimeout(() => {
              flushTimer = undefined;
              if (assetBuffer.length > 0) {
                const batch = assetBuffer.splice(0);
                if (importAnchor !== null) {
                  this.placements.splice(importAnchor, 0, ...batch);
                  importAnchor += batch.length;
                } else {
                  this.placements.push(...batch);
                }
              }
            }, FLUSH_MS);
          }
          break;
        case "finished":
          clearTimeout(flushTimer);
          flushTimer = undefined;
          assetBuffer = [];
          importAnchor = null;
          this.importProgress = null;
          if (e.failed.length > 0) this.importFailed = e.failed;
          // 冲突挂起队列：导入全部完成后逐个询问（10_重复添加提示）
          if (e.conflicts.length > 0) this.importConflicts = e.conflicts;
          // 导入完成提示音（2026-08-10 用户定：重要场合——干净完成成功音 / 有失败失败音）
          if (e.failed.length > 0) playFail();
          else playSuccess();
          void this.refresh(); // 兜底全量对齐（链接 / 层级 / 目标文件夹段首以最终态为准）
          break;
      }
    },
  },
});
