import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";

export type LeftView = "library" | "workspace" | "tags" | "pins";
export type RightView = "detail" | "aiChat" | "aiPaint" | "aiVideo";

// 持久化字段清单（2026-07-28 用户定："用户设置要持久化"）：存全局 config.json
// settings.ui 子树（core/config.rs 单一写点，应用级不随库）
const PERSIST_KEYS = [
  "leftView",
  "rightView",
  "leftWidth",
  "rightWidth",
  "leftCollapsed",
  "rightCollapsed",
  "alwaysOnTop",
  "targetRowHeight",
  "layoutMode",
  "useGlobalView",
  "sortBy",
  "sortAsc",
  "briefMode",
  "groupBy",
  "autoPlayGif",
  "autoPlayVideo",
  "showExtBadge",
  "showSubfolders",
  "timeDisplay",
] as const;

// 变更防抖写回（500ms；模块级单例防重复订阅）
let persistTimer: ReturnType<typeof setTimeout> | undefined;
let subscribed = false;

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
  actions: {
    // 启动加载全局 UI 设置并订阅变更防抖写回（多预览窗口各自独立 Pinia 各自加载，
    // 不做跨窗口同步，2026-07-28 定）；读取失败 / 字段类型不符一律按默认值，不阻断启动
    async initSettings(): Promise<void> {
      try {
        const saved = await invoke<Record<string, unknown>>("get_ui_settings");
        const state = this.$state as unknown as Record<string, unknown>;
        for (const k of PERSIST_KEYS) {
          const v = saved[k];
          // 类型对照默认值校验：config 手改 / 旧版本字段变形不污染 store
          if (v !== undefined && typeof v === typeof state[k]) state[k] = v;
        }
      } catch (e) {
        console.warn("UI 设置读取失败（用默认值）：", e);
      }
      if (subscribed) return;
      subscribed = true;
      this.$subscribe(() => {
        clearTimeout(persistTimer);
        persistTimer = setTimeout(() => {
          const state = this.$state as unknown as Record<string, unknown>;
          const snapshot: Record<string, unknown> = {};
          for (const k of PERSIST_KEYS) snapshot[k] = state[k];
          invoke("save_ui_settings", { settings: snapshot }).catch((e) =>
            console.warn("UI 设置写回失败（静默降级）：", e),
          );
        }, 500);
      });
    },
  },
});
