// 分词搜索（3.6 一轮，PRD 03_查找文件 01）：搜索态 per-panel 独立（搜索框在每个 dockview panel 内）。
// 匹配在后端（search_assets 命令，规范第 1 条业务层）；本 store 持有命中键集合，视图层过滤展示。
// 口径（2026-08-19 用户定）：结果替换素材区（清空恢复）/ 排序生效分组平铺 / 当前目录跟随递归开关 /
// 快速访问 = 当前视图即范围（回收站里搜回收站）。
// 输入形态（2026-08-31 用户定案，7.1 二十二轮悬浮栏接入）：多功能悬浮栏单行输入实时分词——
// tag 化 chips（`~` / `//` / `/` 前缀 tag、空格成 chip、kbd 提示组）废除，筛选与搜索分离；
// `~` 手敲全局保留（后端前缀协议不变）。
// 筛选（2026-10-09 筛选业务二轮点亮）：悬浮栏筛选按钮组 + 弹窗，条件 per-panel 持有随
// scope.filters 下发；口径 = 维度内 OR、跨维度 AND、×搜索 AND；纯筛选同进搜索态（PRD 03 §2）。
//
// 工作空间搜索（2026-10-03，PRD 02「搜索」双层架构）：
// - ws 视口分流到 ws_search 命令（Everything 手写 IPC）；`~` = 全景搜索工作空间（scope 换算根），
//   无 `~` = 当前目录子树（Everything path: 天然递归，Explorer 同语义）
// - 无 Everything：输入不进即时搜索，Enter 显式触发爬盘回退（ws_search_crawl_start，流式批次 +
//   进度 + 可取消）；**回退不接 `~`**（全局慢搜无意义，用户定）——`~` 输入即通知栏指路
//   下载 / 启动 Everything（每 panel 每段 `~` 会话只弹一次，防击键刷屏）
// - 命中条目 = WsEntry 集（wsEntries），hits 同步置合成键（`ws:{rel}|ws:{父rel}`）供标题计数等
//   既有口径消费；缩略图经既有 ws 缩略图管线（卡片可见上报 + thumbs-ready 回填）补
import { ref } from "vue";
import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { formatSize } from "../lib/formatSize";
import { useUiStore } from "./ui";
import { useToastStore } from "./toast";
import { useWsStore, wsEntryToPlacement, WS_ID_PREFIX } from "./ws";
import type { WsEntry } from "../lib/wsOps";
import { useContentStore, type PlacementView } from "./content";
import { semanticSearch, type SemanticAssetHit, type SemanticMode } from "../lib/semantic";

// 后端 services/search.rs SearchHits（camelCase）
interface SearchHits {
  placements: { assetId: string; folderId: string }[];
  archives: string[];
}

// ---------- 筛选（2026-10-09 筛选业务二轮，PRD 03_查找文件 02）----------
// 条件 per-panel 持有（与后端 SearchFilters camelCase 对齐），随 search_assets scope.filters 下发；
// 口径 = 维度内 OR、跨维度 AND、×搜索 AND（2026-10-09 用户定案）；纯筛选（无关键字）同样进入
// 搜索态（结果替换素材区），isActive = 有关键字 || 有生效筛选。ws 视口筛选钮不渲染，filters 恒空。
interface DurationRange {
  unit: string;
  min?: number;
  max?: number;
}
interface ByteRange {
  min?: number;
  max?: number;
}
interface Ymd {
  year?: number;
  month?: number;
  day?: number;
}
interface DateCond {
  buckets: string[];
  custom?: Ymd | null;
}
export interface SearchFilters {
  colors: string[];
  duration: DurationRange | null;
  tags: string[];
  exts: string[];
  folderIds: string[];
  ratios: string[];
  linkStates: string[];
  size: ByteRange | null;
  imported: DateCond | null;
  modified: DateCond | null;
}

function emptyFilters(): SearchFilters {
  return {
    colors: [],
    duration: null,
    tags: [],
    exts: [],
    folderIds: [],
    ratios: [],
    linkStates: [],
    size: null,
    imported: null,
    modified: null,
  };
}

/** 筛选维度（与 FloatBarSearch 默认 filters 数组同文案；「标签」落地名 = 属性，PRD 03 §2） */
const FILTER_DIMS = ["颜色", "时长", "属性", "扩展名", "文件夹", "比例", "链接", "大小", "添加日期", "修改日期"] as const;

