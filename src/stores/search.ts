// 分词搜索（3.6 一轮，PRD 03_查找文件 01）：搜索态 per-panel 独立（搜索框在每个 dockview panel 内）。
// 匹配在后端（search_assets 命令，规范第 1 条业务层）；本 store 持有命中键集合，视图层过滤展示。
// 口径（2026-08-19 用户定）：结果替换素材区（清空恢复）/ 排序生效分组平铺 / 当前目录跟随递归开关 /
// 快速访问 = 当前视图即范围（回收站里搜回收站）。
// 输入形态（同日用户定，tag 化）：已提交前缀 / 关键字 = chips（`~` 全局 / `//` 搜索显式 / `/` 筛选占位 +
// 分词各自成 chip），草稿即时参与——全局 = `~` chip 或草稿 `~` 前缀（无范围按钮）。
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
  chips: string[]; // 已提交 tag："~" / "//" / "/"（占位）/ 关键字
  draft: string; // 输入框草稿（未提交也即时参与搜索）
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
      s = { chips: [], draft: "", hits: new Set(), hitArchives: new Set(), seq: 0 };
      panels.value.set(panelId, s);
    }
    return s;
  }

  /** 拼成后端 query 字符串：`~` chip → "~" 前缀；`//` chip → "//" 前缀；`/` 筛选 chip 占位不参与；
      关键字 chips 与草稿原样拼接（草稿内的 `~` / `//` 前缀由后端同规则解析） */
  function fullQuery(s: PanelSearch): string {
    const parts: string[] = [];
    if (s.chips.includes("~")) parts.push("~");
    if (s.chips.includes("//")) parts.push("//");
    for (const c of s.chips) if (c !== "~" && c !== "//" && c !== "/") parts.push(c);
    if (s.draft.trim()) parts.push(s.draft.trim());
    return parts.join(" ");
  }

  /** 关键字（chips 的非前缀项 + 草稿剥前缀后的词）——搜索态 = 至少一个关键字 */
  function keywordsOf(s: PanelSearch): string[] {
    const kws = s.chips.filter((c) => c !== "~" && c !== "//" && c !== "/");
    let d = s.draft.trim();
    for (;;) {
      if (d.startsWith("//")) d = d.slice(2).trimStart();
      else if (d.startsWith("~")) d = d.slice(1).trimStart();
      else break;
    }
    if (d) kws.push(...d.split(/\s+/));
    return kws.filter(Boolean);
  }

  /** 搜索态 = 剥前缀后有关键字（纯 `~` / `//` 不占位搜索） */
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

  /** 全局范围 = 有 `~` chip 或草稿带 `~` 前缀（无范围按钮，tag 即范围） */
  function effectiveGlobal(panelId: string): boolean {
    const s = panels.value.get(panelId);
    if (!s) return false;
    if (s.chips.includes("~")) return true;
    let d = s.draft.trim();
    for (;;) {
      if (d.startsWith("//")) d = d.slice(2).trimStart();
      else if (d.startsWith("~")) return true;
      else break;
    }
    return false;
  }

  function chipsFor(panelId: string): string[] {
    return panels.value.get(panelId)?.chips ?? [];
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
        query: fullQuery(s),
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

  /** 内容变更（chips 或 draft）：无关键字即时退出搜索态；有关键字防抖 150ms 后搜 */
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

  /** 提交 chip（`~` 唯一——重复输入不追加；空文本忽略） */
  function addChip(panelId: string, viewKey: string, chip: string): void {
    const s = ensure(panelId);
    const v = chip.trim();
    if (!v) return;
    if (v === "~" && s.chips.includes("~")) return;
    s.chips.push(v);
    scheduleRun(panelId, viewKey);
  }

  function removeChip(panelId: string, viewKey: string, chip: string): void {
    const s = panels.value.get(panelId);
    if (!s) return;
    const i = s.chips.lastIndexOf(chip);
    if (i >= 0) s.chips.splice(i, 1);
    scheduleRun(panelId, viewKey);
  }

  /** 空草稿 Backspace = 删尾 chip（TermChipsInput 口径） */
  function popChip(panelId: string, viewKey: string): void {
    const s = panels.value.get(panelId);
    if (!s || s.chips.length === 0) return;
    s.chips.pop();
    scheduleRun(panelId, viewKey);
  }

  /** 视图 / 递归开关变化：搜索保持并按新范围重搜（2026-08-19 定案） */
  function rerun(panelId: string, viewKey: string): void {
    if (isActive(panelId)) void run(panelId, viewKey);
  }

  function clear(panelId: string, viewKey: string): void {
    const s = panels.value.get(panelId);
    if (!s) return;
    s.chips = [];
    s.draft = "";
    scheduleRun(panelId, viewKey);
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
    isActive,
    hitsFor,
    hitArchivesFor,
    effectiveGlobal,
    chipsFor,
    draftFor,
    setDraft,
    addChip,
    removeChip,
    popChip,
    rerun,
    clear,
    removePanel,
    requestFocus,
  };
});
