import { reactive } from "vue";
import { backend } from "./lib/backend";
import type { OpenDoc, OutlineItem, TemplateFile } from "./lib/types";
import { parseMenuFile, applyTemplateData } from "./editor/templatesConfig";
import type { MenuMap, ChainDef } from "./editor/templatesConfig";

const RECENT_KEY = "promptmd-recent";
const RECENT_MAX = 20;

interface Store {
  theme: "light" | "dark";
  recent: string[];
  doc: OpenDoc | null;
  outline: OutlineItem[];
  jumpPos: number | null;
  newMenuOpen: boolean;
  docTemplates: TemplateFile[];
  menuFiles: TemplateFile[];
  chainsFile: TemplateFile | null;
}

function loadRecent(): string[] {
  try {
    const raw = JSON.parse(localStorage.getItem(RECENT_KEY) || "[]");
    return Array.isArray(raw) ? raw.filter((x) => typeof x === "string") : [];
  } catch {
    return [];
  }
}

export const store = reactive<Store>({
  theme: (localStorage.getItem("promptmd-theme") as "light" | "dark") || "light",
  recent: loadRecent(),
  doc: null,
  outline: [],
  jumpPos: null,
  newMenuOpen: false,
  docTemplates: [],
  menuFiles: [],
  chainsFile: null,
});

export function toggleTheme() {
  store.theme = store.theme === "light" ? "dark" : "light";
  localStorage.setItem("promptmd-theme", store.theme);
}

function pushRecent(path: string) {
  store.recent = [path, ...store.recent.filter((p) => p !== path)].slice(0, RECENT_MAX);
  localStorage.setItem(RECENT_KEY, JSON.stringify(store.recent));
}

export function removeRecent(path: string) {
  store.recent = store.recent.filter((p) => p !== path);
  localStorage.setItem(RECENT_KEY, JSON.stringify(store.recent));
}

export async function openFile(path: string) {
  if (store.doc && store.doc.path !== path && isDirty()) await saveDoc();
  try {
    const content = await backend.readMarkdown(path);
    const name = path.split(/[\\/]/).pop() || path;
    store.doc = { path, name, content, saved: content };
    pushRecent(path);
  } catch (e) {
    removeRecent(path);
    alert(`打开失败：${e}`);
  }
}

export async function openViaDialog() {
  const path = await backend.openMdFile();
  if (path) await openFile(path);
}

export function markContent(content: string) {
  if (store.doc) store.doc.content = content;
}

export function isDirty(): boolean {
  return !!store.doc && store.doc.content !== store.doc.saved;
}

export async function saveDoc() {
  if (!store.doc || !isDirty()) return;
  await backend.writeMarkdown(store.doc.path, store.doc.content);
  store.doc.saved = store.doc.content;
}

export function toggleNewMenu() {
  store.newMenuOpen = !store.newMenuOpen;
}

export function closeNewMenu() {
  store.newMenuOpen = false;
}

export async function newFromTemplate(tpl: TemplateFile | null) {
  closeNewMenu();
  const content = tpl ? await backend.readMarkdown(tpl.path) : "";
  const defaultName = `${tpl ? tpl.name : "未命名"}.md`;
  const target = await backend.saveMdAs(defaultName);
  if (!target) return;
  const path = /\.md$/i.test(target) ? target : `${target}.md`;
  await backend.writeMarkdown(path, content);
  await openFile(path);
}

export async function loadTemplateConfig() {
  try {
    const files = await backend.templateFiles();
    store.docTemplates = files.filter((f) => f.group === "docs");
    store.menuFiles = files.filter((f) => f.group === "menus");
    store.chainsFile = files.find((f) => f.group === "chains") ?? null;

    const menus: MenuMap = {};
    for (const f of store.menuFiles) {
      menus[f.name] = parseMenuFile(await backend.readMarkdown(f.path));
    }
    let chains: ChainDef[] = [];
    if (store.chainsFile) {
      const parsed = JSON.parse(await backend.readMarkdown(store.chainsFile.path)) as {
        chains?: ChainDef[];
      };
      chains = (parsed.chains ?? []).filter((c) => c.rules?.length > 0);
    }
    applyTemplateData(menus, chains);
  } catch (e) {
    console.warn("模板配置加载失败，使用内置默认", e);
  }
}

export function setOutline(items: OutlineItem[]) {
  store.outline = items;
}

export function jumpTo(pos: number) {
  store.jumpPos = pos;
}

export function clearJump() {
  store.jumpPos = null;
}
