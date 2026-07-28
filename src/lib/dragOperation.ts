// 拖拽引擎（6.1 阶段 C4，2026-07-28 用户定案）：自实现 pointer 拖拽，禁用 HTML5 DnD
// （避免与系统拖入导入 onDragDropEvent 冲突；卡片 / 树 item 上 pointerdown 移动超阈值启动，
// 未过阈值松开 = 原有 click 选择语义不动）。
// 落点判定：pointermove 时 document.elementFromPoint + 坐标——
//   a) 素材卡片间距排序：AssetGrid 注册的 gridResolver 按 boxes 定位间隙索引，
//      圆角竖向矩形色块填间距（--brand-1-12 填充 + inset 1px --brand-1，由 AssetGrid 渲染）；
//   b) 文件夹卡片并入：落 FolderCard → 覆盖缩略图区域的方形覆盖（FolderCard 自渲染）；
//   c) 文件树落点：item 高度三等分——上/下 1/3 = 排序插入线（仅文件夹拖拽生效，
//      素材拖到树 item 任何位置都算并入），中间 1/3 = 并入高亮；
//   d) Ctrl 拖拽：素材落文件夹（树 / 文件夹卡片）= 弹关联弹窗而非移动；落间距排序忽略 Ctrl。
// 混排护栏：FolderCard / 树文件夹拖拽只做同级排序（reorder_folders），素材只对素材排序；
// 文件夹并入（移动层级）后端无命令——视觉仍显示，松手 toast「暂不支持移动文件夹层级」。
// 拖拽中 Esc 取消；拖出窗口 / 松开无效区域 = 无操作。
import { reactive } from "vue";
import { useContentStore, type FolderNode } from "../stores/content";
import { useSelectionStore, type PlacementRef } from "../stores/selection";
import { useToastStore } from "../stores/toast";
import * as ops from "./placementOps";

export type DragKind = "asset" | "folder";

// 卡片间距落点（cards-layer 坐标系，AssetGrid 渲染色块；beforeKey = 间隙后项 key，null = 段尾）
export interface GapIndicator {
  left: number;
  top: number;
  width: number;
  height: number;
  beforeKey: string | null;
}

// 文件树落点（TreeItem 自渲染：before/after = 上/下沿 2px 插入线 + hover 底；into = 并入高亮）
export interface TreeIndicator {
  folderId: string;
  zone: "before" | "after" | "into";
}

// 拖拽可视态（组件只读）：指示器全部由被落方组件按 id 匹配自渲染，引擎不直接改 DOM
export const dragView = reactive({
  active: false,
  kind: "asset" as DragKind,
  ctrl: false,
  gap: null as GapIndicator | null, // 素材卡片间隙（AssetGrid 渲染）
  folderCardId: null as string | null, // 文件夹卡片并入覆盖（FolderCard 渲染）
  folderGapId: null as string | null, // 文件夹卡片间隙排序目标（FolderCard 渲染竖块）
  folderGapAfter: false,
  tree: null as TreeIndicator | null, // 文件树落点（TreeItem 渲染）
  suppressClick: false, // 完成一次有效拖拽后抑制紧随的 click（防拖完触发单选）
});

// 卡片间距解析器：AssetGrid 挂载时注册（它持有 boxes / 滚动 pos / scale），卸载注销
export type GridResolver = (clientX: number, clientY: number) => GapIndicator | null;
let gridResolver: GridResolver | null = null;
export function registerGridResolver(fn: GridResolver): void {
  gridResolver = fn;
}
export function unregisterGridResolver(): void {
  gridResolver = null;
}

