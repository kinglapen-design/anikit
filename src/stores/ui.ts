import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { mergeDetailSections, type DetailSectionPref } from "../lib/detailSections";

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
  "showBadge",
  "showExt",
  "showBrief",
  "showSubfolders",
  "timeDisplay",
  "tabLayout",
  "detailSections",
] as const;

// 变更防抖写回（500ms；模块级单例防重复订阅）
let persistTimer: ReturnType<typeof setTimeout> | undefined;
let subscribed = false;
// 加载幂等（2026-08-08：tabs 布局恢复需 await 设置就绪——DockHost onReady 可能早于
// App.vue onMounted 的 initSettings 完成，同一 promise 保证只加载一次且可等待）
let settingsPromise: Promise<void> | null = null;

export const useUiStore = defineStore("ui", {
  state: () => ({
    leftView: "library" as LeftView,
    rightView: "detail" as RightView,
    leftWidth: 304, // 1.2 新稿：54 导航栏 + 250 内容区
    rightWidth: 250,
    leftCollapsed: false,
    rightCollapsed: false,
    alwaysOnTop: false,
    // 内容区滑块：自适应布局目标行高（min 80，max = gridWidth 即单张撑满）
    targetRowHeight: 220,
    gridWidth: 800,
    // 递归显示（PRD 04-1 显示子文件夹内容）：文件夹视口含全部后代文件夹内容（2026-08-10 接入
    // placementsFor；文件夹区块标题"递归显示"复选框与查看方式菜单开关同绑本字段）
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
    // 28 查看方式新稿（1.4，2026-08-11）：显示徽标（格式角标显隐）/ 显示扩展名（名称带不带
    // 扩展名）/ 显示简介开关（与 briefMode 内容选择独立）——旧 showExtBadge（徽标语义）废除
    showBadge: true,
    showExt: true,
    showBrief: true,
    // 时间显示（5.3 帧化改造，2026-07-28 用户定：帧数优先）——视频播放条 / 悬停胶囊 / 笔记
    // 按帧号显示（fps 缺失回退时间）；音频无帧概念恒时间显示
    timeDisplay: "frames" as "frames" | "timecode",
    // 文件树排序（6.3 右键菜单，会话级）：custom = folders.json order 原序；
    // levelAsc/levelDesc = 仅右键目标层级按标题（treeSortAnchor 记目标父级，null = 顶层）；
    // allAsc/allDesc = 全部层级按标题
    treeSort: "custom" as "custom" | "levelAsc" | "levelDesc" | "allAsc" | "allDesc",
    treeSortAnchor: null as string | null,
    // 侧栏收展动画进行中标志（会话级，不持久化）：AssetGrid 在此期间逐帧实时重排（reflow，
    // 2026-08-08 收展定案 v3）
    sidebarAnimating: false,
    // 区块管理弹窗开关（2026-08-14 区块子系统，会话级）：SidebarLeft section-manage 按钮置位，
    // 弹窗宿主 App.vue（同 FailedListDialog 惯例）
    blockManageOpen: false,
    // 多标签页布局（1.2）：dockview toJSON 快照，tabs store 防抖写入 / 启动 fromJSON 恢复；
    // null = 无布局（首启默认单标签页）。typeof null === "object"，PERSIST_KEYS 类型校验放行对象
    tabLayout: null as object | null,
    // 详情面板 8 区块排序 / 显隐（2026-08-15 区块体系一轮，22 稿「编辑数据区块」面板）：
    // 默认 = 注册表全 visible；加载后经注册表兜底合并（新增补位 / 已删忽略）。
    // 数组 typeof "object"，PERSIST_KEYS 类型校验放行（同 tabLayout 先例），形状由合并函数校验
    detailSections: mergeDetailSections(null) as DetailSectionPref[],
    // 22 稿「编辑数据区块」面板开关（会话级，同 blockManageOpen 口径）：右栏工具栏与档案侧栏
    // 工具行两处入口（2026-08-15 走查修正：档案面板铅笔开同一个面板），host 记打开来源
    // （两处各自带 anchor/trigger 渲染，同一时刻只一处弹出）
    sectionEditOpen: false,
    sectionEditHost: "right" as "right" | "archive",
    // 档案二级侧边栏开关（2026-08-15 迁入标签页视口）：per-folder 会话级 map
    //（folderId → 是否展开，默认 true）——两个档案标签页各自独立互不干扰
    archivePanelOpen: {} as Record<string, boolean>,
  }),
  actions: {
    // 档案二级侧边栏开关读写（per-folder，缺省 true = 进档案默认展开）
    archivePanelOpenFor(folderId: string): boolean {
      return this.archivePanelOpen[folderId] ?? true;
    },
    toggleArchivePanel(folderId: string): void {
      this.archivePanelOpen[folderId] = !this.archivePanelOpenFor(folderId);
    },
    // 详情区块排序（22 稿面板箭头）：相邻交换，越界不动
    moveDetailSection(id: string, dir: 1 | -1): void {
      const list = this.detailSections;
      const i = list.findIndex((s) => s.id === id);
      const j = i + dir;
      if (i < 0 || j < 0 || j >= list.length) return;
      const [row] = list.splice(i, 1);
      list.splice(j, 0, row);
    },
    // 详情区块拖拽落点排序（22 稿面板让位式拖拽）：直接移到目标位（与逐次 moveDetailSection 等效）
    moveDetailSectionTo(id: string, to: number): void {
      const list = this.detailSections;
      const from = list.findIndex((s) => s.id === id);
      if (from < 0 || to < 0 || to >= list.length || to === from) return;
      const [row] = list.splice(from, 1);
      list.splice(to, 0, row);
    },
    // 详情区块显隐（22 稿面板眼睛）
    toggleDetailSection(id: string): void {
      const s = this.detailSections.find((s) => s.id === id);
      if (s) s.visible = !s.visible;
    },
    // 启动加载全局 UI 设置并订阅变更防抖写回（多预览窗口各自独立 Pinia 各自加载，
    // 不做跨窗口同步，2026-07-28 定）；读取失败 / 字段类型不符一律按默认值，不阻断启动。
    // 幂等可等待：重复调用返回同一 promise（tabs 布局恢复等依赖方 await 同一份就绪）
    initSettings(): Promise<void> {
      settingsPromise ??= this.loadSettings();
      return settingsPromise;
    },
    async loadSettings(): Promise<void> {
      try {
        const saved = await invoke<Record<string, unknown>>("get_ui_settings");
        const state = this.$state as unknown as Record<string, unknown>;
        for (const k of PERSIST_KEYS) {
          const v = saved[k];
          // 类型对照默认值校验：config 手改 / 旧版本字段变形不污染 store
          if (v !== undefined && typeof v === typeof state[k]) state[k] = v;
        }
        // 区块配置兜底合并：注册表新增补位 / 配置里已删 id 忽略（lib/detailSections）
        this.detailSections = mergeDetailSections(this.detailSections);
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
