import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { mergeDetailLinkOrder } from "../lib/detailSections";
import type { SortKey } from "../lib/sortPlacements";

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
  "showLinkBadge",
  "showSearchBar",
  "showSubfolders",
  "timeDisplay",
  "tabLayout",
  "detailLinksOrder",
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
    leftWidth: 254, // 7.1 十八轮左栏轮：左侧边栏稿 254（旧 304 = 54 导航栏 + 250 内容区作废）
    rightWidth: 264, // 7.1 二十三轮右栏轮：右侧边栏稿 264（内容宽 240 + 左右 padding 12×2）
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
    // 3.5 排序（28 稿）："" = 自定义（存储序）；其余键 = 显示层排序（lib/sortPlacements）
    sortBy: "" as SortKey,
    sortAsc: true,
    // 查看方式二级窗口（2026-07-23）：显示简介（尺寸已实现，其余随 7.x 接线）/ 角标开关；
    // 自动播放（5.3 已接，2026-07-27）：动图开关含序列帧（用户定：序列帧也是动图范围）；
    // 2026-08-19 显示简介接入：「无显示」档废除（与 showBrief 开关语义重复，用户定），
    // 旧配置残留 "none" 由 loadSettings 迁移为 "size"
    briefMode: "size" as "size" | "fileSize" | "createdAt" | "modifiedAt" | "tags",
    groupBy: "自定义",
    autoPlayGif: false,
    autoPlayVideo: false,
    // 28 查看方式新稿（1.4，2026-08-11）：显示徽标（格式角标显隐）/ 显示扩展名（名称带不带
    // 扩展名）/ 显示简介开关（与 briefMode 内容选择独立）——旧 showExtBadge（徽标语义）废除
    showBadge: true,
    showExt: true,
    showBrief: true,
    // 视图菜单（2026-09-06 菜单栏「视图」接业务轮）：
    // showLinkBadge = 显示链接标识（卡片/列表行的位置徽章 origin ★ / 链接 icon 显隐总开关）；
    // showSearchBar = 始终显示搜索框（关 = 悬浮栏搜索平时隐藏，Ctrl+F 唤出 / Esc 清空失焦后收回，
    // 关闭动作带 toast 告知 Ctrl+F 通道，2026-09-06 用户定）
    showLinkBadge: true,
    showSearchBar: true,
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
    // 分组弹窗（3.5 二轮，会话级）：groupManageOpen = 33 稿分组管理 / groupEditOpen = 34 稿新建分组；
    // groupDialogFolder = 打开时锁定的目标文件夹（打开后切视口不漂移）；工具栏分组管理钮 /
    // 12 稿分组菜单「新建分组 / 编辑分组」置位，宿主 App.vue
    groupManageOpen: false,
    groupEditOpen: false,
    groupDialogFolder: null as string | null,
    // 多标签页布局（1.2）：dockview toJSON 快照，tabs store 防抖写入 / 启动 fromJSON 恢复；
    // null = 无布局（首启默认单标签页）。typeof null === "object"，PERSIST_KEYS 类型校验放行对象
    tabLayout: null as object | null,
    // 链接关系 tab 区块顺序（7.1 二十三轮 tab 化，稿序 = 链接/音频/关系/缩略图/污染文件）：
    // 仅链接 tab 五区块可排序（面板内拖标题行，2026-08-31 用户定）；显隐体系废除
    //（后续「信息查看管理」面板轮再接）；旧 detailSections / detailSectionsArchive 双套配置作废不迁移。
    // 数组 typeof "object"，PERSIST_KEYS 类型校验放行（同 tabLayout 先例），形状由合并函数校验。
    detailLinksOrder: mergeDetailLinkOrder(null),
    // 自定义数据编辑弹窗（6.1，会话级）：propsEdit = 32 稿属性标签 / descEdit = 31 稿编辑描述；
    // target = 打开时锁定的对象与当前值快照（弹窗草稿以此为初值；打开后切选中不漂移，
    // 同 groupDialogFolder 口径），宿主 App.vue；SectionCustom「添加属性 / 编辑描述」钮置位
    propsEdit: null as { kind: "file" | "archive"; id: string; name: string; initial: string[] } | null,
    descEdit: null as { kind: "file" | "archive"; id: string; name: string; initial: { title: string; content: string }[] } | null,
  }),
  actions: {
    // 链接关系 tab 区块拖拽落点排序（面板内拖标题行让位式，DetailSections 旧范式移植）：
    // 直接移到目标位；越界 / 原位不动
    moveDetailLinkTo(id: string, to: number): void {
      const list = this.detailLinksOrder;
      const from = list.indexOf(id);
      if (from < 0 || to < 0 || to >= list.length || to === from) return;
      const [row] = list.splice(from, 1);
      list.splice(to, 0, row);
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
        // 链接 tab 区块序兜底合并：注册表新增补位 / 配置里已删 id 忽略（lib/detailSections）
        this.detailLinksOrder = mergeDetailLinkOrder(this.detailLinksOrder);
        // 「无显示」档废除迁移（2026-08-19）：旧配置 briefMode "none" 回退尺寸档（赋值循环只校验
        // typeof 不校验枚举值，运行态可能已被 "none" 覆盖，此处纠正）
        if (saved.briefMode === "none") this.briefMode = "size";
        // 左栏宽默认值迁移（2026-08-30，7.1 十八轮左栏轮）：旧默认 304 → 新稿 254；
        // 仅改恰好等于旧默认的值（用户拖过的不动），同 briefMode 迁移先例
        if (saved.leftWidth === 304) this.leftWidth = 254;
        // 左栏最小宽 246（同日用户实机定：再窄菜单栏四文字钮被裁）；存量更窄的值夹到 246
        if (typeof saved.leftWidth === "number" && saved.leftWidth < 246) this.leftWidth = 246;
        // 右栏宽默认值迁移（2026-08-31，7.1 二十三轮右栏轮）：旧默认 250 → 新稿 264；同 leftWidth 先例
        if (saved.rightWidth === 250) this.rightWidth = 264;
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
