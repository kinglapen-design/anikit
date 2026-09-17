import { markRaw } from "vue";
import { defineStore } from "pinia";
import { Channel, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { isStaticImageExt } from "../lib/formatBadge";
import { sortPlacements } from "../lib/sortPlacements";
import { playFail, playSuccess } from "../lib/sounds";
import { useSelectionStore } from "./selection";
import { useToastStore } from "./toast";
import { useUiStore } from "./ui";
import type { RecentAssetView, RecycleEntryView } from "./quickViews";
import type { Relation } from "../lib/relationOps";
import { useWsStore } from "./ws";

// 与后端 commands/import.rs 输出一一对应（camelCase，07_接口文档）
export interface PlacementView {
  assetId: string;
  folderId: string;
  name: string;
  w: number | null; // 源文件尺寸（fla 等无尺寸为 null）；布局不用它——见 thumbW
  h: number | null;
  thumbW?: number | null; // 缩略图实际尺寸（5.4）：布局比例跟缩略图走——自定义缩略图后自适应，刷新缩略图回退；null 回退 w/h
  thumbH?: number | null;
  ext: string;
  thumbPath: string | null; // 缩略图绝对路径，无则卡片显示占位底色
  thumbKind: "asset" | "formatIcon" | null; // 缩略图来源：格式图标为方形，卡片 contain 显示不裁切（4.2）
  origin: boolean;
  frameCount: number | null; // 序列帧帧数；动图（GIF/WEBP 动图）同字段（5.1）；皆非为 null
  animated: boolean; // 动图标记（5.1）：与序列帧区分——序列帧显示"序列帧"角标/后缀，动图显示紫角标
  duration: number | null; // 音视频 / 动图时长（秒，5.1）；无时长资源为 null——卡片标签显示 mm:ss
  bpm: number | null; // 音频 BPM（只读内嵌标签，5.1）；无标签为 null
  importedFrom: string | null; // 导入渠道标记；"inbox" 且未摘标（root）= 收件箱"已导入"持久显示
  // —— 以下四项服务 3.5 排序（后端 views.rs 自 metadata 带出）——
  size: number; // 文件大小（字节）
  createdAt: string | null; // 源文件磁盘创建时间 RFC3339；存量数据为 null（排序排尾）
  modifiedAt: string; // 库内数据最后变更时刻 RFC3339
  importedAt: string; // 导入时刻 RFC3339
  noteCount?: number | null; // 逐帧笔记条数冗余（5.2 展示层：卡片笔记角标据此显隐）；无笔记为 null
  thumbFrame?: string | null; // 封面帧：序列帧/动图 = 帧名；视频 = `t:{秒}`（5.2 写入，5.3 悬停播放范围起点）
  fps?: number | null; // 帧率：序列帧 = 播放帧率；视频 = 素材自身帧率（5.3 帧化：帧号显示 / 逐帧步进按 1/fps）
  hidden: boolean; // hidden 伴生缩略图标记（8.1，PRD 02-15）：普通视图统一过滤，仅「缩略图」视图显示
  playAssetId?: string | null; // 播放源代理（8.1）：fla 等工程卡片动图播放的帧流开流目标（= hidden 伴生资产 id）；无代理为 null
  // 工作空间磁盘实况合成项专属（stores/ws.ts wsEntryToPlacement，PRD 02 步骤 1）：
  // 源文件绝对路径——预览按路径分支（图片直 convertFileSrc / 音视频 get_media_source_by_path）与
  // 卡片悬停层禁用判定用；库 Placement 无此字段。此时 assetId = `ws:${relPath}` 为合成假 id，
  // **严禁发 assetId 类命令**（判定走 stores/ws.ts isWsPlacement）
  wsPath?: string | null;
}

// 最近使用 / 回收站视图 DTO（6.1 二轮）已拆至 stores/quickViews.ts（规范第 3 条行数控制），
// 此处 re-export 保既有引用点不动
export type { RecentAssetView, RecycleEntryView } from "./quickViews";

export interface FolderNode {
  id: string;
  name: string;
  count: number; // 该文件夹递归 Placement 总数（含全部子文件夹，2026-07-25 用户定）
  // 递归容量（字节，2026-08-30 用户定案）：子树全部 Placement 按 asset 去重后 size 合计——
  // 同一资产在父子文件夹各放一份只计一次（占用空间语义，与 count 同口径同趟算出）
  totalSize: number;
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
  // 分组规则（2026-08-19，3.5 二轮 / PRD 02-04）：内容区分组显示配置，档案与普通文件夹通用；
  // 缺省 / 空 = 不分组。归组计算 = lib/grouping.ts（显示层，不改数据）
  groups?: import("../lib/grouping").GroupDef[] | null;
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
  file: string; // 封面文件名（cache/covers/ 内，6.1 独立存不入库、2026-09-08 挪 cache；展示用 coverPath 绝对路径）
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
  totalSize: number; // 档案容量（字节，2026-08-30 用户定案）：档案内资产 size 去重合计（同 get_archive_data 口径）
  coverPath?: string | null; // 封面图绝对路径（6.1 list_archives 输出；无自定义封面 = null 回退子项首图 / 默认封面图）
  coverVer?: string | null; // 封面版本戳（2026-09-08 覆盖式烘培：同路径重写，mtime 戳驱动 ?v= 重载）
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
  coverPath?: string | null; // 封面图绝对路径（6.1）
  coverVer?: string | null; // 封面版本戳（同 list_archives）
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
// CEP 导出接收入库事件监听（8.1）：后端 WS 服务导入完成后广播 cep-imported，全量对齐
let unlistenCep: (() => void) | undefined;

// 视口滚动位置（会话级，2026-07-25 用户定）：进出预览 / 切换目录都恢复——
// 预览返回不跳顶（设缩略图后可直接确认），对比两个文件夹不用重新翻
const scrollPositions = new Map<string, number>();

// 新增区到达序号（3.5）：单调递增计数器，区内先来在前
let arrivalSeq = 0;

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

// 2026-08-19 导出：分组弹窗等按 id 取节点统一走递归（平铺 find 只能命中顶层——
// 子文件夹分组配置读不回 / 新建分组覆盖旧组，复验反馈）
export function findFolder(nodes: FolderNode[], id: string): FolderNode | null {
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
    // 关系清单（6.2 选择模式一轮，空间级 relations.json）：refresh 一并拉取（undo 链同径对齐），
    // 显示侧按当前库过滤（lib/relationOps.relationsForTarget）
    relations: [] as Relation[],
    // 详情数据版本号（6.1）：属性 / 描述 / 封面设置等详情编辑成功后 bump，
    // DetailPanel watch 重拉 get_asset_detail / get_archive_data
    //（selectedAssetKey 不变时 watch 不触发，编辑落盘后的刷新信号）
    detailRev: 0,
    // 3.5 排序新增区（2026-08-19 用户定）：排序状态下新增 / 导入的项不按排序键分散、集中置顶
    // （先来在前）——用户要能找到新建文件、确认导入数量；切视图（sidebarSelected 变化）/ 切库 /
    // 拖拽物化时归位清空。key = `${assetId}|${folderId}`（与选择 / 拖拽键同口径），value = 到达序号
    newArrivals: new Map<string, number>(),
    // 3.5 分组折叠态（会话级）：folderId → 折叠组 id 列表（"__ungrouped" = 未分组组）；
    // 与树展开态同口径（会话级不持久化，lib/treeExpansion 先例）
    groupCollapsed: {} as Record<string, string[]>,
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
    // 详情面板 / 预览等单资产消费方原样不动；空串 = 无选中（语义同原 state 字段）。
    // 序列帧包内视图（5.2，2026-08-21 用户定：详情面板停留序列帧的详情不为空）：
    // 选择域进包时已清空（防全局 Delete 误伤），此处回退 placements 内该包资产（origin 优先；
    // 包已解散 = 空串空面板）——消费方仅 DetailPanel / DetailNotes，两者据此显示包的详情与笔记
    selectedAssetKey(): string {
      const first = useSelectionStore().first;
      if (first) return first;
      if (this.sidebarSelected.startsWith("seq:")) {
        const id = this.sidebarSelected.slice(4);
        const p =
          this.placements.find((x) => x.assetId === id && x.origin) ??
          this.placements.find((x) => x.assetId === id);
        return p ? `${p.assetId}|${p.folderId}` : "";
      }
      return "";
    },
    // 按视口 key 计算资源列表（1.2 多标签页：每个标签页各自视口，原 visiblePlacements 逻辑参数化；
    // "最近使用 / 回收站"已接真实视图，6.1 二轮：元素为 PlacementView 超集，AssetGrid 零改动复用）
    placementsFor(state) {
      return (viewKey: string): PlacementView[] => {
        // 3.5 排序（28 稿）：显示层排序不动存储序——自定义原样返回，其余键全局统一排序；
        // 素材区全视图生效（含快速访问，2026-08-19 用户定）；文件夹/档案卡片不参与。
        // 新增区（2026-08-19 用户定；2026-08-20 放开到自定义排序——用户实机报"全部"视图新导入
        // 沉底：自定义序下存储序按文件夹段组织，root 段靠前/靠后不定）：新增/导入项集中置顶
        // （按到达序），所有排序模式生效——切视图归位
        const sort = (list: PlacementView[]): PlacementView[] => {
          const ui = useUiStore();
          const sorted = sortPlacements(list, ui.sortBy, ui.sortAsc);
          if (state.newArrivals.size === 0) return sorted;
          const fresh: { p: PlacementView; seq: number }[] = [];
          const rest: PlacementView[] = [];
          for (const p of sorted) {
            const seq = state.newArrivals.get(`${p.assetId}|${p.folderId}`);
            if (seq === undefined) rest.push(p);
            else fresh.push({ p, seq });
          }
          if (fresh.length === 0) return sorted;
          fresh.sort((a, b) => a.seq - b.seq);
          return [...fresh.map((f) => f.p), ...rest];
        };
        // hidden 伴生缩略图过滤（8.1，PRD 02-15）：普通视图统一过滤 hidden 资产的全部位置
        // （链接位置一并消失；回收站条目独立数组不受影响）；「缩略图」视图反向取 hidden 集
        const visible = state.placements.filter((p) => !p.hidden);
        // 工作空间磁盘实况（PRD 02 步骤 1）：viewKey "ws:" = 根，"ws:a/b" = relPath——
        // 数据在 ws store 按 rel 缓存，此处合成 PlacementView（assetId = ws: 假 id，禁发 assetId 命令）；
        // 磁盘实况无附加数据，全局排序键仍生效（显示层），新增区不涉
        if (viewKey.startsWith("ws:")) return sort(useWsStore().placementsFor(viewKey.slice(3)));
        if (viewKey.startsWith("folder:")) {
          const id = viewKey.slice(7);
          // 递归显示（ui.showSubfolders，2026-08-10 接入）：开 = 视口含全部后代文件夹的 Placement；
          // 关 = 仅当前文件夹直接 Placement
          if (!useUiStore().showSubfolders) return sort(visible.filter((p) => p.folderId === id));
          const ids = new Set<string>();
          const collect = (n: FolderNode): void => {
            ids.add(n.id);
            n.children.forEach(collect);
          };
          const root = findFolder(state.folders, id);
          if (root) collect(root);
          else ids.add(id); // 失效 viewKey（文件夹已删）：退化为直接匹配
          return sort(visible.filter((p) => ids.has(p.folderId)));
        }
        switch (viewKey) {
          // 2026-08-13 用户定案：快速访问不做内容限制——按实际 Placement 显示，
          // 链接不再隐藏（原 originOnly 资产级去重 + origin 过滤整体废除）
          case "quick:all": // 全部 = 递归显示所有文件（2026-07-22 用户定案）
            return sort(visible);
          case "quick:uncategorized": // 未分类 = 未归入任何文件夹（root 即未分类）
            return sort(visible.filter((p) => p.folderId === "root"));
          case "quick:inbox": // 收件箱"已导入" = importedFrom=inbox 且未摘标的资产，持久显示
            // （2026-07-25 用户定：用户没分类走就要一直在）。不限 folderId=root——手动导入 Keep
            // 生成的层级也算未分类；摘标 = 清 importedFrom（归 6.3），清后自然消失
            return sort(visible.filter((p) => p.importedFrom === "inbox"));
          case "quick:thumbnail": {
            // 缩略图视图（8.1，PRD 02-15 hidden 资产）：按资产显示一次（多条链接位置去重取首条）
            const seen = new Set<string>();
            return sort(
              state.placements.filter((p) => {
                if (!p.hidden || seen.has(p.assetId)) return false;
                seen.add(p.assetId);
                return true;
              }),
            );
          }
          case "quick:recent": // 最近使用（后端已按时间倒序；3.5 排序开启后按排序键覆盖）
            return sort(state.recentAssets.filter((r) => !r.hidden));
          case "quick:recycle": // 回收站（folderId 恒 "recycle"，key 与素材域互不干扰）
            return sort(state.recycleEntries);
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
    // quick 五项 → 固定文案；seq（5.2 包内视图）→ 序列帧包名（资产已删/已解散兜底"序列帧包"）；
    // ws（PRD 02 步骤 1）→ 根 = 「工作空间」，子目录 = 末段目录名（磁盘实况语义，无库内重命名）；
    // 工作空间/标签等占位视图 → "全部"兜底（调用方可另传 fallback）
    viewTitle(state) {
      return (viewKey: string): string => {
        if (viewKey.startsWith("folder:")) {
          return findFolder(state.folders, viewKey.slice(7))?.name ?? "文件夹";
        }
        if (viewKey.startsWith("ws:")) {
          const rel = viewKey.slice(3);
          return rel === "" ? "工作空间" : (rel.split("/").pop() ?? "工作空间");
        }
        if (viewKey.startsWith("seq:")) {
          const id = viewKey.slice(4);
          const p = state.placements.find((x) => x.assetId === id);
          return p ? p.name.replace(/\.[^.]+$/, "") : "序列帧包";
        }
        return QUICK_LABELS[viewKey.slice(6)] ?? "全部";
      };
    },
    // 回收站视口判定（6.1 二轮，2026-07-29）：只读语义统一判断点——禁拖拽 / 禁重命名 / Del 分流彻底删除
    isRecycleView(state): boolean {
      return state.sidebarSelected === "quick:recycle";
    },
    // 工作空间磁盘实况视口判定（PRD 02 步骤 1，2026-09-17）：viewKey = "ws:"+relPath；
    // 只读底座——步骤 2 才落地磁盘写操作，位置级写操作（移动/剪切/副本/删除/重命名/导入）本阶段全禁
    isWorkspaceView(state): boolean {
      return state.sidebarSelected.startsWith("ws:");
    },
    // 禁导入视口判定（2026-09-08 用户定；2026-09-09 入口收窄扩五视口加「全部」——全部不是位置，
    // 落点必借道未分类，与虚拟视图禁位置级同口径）：全部 / 最近使用 / 自动导入 / 缩略图 / 回收站
    // 不允许额外导入 / 新建——右键收窄 / 拖拽拒入弹窗 / 菜单栏导入与新建组禁用 / Ctrl+V 拒入，
    // 多处同一判定；未分类 = 候车区总入口豁免（落点 root 所见即所得）；
    // 工作空间（PRD 02 步骤 1）= 磁盘位置但只读底座，导入语义未立项（库互通另行立项），同档禁
    isNoImportView(state): boolean {
      return (
        (state.sidebarSelected.startsWith("quick:") && state.sidebarSelected !== "quick:uncategorized") ||
        state.sidebarSelected.startsWith("ws:")
      );
    },
    // 虚拟视图判定（2026-09-09 入口收窄，用户定）：全部 / 最近使用 / 缩略图 = 不是位置的视图——
    // 位置级操作（移动 / 剪切 / 副本 / 从文件夹中移除 / 拖出）在此语义不成立（黑箱移动：
    // 用户不知从何移到何），右键收窄 / 快捷键 / 拖拽源同一判定；回收站只读走 isRecycleView
    // 专属口径不在此列；未分类 / 自动导入 = 位置视口（真实落点）也不在此列；
    // 工作空间（PRD 02 步骤 1）= 真实磁盘位置但只读底座——位置级写操作本阶段全禁，同档挂入
    // （磁盘写语义与库不通用，步骤 2 另行定案后摘除）
    isVirtualView(state): boolean {
      return (
        ["quick:all", "quick:recent", "quick:thumbnail"].includes(state.sidebarSelected) ||
        state.sidebarSelected.startsWith("ws:")
      );
    },
    // 快速访问视口判定（2026-09-09 入口收窄）：六视口无文件夹概念——新建文件夹 / 新建档案整组禁用
    isQuickView(state): boolean {
      return state.sidebarSelected.startsWith("quick:");
    },
    // 快速访问 item 数量（4.4 查缺补漏）：全部 / 未分类 / 收件箱接真实统计；
    // 最近使用 / 回收站留 6.1，工作空间 / 标签 / 快捷入口占位视图不接（对应阶段未开工）；
    // 2026-08-13 起与视图同口径：按实际 Placement 计数（链接不再隐藏）
    allCount(state): number {
      return state.placements.filter((p) => !p.hidden).length;
    },
    uncategorizedCount(state): number {
      return state.placements.filter((p) => !p.hidden && p.folderId === "root").length;
    },
    inboxCount(state): number {
      return state.placements.filter((p) => !p.hidden && p.importedFrom === "inbox").length;
    },
    // 缩略图视图计数（8.1，PRD 02-15）：hidden 资产按资产去重（与 quick:thumbnail 视图同口径）
    thumbnailCount(state): number {
      const seen = new Set<string>();
      for (const p of state.placements) {
        if (p.hidden) seen.add(p.assetId);
      }
      return seen.size;
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
    // 视口分组规则（3.5 二轮 / PRD 02-04）：仅文件夹/档案视图（folder: 前缀）有配置位；
    // 快速访问恒 null（无分组，2026-08-19 用户定）；递归显示开启时恒 null（递归不分组，
    // 2026-08-19 复验定案：配置仅对直接子项生效——递归下为后代素材建的组在非递归视图成幽灵配置）
    folderGroupsFor(state) {
      return (viewKey: string): import("../lib/grouping").GroupDef[] | null => {
        if (!viewKey.startsWith("folder:")) return null;
        if (useUiStore().showSubfolders) return null;
        const g = findFolder(state.folders, viewKey.slice(7))?.groups;
        return g && g.length > 0 ? g : null;
      };
    },
    // 激活视口的分组规则（分组渲染 / 弹窗上下文用）
    activeFolderGroups(state): import("../lib/grouping").GroupDef[] | null {
      return this.folderGroupsFor(state.sidebarSelected);
    },
    // 激活视口的子级档案（操作/显示上下文跟随激活标签页）
    stripArchives(state): FolderNode[] {
      return this.stripArchivesFor(state.sidebarSelected);
    },
    // 激活视口的子文件夹（操作/显示上下文跟随激活标签页）
    stripFolders(state): FolderNode[] {
      return this.stripFoldersFor(state.sidebarSelected);
    },
    // 资产活位置计数（6.2 四轮走查③徽章规则：origin ★ / 链接 icon 只在"有链接"（活位置 > 1）时上，
    // 单位置文件无徽章；AssetCard 徽章与徽章悬停链接列表共用）
    placementCountByAsset(state): Map<string, number> {
      const map = new Map<string, number>();
      for (const p of state.placements) map.set(p.assetId, (map.get(p.assetId) ?? 0) + 1);
      return map;
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
    // 各文件夹直接 Placement 数（2026-08-20 树数量口径）：展开态的树节点显示直接子项文件数——
    // 递归总数（FolderNode.count）含全部后代，展开后子文件夹已各自成行，父级数量只算自己的文件
    directCounts(state): Map<string, number> {
      const m = new Map<string, number>();
      for (const p of state.placements) m.set(p.folderId, (m.get(p.folderId) ?? 0) + 1);
      return m;
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

    // 3.5 新增区登记（排序状态下置顶）：导入 / 粘贴 / 新建 / 副本完成后调用；重复登记原位不动
    markArrivals(keys: string[]): void {
      for (const k of keys) {
        if (!this.newArrivals.has(k)) this.newArrivals.set(k, ++arrivalSeq);
      }
    },
    // 新增区归位清空：切视图（sidebarSelected 变化，App.vue watch）/ 切库 / 拖拽物化时调用
    clearArrivals(): void {
      if (this.newArrivals.size > 0) this.newArrivals.clear();
    },

    // 3.5 分组折叠切换（会话级）：groupId null = 未分组组（存 "__ungrouped"）
    toggleGroupCollapsed(folderId: string, groupId: string | null): void {
      const key = groupId ?? "__ungrouped";
      const list = this.groupCollapsed[folderId] ?? [];
      const i = list.indexOf(key);
      if (i >= 0) list.splice(i, 1);
      else list.push(key);
      this.groupCollapsed[folderId] = list;
    },
    // 分组折叠判定（模板直读）
    isGroupCollapsed(folderId: string, groupId: string | null): boolean {
      return (this.groupCollapsed[folderId] ?? []).includes(groupId ?? "__ungrouped");
    },
    // 12 稿展开/折叠二级批量操作：全展开 = 清空折叠集；排除当前展开全部 = 折叠集只留当前组
    expandAllGroups(folderId: string): void {
      if (this.groupCollapsed[folderId]?.length) this.groupCollapsed[folderId] = [];
    },
    expandGroupsExcept(folderId: string, keepId: string | null, allGroupIds: (string | null)[]): void {
      this.groupCollapsed[folderId] = allGroupIds.filter((id) => id !== keepId).map((id) => id ?? "__ungrouped");
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
      await Promise.all([this.refreshRecent(), this.refreshRecycle(), this.refreshArchives(), this.refreshRelations()]);
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
    // 详情编辑落盘信号（6.1）：属性 / 描述 / 封面设置等写命令成功后 bump，
    // DetailPanel watch detailRev 重拉当前详情（选中未变时 watch 不触发）
    bumpDetailRev(): void {
      this.detailRev++;
    },
    // 关系清单（6.2 选择模式一轮）：空间级 relations.json 全量拉取；同 refreshArchives 静默降级口径
    async refreshRelations(): Promise<void> {
      try {
        this.relations = await invoke<Relation[]>("list_relations");
      } catch (e) {
        console.warn("关系清单刷新失败（静默降级）：", e);
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

    // CEP 导出入库热刷新（8.1）：WS 服务接收入库完成广播 cep-imported → 全量对齐
    // （新 fla 卡片显示伴生图 + 缩略图视图入列 + 目标文件夹新增，涉及视图代理与 hidden
    // 过滤多口径，局部同步成本高且易漏，导入级操作频率低，直接 refresh 同导入完成口径）
    // + 成功提示音（2026-09-02 用户定：导出时视口大概率停留在 Animate 窗口，听觉反馈兜底；
    // 批量导出逐条 emit → 500ms 节流合并，一次批量只响一声；失败音待 09——面板侧回执用户可见）
    async initCepListener(): Promise<void> {
      if (unlistenCep) return;
      let lastSound = 0;
      unlistenCep = await listen("cep-imported", () => {
        const now = Date.now();
        if (now - lastSound > 500) {
          lastSound = now;
          playSuccess();
        }
        void this.refresh();
      });
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
                // 3.5 新增区：导入项登记置顶（排序状态下集中顶部，便于确认导入数量）
                this.markArrivals(batch.map((p) => `${p.assetId}|${p.folderId}`));
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
