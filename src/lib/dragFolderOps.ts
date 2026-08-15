// 文件夹拖拽落点动作（6.3 自 dragOperation 拆出，规范第 3 条行数控制——dragOperation 超 500）：
// 树 item 上/下 1/3 同级排序、中间 1/3 与文件夹卡片并入（move_folders）、文件夹卡片间隙同级排序；
// 2026-08-14 区块子系统：文件夹落区块标题（dropFoldersIntoBlock）。
// 均不读拖拽会话（入参在 pointermove 算出时捕获，与 dragOperation 的 drop 语义一致）。
import * as ops from "./placementOps";
import { useContentStore, DEFAULT_BLOCK_ID, type FolderNode } from "../stores/content";
import { useUiStore } from "../stores/ui";

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
// 顺序依据；DOM 顺序即视觉顺序（命名中临时节点无 data-folder-id 天然排除）。
// scope（2026-08-14 区块子系统）：多区块后按区块容器（.block-tree）收窄——跨区块 Shift 范围选 /
// 排序顺序都没有意义；null = 原全局 .library-view 口径
export function visibleTreeFolderIds(scope: ParentNode | null = null): string[] {
  const root: ParentNode = scope ?? document;
  const selector = scope ? ".tree-item[data-folder-id]" : ".library-view .tree-item[data-folder-id]";
  return [...root.querySelectorAll<HTMLElement>(selector)].map((el) => el.getAttribute("data-folder-id")!);
}

// 文件夹的区块容器元素（落点目标区块的 .block-tree；目标 item 可能嵌套，closest 直达区块容器）
function blockTreeScopeOf(folderId: string): HTMLElement | null {
  const item = document.querySelector<HTMLElement>(
    `.library-view .tree-item[data-folder-id="${CSS.escape(folderId)}"]`,
  );
  return item?.closest<HTMLElement>(".block-tree") ?? null;
}

// 顶级文件夹的区块归属（null/缺省 = 默认区块「目录」）；非顶级返回 null（不参与区块判定）
function topBlockOf(content: ReturnType<typeof useContentStore>, id: string): string | null {
  const node = content.folders.find((n) => n.id === id);
  if (!node) return null;
  return node.block ?? DEFAULT_BLOCK_ID;
}

// 文件夹落树 item 中间 1/3 / 文件夹卡片：move_folders 移动层级（2026-07-28 复验反馈 5，
// 替换原"暂不支持移动文件夹层级"toast）；整组操作，落到组成员上 = 无操作（引擎侧已拦）
export async function dropFoldersInto(folderIds: string[], targetId: string): Promise<void> {
  const ids = folderIds.filter((id) => id !== targetId);
  if (ids.length === 0) return;
  await ops.moveFolders(ids, targetId);
}

// 文件夹落区块标题（2026-08-14 区块子系统）：移入该区块根级末尾——
// targetParent = null + targetBlock = 区块 id（"dir" = 移回「目录」）。
// 区块标题互拖排序不在此——LibraryView 本地让位式拖拽直调 blockOps.reorderBlocks
export async function dropFoldersIntoBlock(folderIds: string[], blockId: string): Promise<void> {
  if (folderIds.length === 0) return;
  await ops.moveFolders(folderIds, null, null, blockId);
}

// 文件夹落树 item 上/下 1/3：reorder_folders 同级排序（before = 插到目标前 / after = 目标下一兄弟前）；
// 跨父级（2026-07-29 复验：嵌套文件夹拖出）= move_folders 到目标父级插指示线位置；
// 跨区块（2026-08-14 区块子系统）：顶级目标（parentId null）带上目标区块 targetBlock——
// 同区块同级 = 排序，跨区块 = 跨父移动逻辑同一入口（move_folders 落根级并改区块归属）
export async function dropFolderBeside(
  folderIds: string[],
  targetId: string,
  zone: "before" | "after",
): Promise<void> {
  const content = useContentStore();
  const hit = findParentAndNext(content.folders, targetId);
  if (!hit) return;
  // 整组按树可见顺序（按目标区块容器收窄）；组在树全不可见（条带拖拽且树折叠）时退用引擎侧组序
  const scopeIds = visibleTreeFolderIds(blockTreeScopeOf(targetId));
  const visible = scopeIds.filter((id) => folderIds.includes(id));
  const ordered = visible.length > 0 ? visible : folderIds;
  // 同级护栏（混排）：整组须全部与目标同父；顶级目标还要求同区块（不同区块的顶级文件夹
  // parent 同为 null，但分属不同区块——直接 reorder 会把 order 混写跨区块）
  const targetBlock = hit.parentId === null ? topBlockOf(content, targetId) : null;
  const sameParent = ordered.every((id) => {
    const h = findParentAndNext(content.folders, id);
    if (h?.parentId !== hit.parentId) return false;
    return hit.parentId !== null || topBlockOf(content, id) === targetBlock;
  });
  if (sameParent) {
    // 名称排序显示下拖拽（2026-08-14 用户实机"显示排序成功但实际没变"）：该层显示序 ≠ 存储序时，
    // 先把可见顺序物化为自定义顺序（全层按落点后序列整体重写）并切回"自定义"排序——
    // 否则 refresh 后名称排序盖回去，拖拽等于没执行
    const ui = useUiStore();
    const levelSorted =
      ui.treeSort !== "custom" && (ui.treeSort.startsWith("all") || hit.parentId === ui.treeSortAnchor);
    if (levelSorted) {
      const levelIds = scopeIds.filter((id) => {
        const h = findParentAndNext(content.folders, id);
        if (h?.parentId !== hit.parentId) return false;
        return hit.parentId !== null || topBlockOf(content, id) === targetBlock;
      });
      const rest = levelIds.filter((id) => !ordered.includes(id));
      const at = rest.indexOf(targetId); // 目标恒不在移动组内（引擎已拦拖到自身组）
      if (at >= 0) {
        const insert = zone === "before" ? at : at + 1;
        const seq = [...rest.slice(0, insert), ...ordered, ...rest.slice(insert)];
        await ops.reorderFolders(seq, hit.parentId, null); // 全层序列整体重写（锚点 null = 末尾）
        ui.treeSort = "custom";
        ui.treeSortAnchor = null;
        return;
      }
    }
    await ops.reorderFolders(ordered, hit.parentId, zone === "before" ? targetId : hit.nextId);
    return;
  }
  // 跨父/跨区块移动：落根级（hit.parentId null）时传目标区块；落进文件夹内不传 targetBlock
  await ops.moveFolders(ordered, hit.parentId, zone === "before" ? targetId : hit.nextId, targetBlock);
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
