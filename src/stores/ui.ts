import { defineStore } from "pinia";

export type LeftView = "library" | "workspace" | "tags" | "pins";
export type RightView = "detail" | "aiChat" | "aiPaint" | "aiVideo";

export const useUiStore = defineStore("ui", {
  state: () => ({
    leftView: "library" as LeftView,
    rightView: "detail" as RightView,
    leftWidth: 260,
    rightWidth: 260,
    leftCollapsed: false,
    rightCollapsed: false,
    alwaysOnTop: false,
    // 内容区滑块：自适应布局目标行高（min 80，max = gridWidth 即单张撑满）
    targetRowHeight: 220,
    gridWidth: 800,
    showSubfolders: true,
    // 查看方式（阶段 1：布局生效，排序 / 分组占位）
    layoutMode: "justified" as "justified" | "waterfall" | "grid" | "list",
    useGlobalView: true,
    sortBy: "",
    sortAsc: true,
  }),
});