// 间距解析器工厂（C4a；主体落 lib 控制 AssetGrid 行数 ≤500，规范第 3 条）：
// 按 pointer 在 boxes 中的阅读序定位间隙——行中点之前或同行左中点之前 = 插到该 box 前，
// 否则段尾；返回 cards-layer 坐标系的色块矩形（宽 = 卡片间距 16 / 高 = 行高）与间隙后项 key。
// 仅文件夹视图可排序（快速访问视图无"当前文件夹段"概念，不出间隙落点）
export function makeGridResolver(ctx: {
  isFolderView: () => boolean;
  layer: () => HTMLElement | undefined;
  scale: () => number;
  boxes: () => { top: number; left: number; width: number; height: number; i: number }[];
  keyAt: (i: number) => string;
}): GridResolver {
  const GAP = 16; // 卡片间距（layouts.ts COL_GAP / ROW_GAP，2026-07-23 用户定）
  return (clientX, clientY) => {
    if (!ctx.isFolderView()) return null;
    const layer = ctx.layer();
    if (!layer) return null;
    const r = layer.getBoundingClientRect();
    // 容差上下 24px（行间隙也能落）；超出卡片层横向范围不判
    if (clientX < r.left || clientX > r.right || clientY < r.top - 24 || clientY > r.bottom + 24) {
      return null;
    }
    const s = ctx.scale();
    const x = (clientX - r.left) / s;
    const y = (clientY - r.top) / s;
    const bs = ctx.boxes();
    let idx = -1;
    for (let i = 0; i < bs.length; i++) {
      const b = bs[i];
      if (y < b.top + b.height / 2 || (y < b.top + b.height && x < b.left + b.width / 2)) {
        idx = i;
        break;
      }
    }
    if (idx < 0) {
      const last = bs[bs.length - 1];
      if (!last) return null;
      return { left: last.left + last.width, top: last.top, width: GAP, height: last.height, beforeKey: null };
    }
    const b = bs[idx];
    const prev = bs[idx - 1];
    const sameRow = prev && prev.top === b.top;
    return {
      left: sameRow ? prev.left + prev.width : Math.max(0, b.left - GAP),
      top: b.top,
      width: GAP,
      height: b.height,
      beforeKey: ctx.keyAt(b.i),
    };
  };
}

const START_THRESHOLD = 6;

interface Session {
  kind: DragKind;
  key: string; // asset = `${assetId}|${folderId}`；folder = folderId
  startX: number;
  startY: number;
  started: boolean;
  items: PlacementRef[]; // asset 拖拽项（多选 = 全部选中项）
  folderId: string; // folder 拖拽项（单文件夹）
  ghost: HTMLElement | null;
  drop: (() => Promise<void>) | null; // 最后一次 move 算出的落点动作
}

let sess: Session | null = null;