function dateCondActive(c: DateCond | null): boolean {
  return (
    !!c &&
    (c.buckets.length > 0 ||
      (!!c.custom && (c.custom.year != null || c.custom.month != null || c.custom.day != null)))
  );
}

/** 各维度生效判定（与后端 SearchFilters::has_active 同口径：范围双端空 / 日期空壳不算） */
function dimActives(f: SearchFilters): Record<string, boolean> {
  return {
    "颜色": f.colors.length > 0,
    "时长": !!f.duration && (f.duration.min != null || f.duration.max != null),
    "属性": f.tags.length > 0,
    "扩展名": f.exts.length > 0,
    "文件夹": f.folderIds.length > 0,
    "比例": f.ratios.length > 0,
    "链接": f.linkStates.length > 0,
    "大小": !!f.size && (f.size.min != null || f.size.max != null),
    "添加日期": dateCondActive(f.imported),
    "修改日期": dateCondActive(f.modified),
  };
}

/** 链接状态 / 扩展名 / 比例 / 日期档位特殊值的条件摘要文案 */
const LINK_STATE_LABELS: Record<string, string> = { origin: "原始", linked: "链接", thumb: "缩略图", none: "无链接" };
const EXT_LABELS: Record<string, string> = { seq: "序列帧", archive: "档案" };
const RATIO_LABELS: Record<string, string> = { landscape: "横图", square: "方形", portrait: "竖图" };
const DATE_BUCKET_LABELS: Record<string, string> = {
  today: "当天",
  yesterday: "昨天",
  week: "最近一周",
  month: "最近一月",
  year: "最近一年",
  earlier: "更早",
};
const DURATION_UNIT_LABELS: Record<string, string> = { frames: "帧", seconds: "秒", minutes: "分" };

/** 范围摘要（≥min / ≤max / min~max；大小经 formatSize 人性化） */
function rangeSummary(min: number | undefined, max: number | undefined, suffix: string, size = false): string {
  const fmt = (v: number): string => (size ? formatSize(v) : String(v));
  if (min != null && max != null) return `${fmt(min)}~${fmt(max)}${suffix}`;
  if (min != null) return `≥${fmt(min)}${suffix}`;
  if (max != null) return `≤${fmt(max)}${suffix}`;
  return "";
}

/** 日期条件摘要（档位中文 + 自定义 y-m-d 拼接，任意组合） */
function dateSummary(c: DateCond | null): string {
  if (!c) return "";
  const parts = c.buckets.map((b) => DATE_BUCKET_LABELS[b] ?? b);
  if (c.custom) {
    const custom = [c.custom.year, c.custom.month, c.custom.day].filter((v) => v != null).join("-");
    if (custom) parts.push(custom);
  }
  return parts.join("、");
}

// 后端 commands/workspace.rs WsSearchResponse
interface WsSearchResponse {
  status: "ok" | "everythingUnavailable";
  entries: WsEntry[];
}

// 后端 everything_status
interface EverythingStatusInfo {
  status: "running" | "installedNotRunning" | "notInstalled";
  version: string | null;
}

// 后端 ws_search CrawlEvent
interface CrawlEvent {
  taskId: number;
  scanned: number;
  hits: WsEntry[];
  done: boolean;
  cancelled: boolean;
}

interface CrawlState {
  taskId: number;
  query: string; // 发起时草稿（草稿变更即作废——命中按发起关键字，防张冠李戴）
  viewRel: string; // 发起时视口（切视口同理作废）
  scanned: number;
  done: boolean;
}

interface PanelSearch {
  draft: string; // 输入框草稿（输入即参与搜索）
  hits: Set<string>; // `${assetId}|${folderId}` 基底键（自洽子域：后端搜索只回二元组——同夹多条
  // 位置同亮属正常；选择 / 拖拽键的撞键后缀口径见 content.placementKey，2026-10-11）
  hitArchives: Set<string>; // 命中档案 folderId
  seq: number; // 竞态序号（防抖与异步返回只收最新）
  filters: SearchFilters; // 筛选条件（2026-10-09 筛选业务二轮；ws 视口恒空）
  wsEntries: WsEntry[]; // ws 视口命中条目（2026-10-03；视图层数据源）
  crawl: CrawlState | null; // ws 回退爬盘任务（null = 未发起 / 已结束清理）
  globalNotified: boolean; // 本段 `~` 会话已弹过指路通知（去重）
  mode: SemanticMode;
  semanticHits: Record<string, SemanticAssetHit>;
  semanticOrder: string[];
  error: string | null;
}

