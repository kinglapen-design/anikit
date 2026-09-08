// 分词搜索（3.6 一轮，PRD 03_查找文件 01）：搜索态 per-panel 独立（搜索框在每个 dockview panel 内）。
// 匹配在后端（search_assets 命令，规范第 1 条业务层）；本 store 持有命中键集合，视图层过滤展示。
// 口径（2026-08-19 用户定）：结果替换素材区（清空恢复）/ 排序生效分组平铺 / 当前目录跟随递归开关 /
// 快速访问 = 当前视图即范围（回收站里搜回收站）。
// 输入形态（2026-08-31 用户定案，7.1 二十二轮悬浮栏接入）：多功能悬浮栏单行输入实时分词——
// tag 化 chips（`~` / `//` / `/` 前缀 tag、空格成 chip、kbd 提示组）废除，筛选与搜索分离
//（筛选 = 悬浮栏筛选按钮组 + 弹窗，随筛选轮点亮）；`~` 手敲全局保留（后端前缀协议不变）。
import { ref } from "vue";
import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { useUiStore } from "./ui";

// 后端 services/search.rs SearchHits（camelCase）
interface SearchHits {
  placements: { assetId: string; folderId: string }[];
  archives: string[];
}

interface PanelSearch {
  draft: string; // 输入框草稿（输入即参与搜索）
  hits: Set<string>; // `${assetId}|${folderId}`（与选择 / 拖拽键同口径）
  hitArchives: Set<string>; // 命中档案 folderId
  seq: number; // 竞态序号（防抖与异步返回只收最新）
}

const DEBOUNCE_MS = 150; // inbox-fs-changed 去抖同值先例

export const useSearchStore = defineStore("search", () => {
  const panels = ref(new Map<string, PanelSearch>());
  // Ctrl+F 聚焦请求（FloatBarHost watch 比对 panelId 后聚焦自身输入框）
  const focusSeq = ref(0);
  const focusPanelId = ref("");

  const timers = new Map<string, ReturnType<typeof setTimeout>>();

  function ensure(panelId: string): PanelSearch {
    let s = panels.value.get(panelId);
    if (!s) {
      s = { draft: "", hits: new Set(), hitArchives: new Set(), seq: 0 };
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

  async function run(panelId: string, viewKey: string): Promise<void> {
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
  };
});
