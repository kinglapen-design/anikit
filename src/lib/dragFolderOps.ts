// 文件夹拖拽落点动作（6.3 自 dragOperation 拆出，规范第 3 条行数控制——dragOperation 超 500）：
// 树 item 上/下 1/3 同级排序、中间 1/3 与文件夹卡片并入（move_folders）、文件夹卡片间隙同级排序。
// 均不读拖拽会话（入参在 pointermove 算出时捕获，与 dragOperation 的 drop 语义一致）。
import * as ops from "./placementOps";
import { useContentStore, type FolderNode } from "../stores/content";

// 树查找辅助（文件夹拖拽排序用：同级 parent / 下一兄弟）
export function findParentAndNext(
  nodes: FolderNode[],
  id: string,
  parent: string | null = null,
): { parentId: string | null; nextId: string | null } | null {
  for (let i = 0; i < nodes.length; i++) {
    if (nodes[i].id === id) {
      return { parentId: parent, nextId: nodes[i + 1]?.id ?? null };
    }
    const hit = findParentAndNext(nodes[i].children, id, nodes[i].id);
    if (hit) return hit;
  }
  return null;
}

// 树可见文件夹 id（2026-07-28 复验反馈 5）：可见展开态扁平化顺序——Shift 范围选与整组排序的
// 顺序依据；DOM 顺序即视觉顺序（命名中临时节点无 data-folder-id 天然排除）
export function visibleTreeFolderIds(): string[] {
  return [...document.querySelectorAll<HTMLElement>(".library-view .tree-item[data-folder-id]")].map(
    (el) => el.getAttribute("data-folder-id")!,
  );
}

// 文件夹落树 item 中间 1/3 / 文件夹卡片：move_folders 移动层级（2026-07-28 复验反馈 5，
// 替换原"暂不支持移动文件夹层级"toast）；整组操作，落到组成员上 = 无操作（引擎侧已拦）
export async function dropFoldersInto(folderIds: string[], targetId: string): Promise<void> {
  const ids = folderIds.filter((id) => id !== targetId);
  if (ids.length === 0) return;
  await ops.moveFolders(ids, targetId);
}

// 文件夹落树 item 上/下 1/3：reorder_folders 同级排序（before = 插到目标前 / after = 目标下一兄弟前）；
// 跨父级（2026-07-29 复验：嵌套文件夹拖出）= move_folders 到目标父级插指示线位置
export async function dropFolderBeside(
  folderIds: string[],
  targetId: string,
  zone: "before" | "after",
): Promise<void> {
  const content = useContentStore();
  const hit = findParentAndNext(content.folders, targetId);
  if (!hit) return;
  // 整组按树可见顺序；组在树全不可见（条带拖拽且树折叠）时退用引擎侧组序
  const visible = visibleTreeFolderIds().filter((id) => folderIds.includes(id));
  const ordered = visible.length > 0 ? visible : folderIds;
  const sameParent = ordered.every((id) => findParentAndNext(content.folders, id)?.parentId === hit.parentId);
  if (sameParent) {
    // 同级护栏（混排）：整组须全部与目标同父，否则无操作
    await ops.reorderFolders(ordered, hit.parentId, zone === "before" ? targetId : hit.nextId);
    return;
  }
  await ops.moveFolders(ordered, hit.parentId, zone === "before" ? targetId : hit.nextId);
}

// 文件夹落文件夹卡片间隙：条带内同级排序（parent = 当前文件夹；组按条带顺序）
export async function dropFolderCardBeside(folderIds: string[], targetId: string, after: boolean): Promise<void> {
  const content = useContentStore();
  const stripIds = content.stripFolders.map((f) => f.id);
  const ordered = stripIds.filter((id) => folderIds.includes(id));
  if (ordered.length === 0) return;
  const i = stripIds.indexOf(targetId);
  if (i < 0) return;
  await ops.reorderFolders(ordered, content.currentFolderId, after ? (stripIds[i + 1] ?? null) : targetId);
}
