import { markRaw } from "vue";
import { defineStore } from "pinia";
import { Channel, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

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
  frameCount: number | null; // 序列帧帧数；非序列帧为 null——据此把格式标签与名称后缀显示为"序列帧"
}

export interface FolderNode {
  id: string;
  name: string;
  count: number; // 该文件夹直接 Placement 数
  children: FolderNode[];
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
  linkName?: string | null;
  sequence?: SequenceSource | null; // 序列帧冲突原样回传（4.1）
}

// 导入任务事件（后端 services/import.rs ImportEvent，serde tag="type" camelCase）
export type ImportEvent =
  | { type: "started"; total: number }
  | { type: "folders" } // keep 模式层级已建，重拉文件树
  | { type: "asset"; placement: PlacementView } // 到一个显示一个
  | { type: "finished"; imported: number; conflicts: ConflictItem[]; failed: FailedItem[] };

// 流式追加节流：asset 事件先进缓冲，每 250ms 批量入列——
// 逐条 push 会让 10 万级列表每文件重排一次（布局 ~10ms×N 必卡），批量后重排 ≤4 次/秒
const FLUSH_MS = 250;
let assetBuffer: PlacementView[] = [];
let flushTimer: ReturnType<typeof setTimeout> | undefined;

// 收件箱事件监听（3.2）：App 存活期注册一次，后端监听 inbox 目录自动入库
let unlistenInbox: (() => void) | undefined;

// 递归查文件夹节点（stripFolders / 标题栏 crumbs 共用）
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
    // 左栏选中项：quick:all / quick:uncategorized… / folder:{id}
    sidebarSelected: "quick:all",
    // 选中互斥：选中素材卡片时取消文件夹卡片选中，反之亦然
    selectedAssetKey: "", // `${assetId}|${folderId}`，空串 = 无选中
    selectedFolderId: "",
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
    // 失败清单（ImportResultDialog）与冲突挂起队列（DuplicateDialog 逐个询问）
    importFailed: null as FailedItem[] | null,
    importConflicts: null as ConflictItem[] | null,
    // 收件箱视图（quick:inbox）：本会话收件箱收进来的文件，新的在前；
    // 会话级，重启清空（持久化与"最近使用"体系随对应阶段再定）
    inboxReceived: [] as PlacementView[],
    // 原导入的来源模式（冲突"保留两个"决策继承；剪贴板导入恒 move）
    lastImportSource: "copy" as "copy" | "move",
  }),
  getters: {
    // 当前视口的资源列表（最近使用 / 回收站对应阶段再接，本轮给空）
    visiblePlacements(state): PlacementView[] {
      if (state.sidebarSelected.startsWith("folder:")) {
        const id = state.sidebarSelected.slice(7);
        return state.placements.filter((p) => p.folderId === id);
      }
      switch (state.sidebarSelected) {
        case "quick:all": // 全部 = 递归显示所有文件（2026-07-22 用户定案）
          return state.placements;
        case "quick:uncategorized": // 未分类 = 未归入任何文件夹（root 即未分类；收件箱新件也在此列）
          return state.placements.filter((p) => p.folderId === "root");
        case "quick:inbox": // 收件箱 = 本会话收件箱收进来的文件（新的在前）
          return state.inboxReceived;
        default:
          return [];
      }
    },
    // 文件夹卡片横排（03_中间面板）：仅文件夹视图显示直接子文件夹；
    // 快速访问视图一律不出文件夹区块（2026-07-22 用户定案；回收站保留文件夹随对应阶段另议）
    stripFolders(state): FolderNode[] {
      if (state.sidebarSelected.startsWith("folder:")) {
        return findFolder(state.folders, state.sidebarSelected.slice(7))?.children ?? [];
      }
      return [];
    },
    // 文件夹区块标题计数（全树节点总数）
    folderTotal(state): number {
      const walk = (nodes: FolderNode[]): number =>
        nodes.reduce((sum, n) => sum + 1 + walk(n.children), 0);
      return walk(state.folders);
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
    // 库打开后 / 导入完成后全量刷新（增量刷新随阶段 4 任务队列再做）
    async refresh(): Promise<void> {
      const [placements, folders] = await Promise.all([
        invoke<PlacementView[]>("list_placements", { folder: null }),
        invoke<FolderNode[]>("list_folder_tree"),
      ]);
      // 数组保持可追踪（流式 push 触发）；元素由 Vue 惰性代理，10 万级只代理可视区
      this.placements = placements;
      this.folders = markRaw(folders);
    },
    async refreshFolders(): Promise<void> {
      this.folders = markRaw(await invoke<FolderNode[]>("list_folder_tree"));
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
      this.lastImportSource = "move"; // 剪贴板临时 PNG 以 move 走管线
      this.importProgress = { taskId: null, total: 0, done: 0, startedAt: Date.now() };
      const onEvent = new Channel<ImportEvent>();
      onEvent.onmessage = (e) => this.onImportEvent(e);
      const taskId = await invoke<number>("import_clipboard", { targetFolder, onEvent });
      if (this.importProgress && this.importProgress.taskId === null) {
        this.importProgress.taskId = taskId;
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

    // 冲突决策提交（10_重复添加提示）：逐项或"全部应用"批量，来源模式继承原导入
    async resolveConflicts(
      items: ConflictResolution[],
    ): Promise<{ resolved: number; failed: FailedItem[] }> {
      return await invoke("resolve_conflicts", { items, source: this.lastImportSource });
    },

    // 收件箱后台导入（PRD 05-01，3.2）：后端监听 inbox 目录自动入库，
    // 事件形状与导入 Channel 一致，复用 11_导入进度条 / 10_重复添加提示
    async initInboxListener(): Promise<void> {
      if (unlistenInbox) return;
      unlistenInbox = await listen<ImportEvent>("inbox-import", (e) => {
        if (e.payload.type === "started" && !this.importProgress) {
          this.lastImportSource = "move"; // 收件箱恒 Move（过境：导入后即移走）
          this.importProgress = { taskId: null, total: 0, done: 0, startedAt: Date.now() };
        }
        // 收件箱视图收集：新的在前，同键（assetId|folderId）去重
        if (e.payload.type === "asset") {
          const p = e.payload.placement;
          const key = `${p.assetId}|${p.folderId}`;
          this.inboxReceived = [
            markRaw(p),
            ...this.inboxReceived.filter((x) => `${x.assetId}|${x.folderId}` !== key),
          ];
        }
        this.onImportEvent(e.payload);
      });
    },

    onImportEvent(e: ImportEvent): void {
      switch (e.type) {
        case "started":
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
              if (assetBuffer.length > 0) this.placements.push(...assetBuffer.splice(0));
            }, FLUSH_MS);
          }
          break;
        case "finished":
          clearTimeout(flushTimer);
          flushTimer = undefined;
          assetBuffer = [];
          this.importProgress = null;
          if (e.failed.length > 0) this.importFailed = e.failed;
          // 冲突挂起队列：导入全部完成后逐个询问（10_重复添加提示）
          if (e.conflicts.length > 0) this.importConflicts = e.conflicts;
          void this.refresh(); // 兜底全量对齐（链接 / 层级以最终态为准）
          break;
      }
    },
  },
});
