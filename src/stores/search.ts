// 分词搜索（3.6 一轮，PRD 03_查找文件 01）：搜索态 per-panel 独立（搜索框在每个 dockview panel 内）。
// 匹配在后端（search_assets 命令，规范第 1 条业务层）；本 store 持有命中键集合，视图层过滤展示。
// 口径（2026-08-19 用户定）：结果替换素材区（清空恢复）/ 排序生效分组平铺 / 当前目录跟随递归开关 /
// 快速访问 = 当前视图即范围（回收站里搜回收站）/ `~` 前缀或 chip 全局 = 全库（回收站除外）。
import { ref } from "vue";
import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { QUICK_LABELS } from "./content";
import { useUiStore } from "./ui";

// 后端 services/search.rs SearchHits（camelCase）
interface SearchHits {
  placements: { assetId: string; folderId: string }[];
  archives: string[];
}

interface PanelSearch {
  query: string; // 原始输入（含前缀）
  global: boolean; // chip 全局态（`~` 前缀由后端解析本条强制，不回写 chip）
  hits: Set<string>; // `${assetId}|${folderId}`（与选择 / 拖拽键同口径）
  hitArchives: Set<string>; // 命中档案 folderId
  seq: number; // 竞态序号（防抖与异步返回只收最新）
}

const DEBOUNCE_MS = 150; // inbox-fs-changed 去抖同值先例

export const useSearchStore = defineStore("search", () => {
  const panels = ref(new Map<string, PanelSearch>());
  // Ctrl+F 聚焦请求（SearchBar watch 比对 panelId 后聚焦自身输入框）
  const focusSeq = ref(0);
  const focusPanelId = ref("");

  const timers = new Map<string, ReturnType<typeof setTimeout>>();

  function ensure(panelId: string): PanelSearch {
    let s = panels.value.get(panelId);
    if (!s) {
      s = { query: "", global: false, hits: new Set(), hitArchives: new Set(), seq: 0 };
      panels.value.set(panelId, s);
    }
    return s;
  }

  /** 查询解析（与后端 parse_query 同规则）：剥 `//` / `~` 前缀后是否剩关键字 = 搜索态 */
  function parseQuery(raw: string): { active: boolean; globalPrefix: boolean } {
    let q = raw.trim();
    let globalPrefix = false;
    for (;;) {
      if (q.startsWith("//")) q = q.slice(2).trimStart();
      else if (q.startsWith("~")) {
        globalPrefix = true;
        q = q.slice(1).trimStart();
      } else break;
    }
    return { active: q.length > 0, globalPrefix };
  }

  /** 搜索态 = 剥前缀后有关键字（"//" / "~" 纯前缀不进入搜索态） */
  function isActive(panelId: string): boolean {
    const s = panels.value.get(panelId);
    return !!s && parseQuery(s.query).active;
  }

  /** 原始输入（SearchBar 挂载时回读，面板重建不丢字） */
  function queryFor(panelId: string): string {
    return panels.value.get(panelId)?.query ?? "";
  }

  function hitsFor(panelId: string): Set<string> {
    return panels.value.get(panelId)?.hits ?? new Set();
  }
  function hitArchivesFor(panelId: string): Set<string> {
    return panels.value.get(panelId)?.hitArchives ?? new Set();
  }

  /** 生效范围全局 = chip 全局态或本条 `~` 前缀（chip 文案据此显示） */
  function effectiveGlobal(panelId: string): boolean {
    const s = panels.value.get(panelId);
    return !!s && (s.global || parseQuery(s.query).globalPrefix);
  }

  /** 范围 chip 文案：全局 / 文件夹视图「当前目录」/ 快速访问视图名 */
  function chipLabel(panelId: string, viewKey: string): string {
    if (effectiveGlobal(panelId)) return "全局";
    if (viewKey.startsWith("folder:")) return "当前目录";
    return QUICK_LABELS[viewKey.slice(6)] ?? "全部";
  }

  async function run(panelId: string, viewKey: string): Promise<void> {
    const s = panels.value.get(panelId);
    if (!s) return;
    const seq = ++s.seq;
    if (!parseQuery(s.query).active) {
      s.hits = new Set();
      s.hitArchives = new Set();
      return;
    }
    const ui = useUiStore();
    try {
      const r = await invoke<SearchHits>("search_assets", {
        query: s.query,
        scope: { viewKey, recursive: ui.showSubfolders, global: s.global },
      });
      if (s.seq !== seq) return; // 过期响应丢弃
      s.hits = new Set(r.placements.map((k) => `${k.assetId}|${k.folderId}`));
      s.hitArchives = new Set(r.archives);
    } catch (err) {
      // 未开库等异常：搜索是高频路径，失败静默降级保持旧命中（toast 会刷屏）
      console.debug("搜索失败：", err);
    }
  }

  /** 输入变更：清空即时生效；有关键字防抖 150ms 后搜 */
  function setQuery(panelId: string, viewKey: string, text: string): void {
    const s = ensure(panelId);
    s.query = text;
    const t = timers.get(panelId);
    if (t) clearTimeout(t);
    if (!parseQuery(text).active) {
      void run(panelId, viewKey);
      return;
    }
    timers.set(
      panelId,
      setTimeout(() => void run(panelId, viewKey), DEBOUNCE_MS),
    );
  }

  /** 范围 chip 点击 = 翻转默认范围（激活时立即重搜） */
  function toggleGlobal(panelId: string, viewKey: string): void {
    const s = ensure(panelId);
    s.global = !s.global;
    if (parseQuery(s.query).active) void run(panelId, viewKey);
  }

  /** 视图 / 递归开关变化：搜索保持并按新范围重搜（2026-08-19 定案） */
  function rerun(panelId: string, viewKey: string): void {
    if (isActive(panelId)) void run(panelId, viewKey);
  }

  function clear(panelId: string, viewKey: string): void {
    setQuery(panelId, viewKey, "");
  }

  function removePanel(panelId: string): void {
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

  return {
    focusSeq,
    focusPanelId,
    queryFor,
    isActive,
    hitsFor,
    hitArchivesFor,
    effectiveGlobal,
    chipLabel,
    setQuery,
    toggleGlobal,
    rerun,
    clear,
    removePanel,
    requestFocus,
  };
});
