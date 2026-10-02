// 分词搜索（3.6 一轮，PRD 03_查找文件 01）：搜索态 per-panel 独立（搜索框在每个 dockview panel 内）。
// 匹配在后端（search_assets 命令，规范第 1 条业务层）；本 store 持有命中键集合，视图层过滤展示。
// 口径（2026-08-19 用户定）：结果替换素材区（清空恢复）/ 排序生效分组平铺 / 当前目录跟随递归开关 /
// 快速访问 = 当前视图即范围（回收站里搜回收站）。
// 输入形态（2026-08-31 用户定案，7.1 二十二轮悬浮栏接入）：多功能悬浮栏单行输入实时分词——
// tag 化 chips（`~` / `//` / `/` 前缀 tag、空格成 chip、kbd 提示组）废除，筛选与搜索分离
//（筛选 = 悬浮栏筛选按钮组 + 弹窗，随筛选轮点亮）；`~` 手敲全局保留（后端前缀协议不变）。
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
import { useUiStore } from "./ui";
import { useToastStore } from "./toast";
import { useWsStore, wsEntryToPlacement, WS_ID_PREFIX } from "./ws";
import type { WsEntry } from "../lib/wsOps";
import type { PlacementView } from "./content";

// 后端 services/search.rs SearchHits（camelCase）
interface SearchHits {
  placements: { assetId: string; folderId: string }[];
  archives: string[];
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
  hits: Set<string>; // `${assetId}|${folderId}`（与选择 / 拖拽键同口径）
  hitArchives: Set<string>; // 命中档案 folderId
  seq: number; // 竞态序号（防抖与异步返回只收最新）
  wsEntries: WsEntry[]; // ws 视口命中条目（2026-10-03；视图层数据源）
  crawl: CrawlState | null; // ws 回退爬盘任务（null = 未发起 / 已结束清理）
  globalNotified: boolean; // 本段 `~` 会话已弹过指路通知（去重）
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

  const timers = new Map<string, ReturnType<typeof setTimeout>>();

  function ensure(panelId: string): PanelSearch {
    let s = panels.value.get(panelId);
    if (!s) {
      s = { draft: "", hits: new Set(), hitArchives: new Set(), seq: 0, wsEntries: [], crawl: null, globalNotified: false };
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

  /** 搜索态 = 剥前缀后有关键字（纯 `~` / `//` 不进入搜索态） */
  function isActive(panelId: string): boolean {
    const s = panels.value.get(panelId);
    return !!s && keywordsOf(s).length > 0;
  }

  function hitsFor(panelId: string): Set<string> {
    return panels.value.get(panelId)?.hits ?? new Set();
  }

  function hitArchivesFor(panelId: string): Set<string> {
    return panels.value.get(panelId)?.hitArchives ?? new Set();
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

  async function run(panelId: string, viewKey: string): Promise<void> {
    if (viewKey.startsWith(WS_ID_PREFIX)) {
      return runWs(panelId, viewKey);
    }
    const s = panels.value.get(panelId);
    if (!s) return;
    const seq = ++s.seq;
    if (keywordsOf(s).length === 0) {
      s.hits = new Set();
      s.hitArchives = new Set();
      return;
    }
    const ui = useUiStore();
    try {
      const r = await invoke<SearchHits>("search_assets", {
        query: s.draft.trim(),
        scope: { viewKey, recursive: ui.showSubfolders, global: effectiveGlobal(panelId) },
      });
      if (s.seq !== seq) return; // 过期响应丢弃
      s.hits = new Set(r.placements.map((k) => `${k.assetId}|${k.folderId}`));
      s.hitArchives = new Set(r.archives);
    } catch (err) {
      // 未开库等异常：搜索是高频路径，失败静默降级保持旧命中（toast 会刷屏）
      console.debug("搜索失败：", err);
    }
  }

  /** 内容变更（draft）：无关键字即时退出搜索态；有关键字防抖 150ms 后搜 */
  function scheduleRun(panelId: string, viewKey: string): void {
    const s = panels.value.get(panelId);
    const t = timers.get(panelId);
    if (t) clearTimeout(t);
    if (!s || keywordsOf(s).length === 0) {
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

  /** 视图 / 递归开关变化：搜索保持并按新范围重搜（2026-08-19 定案） */
  function rerun(panelId: string, viewKey: string): void {
    if (isActive(panelId)) void run(panelId, viewKey);
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
    isActive,
    hitsFor,
    hitArchivesFor,
    effectiveGlobal,
    draftFor,
    setDraft,
    rerun,
    removePanel,
    requestFocus,
    prefillGlobal,
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
