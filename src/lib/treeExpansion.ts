// 文件树展开态共享（6.3 右键菜单"展开/折叠 当前 / 同层级 / 所有"）：
// 原 FolderTree 每层自管 openSet——跨层级的展开 / 折叠操作无从着力，提为模块级共享。
// 展开态分两集：库 openSet = 文件夹 id（会话级，与 forceOpenIds 命名强制展开并存）；
// ws wsOpenSet = relPath（按空间持久化，见下方「ws 树展开态」段）。
import { reactive } from "vue";
import type { FolderNode } from "../stores/content";
import { useUiStore } from "../stores/ui";
import { wsDirsExist } from "./wsOps";

export const treeExpansion = reactive({
  openSet: new Set<string>(),
  wsOpenSet: new Set<string>(),
});

export function toggleOpen(id: string): void {
  if (treeExpansion.openSet.has(id)) treeExpansion.openSet.delete(id);
  else treeExpansion.openSet.add(id);
}

export function isTreeOpen(id: string): boolean {
  return treeExpansion.openSet.has(id);
}

function setOpen(ids: string[], open: boolean): void {
  for (const id of ids) {
    if (open) treeExpansion.openSet.add(id);
    else treeExpansion.openSet.delete(id);
  }
}

/** 收集含子级的全部文件夹 id（展开所有用） */
function allFolderIds(nodes: FolderNode[], out: string[] = []): string[] {
  for (const n of nodes) {
    if (n.children.length > 0) {
      out.push(n.id);
      allFolderIds(n.children, out);
    }
  }
  return out;
}

export function expandAll(nodes: FolderNode[]): void {
  setOpen(allFolderIds(nodes), true);
}

export function collapseAll(): void {
  treeExpansion.openSet.clear();
}

/** 同层级（同父）含子级的文件夹 id 集；parentId = null 表顶层 */
function levelIds(nodes: FolderNode[], parentId: string | null): string[] {
  const level = parentId === null ? nodes : (findChildren(nodes, parentId) ?? []);
  return level.filter((n) => n.children.length > 0).map((n) => n.id);
}

function findChildren(nodes: FolderNode[], id: string): FolderNode[] | null {
  for (const n of nodes) {
    if (n.id === id) return n.children;
    const hit = findChildren(n.children, id);
    if (hit) return hit;
  }
  return null;
}

export function expandLevel(nodes: FolderNode[], parentId: string | null): void {
  setOpen(levelIds(nodes, parentId), true);
}

export function collapseLevel(nodes: FolderNode[], parentId: string | null): void {
  setOpen(levelIds(nodes, parentId), false);
}

// —— 区块展开态（2026-08-14 区块子系统，与本模块 openSet 同范式：模块级会话态）——
// 语义默认开，故记录的是折叠集（新区块天然展开，无需种子初始化）
export const blockExpansion = reactive({
  closedSet: new Set<string>(),
});

export function isBlockOpen(blockId: string): boolean {
  return !blockExpansion.closedSet.has(blockId);
}

export function toggleBlockOpen(blockId: string): void {
  if (blockExpansion.closedSet.has(blockId)) blockExpansion.closedSet.delete(blockId);
  else blockExpansion.closedSet.add(blockId);
}

// 强制展开（区块内新建文件夹进命名态时保证新节点可见）
export function openBlock(blockId: string): void {
  blockExpansion.closedSet.delete(blockId);
}

// —— ws 树展开态（2026-10-12 用户报「视口记住上次停留文件夹、树展开每次都要重新展开」补落）——
// 与库 openSet 分设：库键 = 文件夹 id（会话级不变），ws 键 = relPath，按空间持久化于
// settings.ui 的 wsTreeExpanded（Record<空间 rootPath, 展开 relPath 清单>，ui store
// PERSIST_KEYS 防抖写回同口径，后端零改动）。按空间分键 = 踩坑 100 教训（路径类记忆
// 跨空间共享重放必出幽灵态）；恢复时 wsDirsExist 批量 stat 顺带剔除已删 / 改名目录的
// 僵尸键（pruneWsViewKeys 同口径，判定失败保守不清）。

// ws 展开态当前所属空间（rootPath）；null = 尚未恢复——恢复完成前的展开改动不往设置写，
// 防启动竞态（设置未加载完树已可点）把持久化清单冲成空
let wsExpandedSpace: string | null = null;

export function isWsTreeOpen(rel: string): boolean {
  return treeExpansion.wsOpenSet.has(rel);
}

export function toggleWsOpen(rel: string): void {
  if (treeExpansion.wsOpenSet.has(rel)) treeExpansion.wsOpenSet.delete(rel);
  else treeExpansion.wsOpenSet.add(rel);
  syncWsExpanded();
}

// 展开变化即写回 ui store（$subscribe 500ms 防抖落盘，PERSIST_KEYS 全字段同径）
function syncWsExpanded(): void {
  if (wsExpandedSpace === null) return;
  const ui = useUiStore();
  ui.wsTreeExpanded = { ...ui.wsTreeExpanded, [wsExpandedSpace]: [...treeExpansion.wsOpenSet] };
}

// 空间 root 就绪后恢复本空间展开态（WorkspaceTree 根实例 onMounted / watch ws.root 调用）。
// 幂等——同空间重复调用跳过，会话内展开变化不被回拉覆盖；切空间（rootPath 变）才重恢复。
// 代际守卫（ws store epoch 同模式）：恢复在途又切空间时旧空间结果兑现即丢弃，
// 防旧清单盖掉新空间刚恢复的展开态
let wsRestoreEpoch = 0;
export async function restoreWsExpanded(rootPath: string): Promise<void> {
  if (wsExpandedSpace === rootPath) return;
  const gen = ++wsRestoreEpoch;
  wsExpandedSpace = null; // 恢复期间禁写回（半途状态不落盘）
  const ui = useUiStore();
  await ui.initSettings();
  let rels = ui.wsTreeExpanded[rootPath] ?? [];
  if (rels.length > 0) {
    const alive = await wsDirsExist(rels).catch(() => null);
    if (alive) rels = rels.filter((_, i) => alive[i]);
  }
  if (gen !== wsRestoreEpoch) return;
  treeExpansion.wsOpenSet.clear();
  for (const rel of rels) treeExpansion.wsOpenSet.add(rel);
  wsExpandedSpace = rootPath;
  syncWsExpanded(); // 僵尸键剔除结果回写归一（无剔除也是恒等写）
}
