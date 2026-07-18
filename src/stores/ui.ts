import { defineStore } from "pinia";

export type LeftView = "library" | "workspace" | "tags" | "pins";
export type RightView = "detail" | "aiChat" | "aiPaint" | "aiVideo";

export const useUiStore = defineStore("ui", {
  state: () => ({
    leftView: "library" as LeftView,
    rightView: "detail" as RightView,
    leftWidth: 250,
    rightWidth: 250,
    leftCollapsed: false,
    rightCollapsed: false,
    alwaysOnTop: false,
    // 内容区滑块：自适应布局目标行高
    targetRowHeight: 220,
    showSubfolders: true,
  }),
});
