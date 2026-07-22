import { markRaw } from "vue";
import { defineStore } from "pinia";
import { Channel, invoke } from "@tauri-apps/api/core";

// 与后端 commands/import.rs 输出一一对应（camelCase，07_接口文档）
export interface PlacementView {
  assetId: string;
  folderId: string;
  name: string;
  w: number | null; // fla 等无尺寸为 null，布局按 400×300 默认比例
  h: number | null;
  ext: string;
  thumbPath: string | null; // 缩略图绝对路径，无则卡片显示占位底色
  origin: boolean;
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
  sourcePath: string; // 源文件绝对路径
  sourceName: string; // 源文件名（含扩展名）
  sourceSize: number;
  sourceW: number | null; // 图片宽高（非图片 null）
  sourceH: number | null;
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
}

// 冲突决策（后端 commands resolve_conflicts 入参）
export interface ConflictResolution {
  sourcePath: string;
  existingAssetId: string;
  targetFolder: string;
  action: "useExisting" | "copy" | "link" | "skip";
  linkName?: string | null;
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
    // 原导入的来源模式（冲突"保留两个"决策继承；剪贴板导入恒 move）
    lastImportSource: "copy" | "move",
  }),
  getters: {
    // 当前视口的资源列表（未分类 / 收件箱 / 最近 / 回收站阶段 4 接，本轮给空）
    visiblePlacements(state): PlacementView[] {
      if (state.sidebarSelected.startsWith("folder:")) {
        const id = state.sidebarSelected.slice(7);
        return state.placements.filter((p) => p.folderId === id);
      }
      return state.sidebarSelected === "quick:all" ? state.placements : [];
    },
    // 文件夹卡片横排（03_中间面板）：当前视口的直接子文件夹；"全部"视图 = 根一级
    stripFolders(state): FolderNode[] {
      if (state.sidebarSelected.startsWith("folder:")) {
        return findFolder(state.folders, state.sidebarSelected.slice(7))?.children ?? [];
      }
      return state.folders;
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

    // 后台导入任务：命令立即返回 task_id，进度经 Channel 推送（异步不阻塞 UI）
    async startImport(
      draft: ImportDraft,
      structure: "flatten" | "keep",
      source: "copy" | "move",
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