const DEBOUNCE_MS = 150; // inbox-fs-changed 去抖同值先例
const EVERYTHING_DOWNLOAD_URL = "https://www.voidtools.com/";

// Everything 能力 = 机器级状态：探测缓存 3 秒（FindWindow 本也廉价，缓存防输入期逐击键 invoke）；
// ws_search 回 everythingUnavailable 时直接翻 false（真值源），launch_everything 后作废旧缓存
let evCache: { up: boolean; at: number } | null = null;
const EV_CACHE_TTL = 3000;
/** Everything 能力态（UI 反应式镜像：null = 未知，true = 运行中，false = 不可用） */
const everythingUp = ref<boolean | null>(null);

let listenersStarted = false; // 爬盘事件 + 缩略图回填监听（幂等）
const crawlOwners = new Map<number, string>(); // 爬盘 taskId → panelId（事件路由）

export const useSearchStore = defineStore("search", () => {
  const panels = ref(new Map<string, PanelSearch>());
  // Ctrl+F 聚焦请求（FloatBarHost watch 比对 panelId 后聚焦自身输入框）
  const focusSeq = ref(0);
  const focusPanelId = ref("");
  // 菜单栏「查找 ▸ 筛选器」打开弹窗请求（2026-10-09 五轮；focusSeq 同模式：
  // FloatBarHost watch 比对 panelId 后展开筛选组并打开对应维度弹窗）
  const filterPopSeq = ref(0);
  const filterPopPanelId = ref("");
  const filterPopDim = ref("");

  const timers = new Map<string, ReturnType<typeof setTimeout>>();

  function ensure(panelId: string): PanelSearch {
    let s = panels.value.get(panelId);
    if (!s) {
      s = {
        draft: "", hits: new Set(), hitArchives: new Set(), seq: 0, filters: emptyFilters(), wsEntries: [], crawl: null, globalNotified: false,
        mode: "auto", semanticHits: {}, semanticOrder: [], error: null,
      };
      panels.value.set(panelId, s);
    }
    return s;
  }

  /** 关键字（草稿剥 `~` / `//` 前缀后的分词）——与后端 parse_query 同规则 */
  function keywordsOf(s: PanelSearch): string[] {
    let d = s.draft.trim();
    for (;;) {
      if (d.startsWith("//")) d = d.slice(2).trimStart();
      else if (d.startsWith("~")) d = d.slice(1).trimStart();
      else break;
    }
    return d ? d.split(/\s+/).filter(Boolean) : [];
  }

  /** 有生效筛选（与后端 has_active 同口径） */
  function hasAnyFilter(s: PanelSearch): boolean {
    return Object.values(dimActives(s.filters)).some(Boolean);
  }

  /** 搜索态 = 剥前缀后有关键字或有生效筛选（纯 `~` / `//` 且无条件不进入搜索态） */
  function isActive(panelId: string): boolean {
    const s = panels.value.get(panelId);
    return !!s && (keywordsOf(s).length > 0 || hasAnyFilter(s));
  }

  function hitsFor(panelId: string): Set<string> {
    return panels.value.get(panelId)?.hits ?? new Set();
  }

  function hitArchivesFor(panelId: string): Set<string> {
    return panels.value.get(panelId)?.hitArchives ?? new Set();
  }

  function resultPlacementsFor(panelId: string, viewKey: string): PlacementView[] {
    const s = panels.value.get(panelId);
    if (!s) return [];
    const content = useContentStore();
    const base = content.placementsFor(effectiveGlobal(panelId) ? "quick:all" : viewKey);
    const key = (p: PlacementView): string => `${p.assetId}|${p.folderId}`;
    if (s.mode === "semantic") {
      // 纯语义模式：仅语义命中，按相关度序（原口径不变）
      if (s.semanticOrder.length === 0) return [];
      const order = new Map(s.semanticOrder.map((k, index) => [k, index]));
      return base.filter((p) => order.has(key(p))).sort((left, right) => {
        const a = order.get(key(left))!;
        const b = order.get(key(right))!;
        return a - b;
      });
    }
    // 分词 / 智能模式：关键词命中按视口序；**auto 并联合并**（2026-11-02 用户实机定）——
    // 语义命中按相关度追加在后，同 assetId 去重（关键词卡优先占位，不为语义重复出卡）；
    // 语义命中卡以命中徽标 / 命中帧图为天然区分（无新 UI 元素），预览上下个同源
    const keywordResults = base.filter((p) => s.hits.has(key(p)));
    if (s.semanticOrder.length === 0) return keywordResults;
    const seen = new Set(keywordResults.map((p) => p.assetId));
    const order = new Map(s.semanticOrder.map((k, index) => [k, index]));
    const semanticResults = base
      .filter((p) => order.has(key(p)) && !seen.has(p.assetId))
      .sort((left, right) => {
        const a = order.get(key(left))!;
        const b = order.get(key(right))!;
        return a - b;
      });
    return [...keywordResults, ...semanticResults];
  }

  function semanticHitFor(panelId: string, assetId: string): SemanticAssetHit | null {
    return panels.value.get(panelId)?.semanticHits[assetId] ?? null;
  }

  function modeFor(panelId: string): SemanticMode {
    return panels.value.get(panelId)?.mode ?? "auto";
  }

  function errorFor(panelId: string): string | null {
    return panels.value.get(panelId)?.error ?? null;
  }

  // ---------- 筛选条件读写（2026-10-09 筛选业务二轮） ----------

  /** 当前筛选条件（FloatBarHost 弹窗数据源；无 panel 回空集） */
  function filtersFor(panelId: string): SearchFilters {
    return panels.value.get(panelId)?.filters ?? emptyFilters();
  }

  /** 生效维度列表（按钮 on 态数据源，顺序 = 按钮组序） */
  function activeFilterDims(panelId: string): string[] {
    const s = panels.value.get(panelId);
    if (!s) return [];
    const act = dimActives(s.filters);
    return FILTER_DIMS.filter((d) => act[d]);
  }

  /** 条件摘要（按钮 v-tip；文件夹维度需树名称解析暂不出摘要，随走查反馈补） */
  function filterSummaries(panelId: string): Record<string, string> {
    const f = panels.value.get(panelId)?.filters;
    if (!f) return {};
    const act = dimActives(f);
    const out: Record<string, string> = {};
    if (act["颜色"]) out["颜色"] = f.colors.join("、");
    if (act["时长"]) out["时长"] = rangeSummary(f.duration?.min, f.duration?.max, DURATION_UNIT_LABELS[f.duration?.unit ?? "seconds"] ?? "秒");
    if (act["属性"]) out["属性"] = f.tags.join("、");
    if (act["扩展名"]) out["扩展名"] = f.exts.map((e) => EXT_LABELS[e] ?? e).join("、");
    if (act["比例"]) out["比例"] = f.ratios.map((r) => RATIO_LABELS[r] ?? r).join("、");
    if (act["链接"]) out["链接"] = f.linkStates.map((s) => LINK_STATE_LABELS[s] ?? s).join("、");
    if (act["大小"]) out["大小"] = rangeSummary(f.size?.min, f.size?.max, "", true);
    if (act["添加日期"]) out["添加日期"] = dateSummary(f.imported);
    if (act["修改日期"]) out["修改日期"] = dateSummary(f.modified);
    return out;
  }

  /** 改某维度条件（patch 维度级整体覆盖）并按防抖重搜 */
  function patchFilters(panelId: string, viewKey: string, patch: Partial<SearchFilters>): void {
    const s = ensure(panelId);
    Object.assign(s.filters, patch);
    scheduleRun(panelId, viewKey);
  }

  /** 清空全部筛选（菜单「重置所有筛选」同径） */
  function clearAllFilters(panelId: string, viewKey: string): void {
    const s = panels.value.get(panelId);
    if (!s || !hasAnyFilter(s)) return;
    s.filters = emptyFilters();
    scheduleRun(panelId, viewKey);
  }

  /** 全局范围 = 草稿带 `~` 前缀（无范围按钮，手敲生效；后端 parse_query 同规则再剥一次） */
  function effectiveGlobal(panelId: string): boolean {
    const s = panels.value.get(panelId);
    if (!s) return false;
    let d = s.draft.trim();
    for (;;) {
      if (d.startsWith("//")) d = d.slice(2).trimStart();
      else if (d.startsWith("~")) return true;
      else break;
    }
    return false;
  }

  function draftFor(panelId: string): string {
    return panels.value.get(panelId)?.draft ?? "";
  }

  // ---------- ws 搜索（2026-10-03 双层架构） ----------

  /** ws 命中条目（ContentArea 数据源；空 = 未搜索 / 无命中） */
  function wsEntriesFor(panelId: string): WsEntry[] {
    return panels.value.get(panelId)?.wsEntries ?? [];
  }

  /** ws 命中合成 PlacementView（wsEntryToPlacement 同口径：键 `ws:{rel}|ws:{父rel}`） */
  function wsPlacementsFor(panelId: string): PlacementView[] {
    const s = panels.value.get(panelId);
    if (!s) return [];
    const ws = useWsStore();
    const rootPath = ws.root?.rootPath ?? null;
    return s.wsEntries.map((e) => {
      const i = e.relPath.lastIndexOf("/");
      return wsEntryToPlacement(e, i < 0 ? "" : e.relPath.slice(0, i), rootPath);
    });
  }

  /** 爬盘进行态（进度条数据源；done / null 不出条） */
  function crawlActiveFor(panelId: string): { scanned: number } | null {
    const c = panels.value.get(panelId)?.crawl;
    return c && !c.done ? { scanned: c.scanned } : null;
  }

  /** ws 降级提示态：搜索态且 Everything 不可用且未发起爬盘 → 空态改显「按 Enter 搜索」引导 */
  function wsNeedsEnter(panelId: string): boolean {
    const s = panels.value.get(panelId);
    return !!s && isActive(panelId) && everythingUp.value === false && s.crawl === null;
  }

  async function probeEverything(): Promise<boolean> {
    if (evCache && Date.now() - evCache.at < EV_CACHE_TTL) return evCache.up;
    try {
      const r = await invoke<EverythingStatusInfo>("everything_status");
      evCache = { up: r.status === "running", at: Date.now() };
    } catch {
      evCache = { up: false, at: Date.now() };
    }
    everythingUp.value = evCache.up;
    return evCache.up;
  }

  /** `~` 且 Everything 不可用：通知栏指路（每段 `~` 会话一次）；按已安装 / 未安装分文案与动作 */
  async function notifyGlobalUnavailable(s: PanelSearch): Promise<void> {
    if (s.globalNotified) return;
    s.globalNotified = true;
    const toast = useToastStore();
    let installed = false;
    try {
      installed = (await invoke<EverythingStatusInfo>("everything_status")).status === "installedNotRunning";
    } catch {
      /* 探测失败按未安装指路 */
    }
    if (installed) {
      toast.notify({
        message: "全局搜索需要 Everything——已安装但未运行",
        icon: "fail",
        actionLabel: "启动 Everything",
        duration: 8000,
        onAction: () => {
          void invoke("launch_everything").then(() => {
            evCache = null; // 启动后作废能力缓存，下一次搜索重探测
          });
        },
      });
    } else {
      toast.notify({
        message: "全局搜索需要 Everything 支持（当前目录可按 Enter 遍历搜索）",
        icon: "fail",
        actionLabel: "下载 Everything",
        duration: 8000,
        onAction: () => void invoke("open_external_url", { url: EVERYTHING_DOWNLOAD_URL }),
      });
    }
  }

  function cancelCrawl(s: PanelSearch): void {
    const c = s.crawl;
    if (!c) return;
    if (!c.done) void invoke("ws_search_crawl_cancel", { taskId: c.taskId }).catch(() => {});
    crawlOwners.delete(c.taskId);
    s.crawl = null;
  }

  /** ws 搜索主路径（run 的 ws 分支）：Everything 实时；不可用则清命中等 Enter（`~` 弹指路通知） */
  async function runWs(panelId: string, viewKey: string): Promise<void> {
    listenersInit(); // 缩略图回填监听（幂等）——Everything 路径命中也靠它，不能等首次爬盘才注册
    const s = ensure(panelId);
    const seq = ++s.seq;
    const kws = keywordsOf(s);
    const global = effectiveGlobal(panelId);
    if (!global) s.globalNotified = false; // `~` 摘掉即重置通知闸
    const draftNow = s.draft.trim();
    const viewRel = viewKey.slice(WS_ID_PREFIX.length);
    // 爬盘上下文（关键字 / 视口）漂移 = 旧任务作废（命中按发起时口径，不混批）
    if (s.crawl && (s.crawl.query !== draftNow || s.crawl.viewRel !== viewRel)) cancelCrawl(s);
    if (kws.length === 0) {
      s.hits = new Set();
      s.hitArchives = new Set();
      s.wsEntries = [];
      return;
    }
    const up = await probeEverything();
    if (s.seq !== seq) return;
    if (!up) {
      // 无 Everything：不冒充即时搜索——清空命中等 Enter；`~` 指路通知
      s.hits = new Set();
      s.wsEntries = [];
      if (global) void notifyGlobalUnavailable(s);
      return;
    }
    try {
      const r = await invoke<WsSearchResponse>("ws_search", { query: draftNow, viewRel });
      if (s.seq !== seq) return; // 过期响应丢弃
      if (r.status === "everythingUnavailable") {
        evCache = { up: false, at: Date.now() }; // 真值源翻转（窗口刚关 / 版本过低等）
        everythingUp.value = false;
        s.hits = new Set();
        s.wsEntries = [];
        if (global) void notifyGlobalUnavailable(s);
        return;
      }
      everythingUp.value = true;
      s.wsEntries = r.entries;
      s.hits = new Set(r.entries.map(wsHitKey));
    } catch (err) {
      // 未开空间等异常：搜索是高频路径，失败静默降级保持旧命中（toast 会刷屏）
      console.debug("工作空间搜索失败：", err);
    }
  }

  /** 命中条目 → 选择 / 拖拽同口径键（`ws:{rel}|ws:{父rel}`，与 wsEntryToPlacement 合成一致） */
  function wsHitKey(e: WsEntry): string {
    const i = e.relPath.lastIndexOf("/");
    return `${WS_ID_PREFIX}${e.relPath}|${WS_ID_PREFIX}${i < 0 ? "" : e.relPath.slice(0, i)}`;
  }

  /** Enter（FloatBarHost 转发）：Everything 可用 = 立即重搜（跳过防抖）；ws 无 Everything =
   *  显式触发爬盘回退（`~` 全局后端拒收，前端不发起直接再弹指路通知） */
  function enterPressed(panelId: string, viewKey: string): void {
    const s = panels.value.get(panelId);
    if (!s || keywordsOf(s).length === 0) return;
    if (!viewKey.startsWith(WS_ID_PREFIX)) {
      void run(panelId, viewKey);
      return;
    }
    if (everythingUp.value !== false) {
      void run(panelId, viewKey);
      return;
    }
    if (effectiveGlobal(panelId)) {
      s.globalNotified = false; // Enter 是显式动作，允许再弹
      void notifyGlobalUnavailable(s);
      return;
    }
    void startCrawl(panelId, viewKey.slice(WS_ID_PREFIX.length));
  }

  async function startCrawl(panelId: string, viewRel: string): Promise<void> {
    const s = ensure(panelId);
    cancelCrawl(s);
    const query = s.draft.trim();
    try {
      listenersInit();
      const taskId = await invoke<number>("ws_search_crawl_start", { query, viewRel });
      const crawl: CrawlState = { taskId, query, viewRel, scanned: 0, done: false };
      s.crawl = crawl;
      s.wsEntries = [];
      s.hits = new Set();
      crawlOwners.set(taskId, panelId);
    } catch (err) {
      console.debug("爬盘搜索启动失败：", err);
    }
  }

  /** 爬盘事件 + ws 缩略图回填（懒注册幂等；thumbs-ready 与 ws store 共用同一后端事件） */
  function listenersInit(): void {
    if (listenersStarted) return;
    listenersStarted = true;
    void listen<CrawlEvent>("ws-search-crawl", (e) => {
      const pid = crawlOwners.get(e.payload.taskId);
      if (!pid) return;
      const s = panels.value.get(pid);
      if (!s) return;
      if (s.crawl) s.crawl.scanned = e.payload.scanned;
      if (e.payload.hits.length > 0) {
        s.wsEntries = [...s.wsEntries, ...e.payload.hits];
        s.hits = new Set(s.wsEntries.map(wsHitKey));
      }
      if (e.payload.done) {
        if (s.crawl) s.crawl.done = true;
        crawlOwners.delete(e.payload.taskId);
      }
    });
    // 搜索命中条目的缩略图回填（ws store 的 patchThumbs 只管目录缓存，搜索结果集在本 store）
    void listen<import("../lib/wsOps").WsThumbReady[]>("workspace-thumbs-ready", (e) => {
      const ws = useWsStore();
      const byAbs = new Map(e.payload.map((it) => [it.absPath, it]));
      for (const s of panels.value.values()) {
        if (s.wsEntries.length === 0) continue;
        let changed = false;
        const next = s.wsEntries.map((en) => {
          const abs = ws.absPath(en.relPath);
          const ready = abs !== null ? byAbs.get(abs) : undefined;
          if (!ready || en.thumbPath !== null) return en;
          changed = true;
          const dims = en.w === null && ready.thumbKind === "asset" ? { w: ready.w, h: ready.h } : {};
          return { ...en, ...dims, thumbPath: ready.thumbPath, thumbKind: ready.thumbKind };
        });
        if (changed) s.wsEntries = next;
      }
    });
  }

  // ---------- 库搜索（原链路） ----------

  function clearSemantic(s: PanelSearch): void {
    s.semanticHits = {};
    s.semanticOrder = [];
  }

  async function runSemantic(s: PanelSearch, scope: Parameters<typeof semanticSearch>[1], seq: number, keepKeywordHits = false): Promise<void> {
    const query = keywordsOf(s).join(" ");
    // 并联合并（2026-11-02 用户实机定，PRD 03 智能路由节）：auto 模式保留关键词命中，
    // 语义结果追加在后；纯语义模式（手动）维持清空关键词通道的原口径
    if (!keepKeywordHits) {
      s.hits = new Set();
      s.hitArchives = new Set();
    }
    if (!query) {
      clearSemantic(s);
      return;
    }
    try {
      const result = await semanticSearch(query, scope);
      if (s.seq !== seq) return;
      s.semanticHits = Object.fromEntries(result.hits.map((hit) => [hit.best.assetId, hit]));
      s.semanticOrder = result.hits.flatMap((hit) => hit.placements.map((p) => `${p.assetId}|${p.folderId}`));
      s.error = null;
    } catch (err) {
      if (s.seq !== seq) return;
      clearSemantic(s);
      s.error = err instanceof Error ? err.message : String(err);
      console.debug("语义搜索失败：", err);
    }
  }

  async function run(panelId: string, viewKey: string): Promise<void> {
    if (viewKey.startsWith(WS_ID_PREFIX)) {
      return runWs(panelId, viewKey);
    }
    const s = panels.value.get(panelId);
    if (!s) return;
    const seq = ++s.seq;
    if (keywordsOf(s).length === 0 && !hasAnyFilter(s)) {
      s.hits = new Set();
      s.hitArchives = new Set();
      clearSemantic(s);
      s.error = null;
      return;
    }
    const scope = { viewKey, recursive: useUiStore().showSubfolders, global: effectiveGlobal(panelId), filters: s.filters };
    if (s.mode === "semantic" && keywordsOf(s).length > 0) {
      await runSemantic(s, scope, seq);
      return;
    }
    try {
      // auto 并联合并（2026-11-02 用户实机「搜左轮只出资产图片、视频出不来」修订，PRD 03）：
      // 原降级路由 = 关键词有命中即短路不跑语义，向量结果被遮盖——改为关键词与语义并发双跑，
      // 结果合并在 resultPlacementsFor（关键词视口序在前、语义按相关度去重追加）；
      // 手动「分词」模式仍只跑关键词（wantSemantic=false）
      const wantSemantic = s.mode === "auto" && keywordsOf(s).length > 0;
      const semanticJob = wantSemantic ? runSemantic(s, scope, seq, true) : null;
      const r = await invoke<SearchHits>("search_assets", { query: s.draft.trim(), scope });
      await semanticJob; // 已并发起跑（内部自捕错误不 reject），等齐收口
      if (s.seq !== seq) return;
      s.hits = new Set(r.placements.map((k) => `${k.assetId}|${k.folderId}`));
      s.hitArchives = new Set(r.archives);
      if (!wantSemantic) {
        clearSemantic(s);
        s.error = null; // 纯关键词路径维持旧清错口径；并联路径 error 归 runSemantic 自管（防吞语义错误）
      }
    } catch (err) {
      console.debug("搜索失败：", err);
    }
  }

  /** 内容变更（draft / filters）：无条件即时退出搜索态；有条件防抖 150ms 后搜 */
  function scheduleRun(panelId: string, viewKey: string): void {
    const s = panels.value.get(panelId);
    const t = timers.get(panelId);
    if (t) clearTimeout(t);
    if (!s || (keywordsOf(s).length === 0 && !hasAnyFilter(s))) {
      void run(panelId, viewKey);
      return;
    }
    timers.set(
      panelId,
      setTimeout(() => void run(panelId, viewKey), DEBOUNCE_MS),
    );
  }

  function setDraft(panelId: string, viewKey: string, text: string): void {
    ensure(panelId).draft = text;
    scheduleRun(panelId, viewKey);
  }

  function setMode(panelId: string, viewKey: string, mode: SemanticMode): void {
    const s = ensure(panelId);
    s.mode = mode;
    s.seq++;
    s.error = null;
    if (mode === "semantic") {
      s.hits = new Set();
      s.hitArchives = new Set();
    } else {
      clearSemantic(s);
    }
    scheduleRun(panelId, viewKey);
  }

  async function runNow(panelId: string, viewKey: string): Promise<string | null> {
    await run(panelId, viewKey);
    return errorFor(panelId);
  }

  /** 视图 / 递归开关变化：搜索保持并按新范围重搜（2026-08-19 定案） */
  function rerun(panelId: string, viewKey: string): void {
    if (isActive(panelId)) void run(panelId, viewKey);
  }

  async function refreshSemanticSearches(viewKeyOf: (panelId: string) => string): Promise<void> {
    const pending: Promise<void>[] = [];
    for (const [panelId, state] of panels.value) {
      if (state.mode !== "keyword" && isActive(panelId)) pending.push(run(panelId, viewKeyOf(panelId)));
    }
    await Promise.all(pending);
  }

  function removePanel(panelId: string): void {
    const s = panels.value.get(panelId);
    if (s) cancelCrawl(s);
    const t = timers.get(panelId);
    if (t) clearTimeout(t);
    timers.delete(panelId);
    panels.value.delete(panelId);
  }

  function requestFocus(panelId: string): void {
    if (!panelId) return;
    focusPanelId.value = panelId;
    focusSeq.value++;
  }

  /** 菜单栏筛选器入口：请求某 panel 悬浮栏展开筛选组并打开对应维度弹窗 */
  function requestFilterPop(panelId: string, dim: string): void {
    if (!panelId) return;
    filterPopPanelId.value = panelId;
    filterPopDim.value = dim;
    filterPopSeq.value++;
  }

  /** 全局搜索入口（菜单栏主菜单「查找 ▸ 全局搜索」，2026-09-05 用户定：聚焦搜索 + 帮用户输入 `~`）：
   *  草稿补 `~` 前缀（已带 `~` 不动只聚焦；纯 `~` 无关键字不进搜索态，等用户续输） */
  function prefillGlobal(panelId: string, viewKey: string): void {
    if (!panelId) return;
    const s = ensure(panelId);
    if (!s.draft.trimStart().startsWith("~")) setDraft(panelId, viewKey, `~${s.draft}`);
    requestFocus(panelId);
  }

  return {
    focusSeq,
    focusPanelId,
    filterPopSeq,
    filterPopPanelId,
    filterPopDim,
    isActive,
    hitsFor,
    hitArchivesFor,
    resultPlacementsFor,
    semanticHitFor,
    modeFor,
    errorFor,
    setMode,
    runNow,
    refreshSemanticSearches,
    effectiveGlobal,
    draftFor,
    setDraft,
    rerun,
    removePanel,
    requestFocus,
    requestFilterPop,
    prefillGlobal,
    // 筛选（2026-10-09 筛选业务二轮）
    filtersFor,
    activeFilterDims,
    filterSummaries,
    patchFilters,
    clearAllFilters,
    // ws 搜索（2026-10-03）
    wsEntriesFor,
    wsPlacementsFor,
    crawlActiveFor,
    wsNeedsEnter,
    enterPressed,
    cancelCrawlFor: (panelId: string) => {
      const s = panels.value.get(panelId);
      if (s) cancelCrawl(s);
    },
  };
});