// 树查找辅助（文件夹拖拽排序用：同级 parent / 下一兄弟）
function findParentAndNext(
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

function folderDragInto(folderId: string): void {
  // 文件夹并入 = 移动层级：后端无 move_folder（6.1 定案不做），视觉显示、松手提示
  void folderId;
  useToastStore().notify({ message: "暂不支持移动文件夹层级", icon: "info" });
}

function resetIndicators(): void {
  dragView.gap = null;
  dragView.folderCardId = null;
  dragView.folderGapId = null;
  dragView.tree = null;
  if (sess) sess.drop = null;
}

function makeGhost(anchor: HTMLElement | null): HTMLElement {
  const ghost = document.createElement("div");
  ghost.style.cssText =
    "position:fixed;z-index:3000;pointer-events:none;opacity:0.7;" +
    "border-radius:var(--radius-8);box-shadow:var(--shadow-pop);overflow:visible;max-width:96px;";
  const img = anchor?.querySelector("img");
  if (img?.src) {
    const g = document.createElement("img");
    g.src = img.src;
    g.draggable = false;
    g.style.cssText =
      "display:block;max-width:96px;max-height:96px;border-radius:var(--radius-8);" +
      "box-shadow:inset 0 0 0 1px var(--border-1);";
    ghost.appendChild(g);
  } else {
    // 无缩略图（格式图标卡片 / 文件夹）：名称纸片
    const chip = document.createElement("div");
    chip.textContent =
      anchor?.getAttribute("data-drag-name") ?? (sess?.kind === "folder" ? "文件夹" : "素材");
    chip.style.cssText =
      "padding:var(--space-6) var(--space-12);border-radius:var(--radius-8);background:var(--c4);" +
      "box-shadow:inset 0 0 0 1px var(--border-1);color:var(--font-icon-1);" +
      "font-size:var(--text-caption-size);line-height:var(--text-caption-line);" +
      "white-space:nowrap;max-width:160px;overflow:hidden;text-overflow:ellipsis;";
    ghost.appendChild(chip);
  }
  if (sess && sess.kind === "asset" && sess.items.length > 1) {
    const badge = document.createElement("span");
    badge.textContent = String(sess.items.length);
    badge.style.cssText =
      "position:absolute;top:-6px;right:-6px;min-width:18px;height:18px;border-radius:9px;" +
      "background:var(--brand-1);color:var(--font-icon-1);font-size:var(--text-caption-size);" +
      "line-height:18px;text-align:center;padding:0 var(--space-4);";
    ghost.appendChild(badge);
  }
  document.body.appendChild(ghost);
  return ghost;
}

function moveGhost(x: number, y: number): void {
  if (!sess?.ghost) return;
  sess.ghost.style.left = `${x}px`;
  sess.ghost.style.top = `${y}px`;
  sess.ghost.style.transform = "translate(-50%, -50%)";
}

// 落点判定（pointermove 实时）：树 > 文件夹卡片 > 卡片间隙；互不叠加
function updateDropTarget(e: PointerEvent): void {
  if (!sess) return;
  resetIndicators();
  dragView.ctrl = e.ctrlKey || e.metaKey;
  const el = document.elementFromPoint(e.clientX, e.clientY) as HTMLElement | null;
  if (!el) return;

  // 文件树 item（TreeItem 根带 data-folder-id；快速访问项无此属性天然不作落点）
  const treeItem = el.closest<HTMLElement>(".tree-item[data-folder-id]");
  if (treeItem) {
    const id = treeItem.getAttribute("data-folder-id")!;
    const r = treeItem.getBoundingClientRect();
    const rel = (e.clientY - r.top) / Math.max(1, r.height);
    if (sess.kind === "asset") {
      // 素材拖到树 item 任何位置都算并入（2026-07-28 定案）
      dragView.tree = { folderId: id, zone: "into" };
      sess.drop = async () => dropAssetsInto(id, e.ctrlKey || e.metaKey);
    } else {
      if (id === sess.folderId) return; // 拖到自身 = 无操作
      if (rel < 1 / 3 || rel > 2 / 3) {
        const zone = rel < 1 / 3 ? "before" : "after";
        dragView.tree = { folderId: id, zone };
        sess.drop = async () => dropFolderBeside(id, zone);
      } else {
        dragView.tree = { folderId: id, zone: "into" };
        sess.drop = async () => folderDragInto(id);
      }
    }
    return;
  }

  // 文件夹卡片（FolderCard 根带 data-folder-id）
  const folderCard = el.closest<HTMLElement>(".folder-card[data-folder-id]");
  if (folderCard) {
    const id = folderCard.getAttribute("data-folder-id")!;
    if (sess.kind === "asset") {
      dragView.folderCardId = id;
      sess.drop = async () => dropAssetsInto(id, e.ctrlKey || e.metaKey);
      return;
    }
    if (id !== sess.folderId) {
      const r = folderCard.getBoundingClientRect();
      const relX = (e.clientX - r.left) / Math.max(1, r.width);
      const siblings = sameStripSiblings(id);
      if (siblings && (relX < 0.3 || relX > 0.7)) {
        // 同级文件夹卡片间隙排序（混排护栏：文件夹只与文件夹排序）
        dragView.folderGapId = id;
        dragView.folderGapAfter = relX > 0.7;
        sess.drop = async () => dropFolderCardBeside(id, relX > 0.7);
      } else {
        dragView.folderCardId = id;
        sess.drop = async () => folderDragInto(id);
      }
    }
    return;
  }

  // 素材卡片间隙排序（仅素材拖拽；网格 resolver 由 AssetGrid 注册）
  if (sess.kind === "asset" && gridResolver) {
    const gap = gridResolver(e.clientX, e.clientY);
    if (gap) {
      dragView.gap = gap;
      sess.drop = async () => dropAssetsGap(gap);
    }
  }
}

// 文件夹卡片同级判定：目标卡片与拖拽文件夹须同在当前文件夹条带（混排护栏，同级排序）
function sameStripSiblings(targetId: string): boolean {
  if (!sess) return false;
  const content = useContentStore();
  const stripIds = new Set(content.stripFolders.map((f) => f.id));
  return stripIds.has(targetId) && stripIds.has(sess.folderId);
}

// 素材落文件夹（树 / 文件夹卡片）：Ctrl = 弹关联弹窗（C5）；否则移动
async function dropAssetsInto(folderId: string, ctrl: boolean): Promise<void> {
  if (!sess) return;
  const items = sess.items;
  if (items.length === 0) return;
  if (ctrl) {
    useSelectionStore().linkDraft = { items, targetFolder: folderId };
    return;
  }
  await ops.movePlacements(items, folderId);
}

// 素材落卡片间隙：reorder_placements（仅当前文件夹段内项参与；beforeItem = 间隙后项）
async function dropAssetsGap(gap: GapIndicator): Promise<void> {
  if (!sess) return;
  const content = useContentStore();
  const target = content.currentFolderId;
  if (target === null) return;
  const items = sess.items.filter((it) => it.folderId === target);
  if (items.length === 0) return;
  let beforeItem: PlacementRef | null = null;
  if (gap.beforeKey) {
    if (items.some((it) => `${it.assetId}|${it.folderId}` === gap.beforeKey)) return; // 插到自身前 = 无操作
    const p = content.visiblePlacements.find((v) => `${v.assetId}|${v.folderId}` === gap.beforeKey);
    if (p) beforeItem = { assetId: p.assetId, folderId: p.folderId, origin: p.origin };
  }
  await ops.reorderPlacements(items, target, beforeItem);
}

// 文件夹落树 item 上/下 1/3：reorder_folders 同级排序（before = 插到目标前 / after = 目标下一兄弟前）
async function dropFolderBeside(targetId: string, zone: "before" | "after"): Promise<void> {
  if (!sess) return;
  const content = useContentStore();
  const hit = findParentAndNext(content.folders, targetId);
  if (!hit) return;
  const dragged = findParentAndNext(content.folders, sess.folderId);
  if (!dragged || dragged.parentId !== hit.parentId) return; // 同级护栏
  await ops.reorderFolders([sess.folderId], hit.parentId, zone === "before" ? targetId : hit.nextId);
}

// 文件夹落文件夹卡片间隙：条带内同级排序（parent = 当前文件夹）
async function dropFolderCardBeside(targetId: string, after: boolean): Promise<void> {
  if (!sess) return;
  const content = useContentStore();
  const parentId = content.currentFolderId;
  const ids = content.stripFolders.map((f) => f.id);
  const i = ids.indexOf(targetId);
  if (i < 0) return;
  await ops.reorderFolders([sess.folderId], parentId, after ? (ids[i + 1] ?? null) : targetId);
}

function startDrag(e: PointerEvent): void {
  if (!sess || sess.started) return;
  sess.started = true;
  const selection = useSelectionStore();
  if (sess.kind === "asset") {
    // 拖拽未选中项 = 先单选它再拖（2026-07-28 定案）；多选拖拽 = 对全部选中项执行
    if (!selection.isSelected(sess.key)) selection.clickItem(sess.key);
    sess.items = selection.selectedRefs();
  }
  const anchor = document.elementFromPoint(sess.startX, sess.startY) as HTMLElement | null;
  sess.ghost = makeGhost(anchor);
  moveGhost(e.clientX, e.clientY);
  dragView.active = true;
  dragView.kind = sess.kind;
  document.body.style.userSelect = "none"; // 拖拽期间禁止文本选择
}

function onMove(e: PointerEvent): void {
  if (!sess) return;
  if (!sess.started) {
    if (Math.hypot(e.clientX - sess.startX, e.clientY - sess.startY) < START_THRESHOLD) return;
    startDrag(e);
  }
  moveGhost(e.clientX, e.clientY);
  updateDropTarget(e);
}

function endDrag(cancelled: boolean): void {
  const s = sess;
  if (!s) return;
  sess = null;
  window.removeEventListener("pointermove", onMove);
  window.removeEventListener("pointerup", onUp);
  window.removeEventListener("pointercancel", onCancel);
  window.removeEventListener("keydown", onKey, true);
  s.ghost?.remove();
  document.body.style.userSelect = "";
  const drop = s.drop;
  resetIndicators();
  dragView.active = false;
  dragView.ctrl = false;
  if (s.started && !cancelled && drop) {
    // 抑制紧随的 click（同步派发后才清，防拖完触发落点 / 源的单选切换）
    dragView.suppressClick = true;
    setTimeout(() => (dragView.suppressClick = false), 0);
    void drop();
  }
}

function onUp(e: PointerEvent): void {
  // 松手前按最后坐标 / Ctrl 态重算一次落点（松手瞬间才按 Ctrl 的边界，C4d）
  if (sess?.started) updateDropTarget(e);
  endDrag(false);
}
function onCancel(): void {
  endDrag(true);
}
function onKey(e: KeyboardEvent): void {
  if (e.key === "Escape") {
    e.stopPropagation();
    endDrag(true); // 拖拽中 Esc = 取消（2026-07-28 定案）
  }
}

function begin(e: PointerEvent, kind: DragKind, key: string): void {
  if (e.button !== 0 || sess) return;
  sess = {
    kind,
    key,
    startX: e.clientX,
    startY: e.clientY,
    started: false,
    items: [],
    folderId: kind === "folder" ? key : "",
    ghost: null,
    drop: null,
  };
  window.addEventListener("pointermove", onMove);
  window.addEventListener("pointerup", onUp);
  window.addEventListener("pointercancel", onCancel);
  window.addEventListener("keydown", onKey, true);
}

// 素材卡片拖拽起点（AssetCard / ListRowCard，卡片根带 data-key；按钮 / 输入框上不启动）
export function beginAssetDrag(e: PointerEvent, key: string): void {
  const t = e.target as HTMLElement;
  if (t.closest("button, input")) return;
  begin(e, "asset", key);
}

// 网格 pointerdown 分发（AssetGrid 模板一行接线，控制组件行数 ≤500，规范第 3 条）：
// 素材卡片 → 素材拖拽；文件夹卡片 → 文件夹拖拽；其余（空白）交回调（框选）
export function gridPointerDown(e: PointerEvent, onBlank: (e: PointerEvent) => void): void {
  const t = e.target as HTMLElement;
  const card = t.closest(".asset-card, .list-row");
  if (card) {
    const key = card.getAttribute("data-key");
    if (key) beginAssetDrag(e, key);
    return;
  }
  const folderCard = t.closest(".folder-card[data-folder-id]");
  if (folderCard) {
    beginFolderDrag(e, folderCard.getAttribute("data-folder-id")!);
    return;
  }
  onBlank(e);
}

// 文件夹拖拽起点（FolderCard / 文件树 TreeItem；树展开按钮上不启动）
export function beginFolderDrag(e: PointerEvent, folderId: string): void {
  const t = e.target as HTMLElement;
  if (t.closest(".expand")) return;
  begin(e, "folder", folderId);
}
