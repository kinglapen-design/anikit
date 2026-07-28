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
    // 查看方式二级窗口（2026-07-23）：显示简介（尺寸已实现，其余随 7.x 接线）/ 角标开关；
    // 自动播放（5.3 已接，2026-07-27）：动图开关含序列帧（用户定：序列帧也是动图范围）
    briefMode: "size" as "size" | "fileSize" | "createdAt" | "modifiedAt" | "tags" | "none",
    groupBy: "自定义",
    autoPlayGif: false,
    autoPlayVideo: false,
    showExtBadge: true,
    // 时间显示（5.3 帧化改造，2026-07-28 用户定：帧数优先）——视频播放条 / 悬停胶囊 / 笔记
    // 按帧号显示（fps 缺失回退时间）；音频无帧概念恒时间显示
    timeDisplay: "frames" as "frames" | "timecode",
  }),
});
