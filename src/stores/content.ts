import { markRaw } from "vue";
import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";

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
    // 10 万级数据：markRaw 避免深响应式代理（规范第 7 条性能纪律）
    placements: markRaw([]) as PlacementView[],
    folders: markRaw([]) as FolderNode[],
    // 左栏选中项：quick:all / quick:uncategorized… / folder:{id}
    sidebarSelected: "quick:all",
    // 选中互斥：选中素材卡片时取消文件夹卡片选中，反之亦然
    selectedAssetKey: "", // `${assetId}|${folderId}`，空串 = 无选中
    selectedFolderId: "",
    // 导入弹窗草稿：菜单 / 拖拽触发后填入，ImportDialog 读取；确认或取消后清空
    importDraft: null as {
      kind: "files" | "folder" | "mixed";
      paths: string[];
      targetFolder: string | null; // null = 根目录
    } | null,
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
      this.placements = markRaw(placements);
      this.folders = markRaw(folders);
    },
  },
});
