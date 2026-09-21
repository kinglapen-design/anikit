#!/usr/bin/env node
// 组件台账生成器（阶段七重构 R0）：扫描 src/components 全部组件，
// 统计消费方数（src/ 内 import 计数，排除 src/preview）与 legacy token 引用数，
// 输出 src/preview/inventory.generated.ts 供预览页「组件台账」展区渲染。
// 用法：node tools/gen-component-inventory.mjs（或 pnpm inventory）
import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const srcDir = join(root, "src");
const componentsDir = join(srcDir, "components");
const previewDir = join(srcDir, "preview");
const outFile = join(previewDir, "inventory.generated.ts");

// 新体系子目录：目录内组件一律算新体系件
const NEW_DIRS = new Set(["bars", "buttons", "cards", "inputs", "items", "modals", "plugins", "sections"]);
// 根目录旧件待烧清单（2026-09-20 盘点定案，烧一件删一行；
// AppMenu/ScrollThumb 查实为已换代件挪白名单；AppCheckbox/AppRadio/AppSegmented/AppSwitch/AppTag/AppInput/AppSearchBar/IconButton 已烧删
//（IconButton 四变体定案：deleteRed→clear X bare sm、danger→禁用灰，2026-09-20 用户定）；
// AppButton/AppModal 2026-09-21 烧删（OrphanInfosAlert 换代 ConfirmModal warning 单钮 + open-in-explorer 入口，消费方清零）——旧件清零
const LEGACY = new Map([]);
// 根目录里的新体系件 / 已换代壳件（preview 已验收）
const NEW_AT_ROOT = new Set([
  "AppIcon", "MenuItem", "AppMenu", "ScrollThumb", "ToastStack", "PickerBanner", "LinkListPopover", "EmptyFolderState",
  "CreateLibraryModal", "MergeLibraryModal", "ImportDialog", "SequenceDialog",
  "AppConfirm", "ListRowCard", "ListHeader",
]);

function walk(dir, acc = []) {
  for (const name of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, name.name);
    if (name.isDirectory()) walk(p, acc);
    else if (/\.(vue|ts)$/.test(name.name)) acc.push(p);
  }
  return acc;
}

const norm = (p) => p.replace(/\\/g, "/");
const componentFiles = walk(componentsDir).filter((f) => f.endsWith(".vue"));
const srcFiles = walk(srcDir);
const nonPreviewFiles = srcFiles.filter((f) => !norm(f).startsWith(norm(previewDir)));

const componentSet = new Map(componentFiles.map((f) => [norm(f), f]));
const consumers = new Map(componentFiles.map((f) => [norm(f), 0]));

const importRe = /import\s+(?:[^'"()]*?\s+from\s+)?["']([^"']+)["']|import\s*\(\s*["']([^"']+)["']\s*\)/g;
for (const file of nonPreviewFiles) {
  if (componentSet.has(norm(file)) && false) continue;
  const text = readFileSync(file, "utf8");
  const seen = new Set();
  for (const m of text.matchAll(importRe)) {
    const spec = m[1] ?? m[2];
    if (!spec || !spec.startsWith(".")) continue;
    const target = norm(resolve(dirname(file), spec));
    if (componentSet.has(target) && !seen.has(target)) {
      seen.add(target);
      consumers.set(target, (consumers.get(target) ?? 0) + 1);
    }
  }
}
// 自递归引用（如 FolderPickNode 引自己）不计消费方
for (const [path] of componentSet) {
  const text = readFileSync(path, "utf8");
  const selfName = path.split("/").pop();
  void selfName;
}

const legacyTokenRe = /var\(\s*--(c[1-5]\b|text-[a-z0-9-]+|brand\b)/g;

const entries = componentFiles
  .map((file) => {
    const rel = norm(relative(componentsDir, file)).replace(/\.vue$/, "");
    const name = rel.split("/").pop();
    const dir = rel.includes("/") ? rel.split("/")[0] : "";
    const text = readFileSync(file, "utf8");
    const legacyTokens = (text.match(legacyTokenRe) ?? []).length;
    const consumerCount = consumers.get(norm(file)) ?? 0;
    let status;
    if (dir && NEW_DIRS.has(dir)) status = "new";
    else if (LEGACY.has(name)) status = "legacy";
    else if (NEW_AT_ROOT.has(name)) status = "new";
    else if (consumerCount === 0) status = "dead";
    else status = "business";
    return {
      path: `components/${rel}`,
      name,
      status,
      consumers: consumerCount,
      legacyTokens,
      replacement: LEGACY.get(name) ?? "",
    };
  })
  .sort((a, b) => {
    const order = { legacy: 0, dead: 1, business: 2, new: 3 };
    return order[a.status] - order[b.status] || b.consumers - a.consumers || a.path.localeCompare(b.path);
  });

const totals = {
  new: entries.filter((e) => e.status === "new").length,
  legacy: entries.filter((e) => e.status === "legacy").length,
  business: entries.filter((e) => e.status === "business").length,
  dead: entries.filter((e) => e.status === "dead").length,
  legacyConsumers: entries.filter((e) => e.status === "legacy").reduce((s, e) => s + e.consumers, 0),
  legacyTokens: entries.reduce((s, e) => s + e.legacyTokens, 0),
};

const today = new Date().toISOString().slice(0, 10);
const body = `// 本文件由 tools/gen-component-inventory.mjs 生成（${today}），勿手改——重跑 \`pnpm inventory\` 刷新。
// 阶段七重构「组件台账」数据源：status = new 新体系 / legacy 旧件待烧 / business 业务件 / dead 零消费休眠。
export type InventoryStatus = "new" | "legacy" | "business" | "dead";
export interface InventoryEntry {
  path: string;
  name: string;
  status: InventoryStatus;
  consumers: number;
  legacyTokens: number;
  replacement: string;
}
export const inventoryDate = "${today}";
export const inventoryTotals = ${JSON.stringify(totals, null, 2)};
export const inventory: InventoryEntry[] = ${JSON.stringify(entries, null, 2)};
`;

writeFileSync(outFile, body);
console.log(`inventory.generated.ts written: ${entries.length} components`);
console.log(`  new=${totals.new} legacy=${totals.legacy} business=${totals.business} dead=${totals.dead}`);
console.log(`  legacy consumers=${totals.legacyConsumers} legacyTokens(in components)=${totals.legacyTokens}`);
const deadNames = entries.filter((e) => e.status === "dead").map((e) => e.name);
if (deadNames.length) console.log(`  dead: ${deadNames.join(", ")}`);
