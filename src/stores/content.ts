import { markRaw } from "vue";
import { defineStore } from "pinia";
import { genAssets, genFolderTree, type FakeAsset, type FakeFolder } from "../lib/fakeData";

// 10 万条假数据：markRaw 避免深响应式代理（规范第 7 条性能纪律）
export const useContentStore = defineStore("content", {
  state: () => ({
    assets: markRaw(genAssets(100_000)),
    folders: markRaw(genFolderTree()) as FakeFolder[],
    // 左栏选中项（阶段 1 仅样式展示，不联动内容区）：quick:all / folder:{id}
    sidebarSelected: "quick:all",
    selectedAssetId: -1,
  }),
  getters: {
    // 目录卡片横排用的假文件夹（取树一级节点）
    stripFolders(state): FakeFolder[] {
      return state.folders;
    },
  },
});

export type { FakeAsset };
