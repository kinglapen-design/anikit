// 文件树展开态共享（6.3 右键菜单"展开/折叠 当前 / 同层级 / 所有"）：
// 原 FolderTree 每层自管 openSet——跨层级的展开 / 折叠操作无从着力，提为模块级共享。
// 展开态 = 文件夹 id 集合（会话级，与 forceOpenIds 命名强制展开并存）。
import { reactive } from "vue";
import type { FolderNode } from "../stores/content";

export const treeExpansion = reactive({
  openSet: new Set<string>(),
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
