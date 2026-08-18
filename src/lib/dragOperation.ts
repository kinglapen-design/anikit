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
// 混排护栏：FolderCard / 树文件夹拖拽只与文件夹排序，素材只对素材排序；
// 树上/下 1/3：整组同父 = reorder_folders 同级排序，跨父级 = move_folders 带 beforeFolderId
// 定位插入（2026-07-29 复验：嵌套文件夹拖出）；树中间 1/3 / 文件夹卡片并入 = move_folders 移动层级
//（2026-07-28 复验反馈 5 接入，替换原"暂不支持"toast）。
// 拖拽中 Esc 取消；拖出窗口 / 松开无效区域 = 无操作。
//
// 2026-07-28 复验反馈修复（根因记录）：
//   ① 落点动作闭包统一从全局 sess 取数，而 endDrag 先 sess = null 再 void drop()——drop 函数
//      首行 if (!sess) return 恒命中，全部落点"只有动画没有实际事件执行"（复验反馈 1）；
//      改为闭包在 pointermove 算出落点时即捕获所需入参，drop 函数不再读 sess。
//   ② 间隙 resolver 用 layer.getBoundingClientRect() 判容差，而 cards-layer 子卡片全绝对定位、
//      层高度塌缩为 0，rect 只剩一条线 → 容差恒失败、间隙永不命中（复验反馈 2）；
//      容差范围改按 boxes 包围盒计算（AssetGrid 同步给层补显式高度，复验反馈 3 同因）。
//
// 2026-07-29 复验二轮修复（根因记录）：
//   ① 间隙指示与鼠标错位（鼠标在右侧卡片、竖块在左侧远处）——原间隙判定
//      y < b.top + b.height/2 对同行所有 box 恒真，循环在阅读序首个同排 box 即 break，
//      指针在行上半部时恒插到行首；修为"先定行再定列"（makeGridResolver 内详注）。
//   ② 树多选排序只动被拖一项——根因在 FolderTree：树 plain click 只导航不入文件夹选择域
//     （sidebarSelected 高亮冒充选中态），拖"看着选中实则不在域内"的项时 startDrag 按
//      "未选中 = 单选再拖"把多选清成单选；修为树 plain click 同步单选入域（FolderTree）。
//   ③ ghost 无缩略图——elementFromPoint 命中最深元素（常是 img 自身 / 信息区文字 span），
//      dragGhost 只 querySelector 后代恒 miss 走纸片 fallback；修为 startDrag 先 closest
//      到卡片根、多选取首个选中项卡片，dragGhost 读 currentSrc。
//   ④ 信息区起框选（对齐 Eagle）：.card-info 在 gridPointerDown 视为空白交框选回调。
//   ⑤ 拖拽期间禁用悬停层：startDrag 置 body.dragging（CSS 闸）+ dragView.active（AssetCard JS 闸）。
import { reactive } from "vue";
import { useContentStore } from "../stores/content";
import { useSelectionStore, type PlacementRef } from "../stores/selection";
import { COL_GAP } from "./layouts";
import * as ops from "./placementOps";
import { makeGhost, moveGhost } from "./dragGhost";
import {
  dropFolderBeside,
  dropFolderCardBeside,
  dropFoldersInto,
  dropFoldersIntoBlock,
  isArchiveFolder,
  visibleTreeFolderIds,
} from "./dragFolderOps";

// 自 dragFolderOps 转出（规范第 3 条行数控制，2026-07-29）；re-export 保既有引用点不动
export { visibleTreeFolderIds };

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
  // 区块标题落点（2026-08-14 区块子系统，LibraryView .block-head 渲染）：文件夹拖入该区块根级
  //（并入高亮）。区块标题互拖排序不走引擎——改 LibraryView 本地让位式拖拽（库菜单同款，
  // 区块段高不定，间隙/插入线模型不适配）
  block: null as { id: string; zone: "into" } | null,
  // 拖拽源项（2026-07-28 复验反馈 6①：源卡片 / 源项原地半透明 var(--opacity-drag-source)，结束恢复）
  assetKeys: null as Set<string> | null, // 素材拖拽源 key 集（AssetCard / ListRowCard 读）
  folderIds: null as Set<string> | null, // 文件夹拖拽源 id 集（FolderCard / TreeItem 读）
  suppressClick: false, // 完成一次有效拖拽后抑制紧随的 click（防拖完触发单选）
});

// 素材卡片透明度态（2026-07-28 复验反馈 6①/8）：拖拽源 = source（--opacity-drag-source）/
// 被剪切 = cut（--opacity-cut，粘贴 / 覆盖 / 清空后 clipboard 变化自动恢复）
export function assetCardFade(key: string): "source" | "cut" | null {
  if (dragView.assetKeys?.has(key)) return "source";
  return useSelectionStore().isCut(key) ? "cut" : null;
}

// 卡片间距解析器：AssetGrid 挂载时注册（它持有 boxes / 滚动 pos），卸载注销
export type GridResolver = (clientX: number, clientY: number) => GapIndicator | null;
let gridResolver: GridResolver | null = null;
export function registerGridResolver(fn: GridResolver): void {
  gridResolver = fn;
}
export function unregisterGridResolver(): void {
  gridResolver = null;
}

// 间距解析器工厂（C4a；主体落 lib 控制 AssetGrid 行数 ≤500，规范第 3 条）：
// 先定行再定列（2026-07-29 复验二轮 1）——行底 + 半行距之上入目标行，行内按 x 中点定间隙，
// 行内未命中 = 插到下一行行首（竖块画在指针所在行的行末，贴指针）；无目标行 = 段尾。
// 返回 cards-layer 坐标系的色块矩形（宽 = 卡片间距 12 / 高 = 行高）与间隙后项 key。
// 仅文件夹视图可排序（快速访问视图无"当前文件夹段"概念，不出间隙落点）
export function makeGridResolver(ctx: {
  isFolderView: () => boolean;
  layer: () => HTMLElement | undefined;
  boxes: () => { top: number; left: number; width: number; height: number; i: number }[];
  keyAt: (i: number) => string;
}): GridResolver {
  const GAP = COL_GAP; // 卡片间距（layouts.ts COL_GAP，figma 42 文件卡片列表 gap 12）
  return (clientX, clientY) => {
    if (!ctx.isFolderView()) return null;
    const layer = ctx.layer();
    if (!layer) return null;
    const r = layer.getBoundingClientRect();
    const bs = ctx.boxes();
    if (bs.length === 0) return null;
    // 容差范围按 boxes 包围盒（上下 24px，行间隙也能落；横向取层宽）——不读 rect.bottom：
    // 层高度塌缩时它不可信（根因②，见文件头）
    let maxBottom = 0;
    for (const b of bs) maxBottom = Math.max(maxBottom, b.top + b.height);
    if (clientX < r.left || clientX > r.right || clientY < r.top - 24 || clientY > r.top + maxBottom + 24) {
      return null;
    }
    const x = clientX - r.left;
    const y = clientY - r.top;
    // 2026-07-29 复验二轮 1 根因：原判定 y < b.top + b.height/2 对同行所有 box 恒真，
    // 循环在阅读序首个同排 box 即 break——指针在行上半部时指示恒落行首（鼠标在右侧、
    // 竖块在左侧远处）。修为两段：先定行（底 + 半行距），行内按 x 中点定间隙
    let idx = -1;
    let rowBreak = false; // 行内 x 未命中 = 插到下一行行首（竖块画本行行末，贴指针）
    let rowTop = Number.NaN;
    for (let i = 0; i < bs.length; i++) {
      const b = bs[i];
      if (Number.isNaN(rowTop)) {
        if (y >= b.top + b.height + GAP / 2) continue;
        rowTop = b.top;
      }
      if (b.top !== rowTop) {
        idx = i;
        rowBreak = true;
        break;
      }
      if (x < b.left + b.width / 2) {
        idx = i;
        break;
      }
    }
    if (idx < 0) {
      const last = bs[bs.length - 1];
      return { left: last.left + last.width, top: last.top, width: GAP, height: last.height, beforeKey: null };
    }
    const b = bs[idx];
    const prev = bs[idx - 1];
    if (rowBreak && prev) {
      // 行末间隙：竖块贴在指针所在行的末卡之后（beforeKey 仍是下一行行首项，语义不变）
      return {
        left: prev.left + prev.width,
        top: prev.top,
        width: GAP,
        height: prev.height,
        beforeKey: ctx.keyAt(b.i),
      };
    }
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
  key: string; // asset = `${assetId}|${folderId}`；folder = 按下的 folderId
  startX: number;
  startY: number;
  started: boolean;
  items: PlacementRef[]; // asset 拖拽项（多选 = 全部选中项）
  folderIds: string[]; // folder 拖拽组（多选 = 全部选中文件夹，2026-07-28 复验反馈 5）
  ghost: HTMLElement | null;
  drop: (() => Promise<void>) | null; // 最后一次 move 算出的落点动作（入参已捕获，不读 sess）
  pointerId: number; // pointer capture 用（2026-08-13 僵尸会话修复）
  capEl: HTMLElement | null; // capture 元素（按下的卡片 / 树 item 根），endDrag 释放
}

let sess: Session | null = null;

function resetIndicators(): void {
  dragView.gap = null;
  dragView.folderCardId = null;
  dragView.folderGapId = null;
  dragView.tree = null;
  dragView.block = null;
  if (sess) sess.drop = null;
}

// 落点判定（pointermove 实时）：树 > 文件夹卡片 > 卡片间隙；互不叠加
function updateDropTarget(e: PointerEvent): void {
  if (!sess) return;
  resetIndicators();
  dragView.ctrl = e.ctrlKey || e.metaKey;
  const el = document.elementFromPoint(e.clientX, e.clientY) as HTMLElement | null;
  if (!el) return;
  const items = sess.items;
  const folderIds = sess.folderIds;
  const ctrl = e.ctrlKey || e.metaKey;

  // 文件树 item（TreeItem 根带 data-folder-id；快速访问项无此属性天然不作落点）
  const treeItem = el.closest<HTMLElement>(".tree-item[data-folder-id]");
  if (treeItem) {
    const id = treeItem.getAttribute("data-folder-id")!;
    const r = treeItem.getBoundingClientRect();
    const rel = (e.clientY - r.top) / Math.max(1, r.height);
    if (sess.kind === "asset") {
      // 素材拖到树 item 任何位置都算并入（2026-07-28 定案）
      dragView.tree = { folderId: id, zone: "into" };
      sess.drop = async () => dropAssetsInto(items, id, ctrl);
    } else {
      if (folderIds.includes(id)) return; // 拖到自身组 = 无操作
      // 档案节点禁止并入（2026-08-15 档案子系统一轮：档案下不允许子级文件夹，后端护栏）——
      // 中间 1/3 并入落点降级为排序落点（按半高定前/后）；素材拖入档案不受限（档案可含文件）
      if (!isArchiveFolder(id) && rel >= 1 / 3 && rel <= 2 / 3) {
        dragView.tree = { folderId: id, zone: "into" };
        sess.drop = async () => dropFoldersInto(folderIds, id);
      } else {
        const zone = rel < 0.5 ? "before" : "after";
        dragView.tree = { folderId: id, zone };
        sess.drop = async () => dropFolderBeside(folderIds, id, zone);
      }
    }
    return;
  }

  // 文件夹卡片 / 档案卡片（根带 data-folder-id）；档案卡片不作文件夹并入落点（同树档案护栏）
  const folderCard = el.closest<HTMLElement>(".folder-card[data-folder-id], .archive-card[data-folder-id]");
  if (folderCard) {
    const id = folderCard.getAttribute("data-folder-id")!;
    if (sess.kind === "asset") {
      dragView.folderCardId = id;
      sess.drop = async () => dropAssetsInto(items, id, ctrl);
      return;
    }
    if (!folderIds.includes(id) && !isArchiveFolder(id)) {
      const r = folderCard.getBoundingClientRect();
      const relX = (e.clientX - r.left) / Math.max(1, r.width);
      if (sameStripSiblings(id) && (relX < 0.3 || relX > 0.7)) {
        // 同级文件夹卡片间隙排序（混排护栏：文件夹只与文件夹排序）
        dragView.folderGapId = id;
        dragView.folderGapAfter = relX > 0.7;
        sess.drop = async () => dropFolderCardBeside(folderIds, id, relX > 0.7);
      } else {
        dragView.folderCardId = id;
        sess.drop = async () => dropFoldersInto(folderIds, id);
      }
    }
    return;
  }

  // 文件夹拖到区块标题（2026-08-14 区块子系统）：移入该区块根级（move_folders targetBlock），
  // 并入高亮同树 item 中间 1/3 口径；素材拖区块标题无意义（素材落点 = 文件夹），不作落点
  if (sess.kind === "folder") {
    const head = el.closest<HTMLElement>(".block-head[data-block-id]");
    if (head) {
      const id = head.getAttribute("data-block-id")!;
      dragView.block = { id, zone: "into" };
      sess.drop = async () => dropFoldersIntoBlock(folderIds, id);
      return;
    }
  }

  // 素材卡片间隙排序（仅素材拖拽；网格 resolver 由 AssetGrid 注册）
  if (sess.kind === "asset" && gridResolver) {
    const gap = gridResolver(e.clientX, e.clientY);
    if (gap) {
      dragView.gap = gap;
      sess.drop = async () => dropAssetsGap(items, gap);
    }
  }
}

// 文件夹卡片同级判定：目标卡片在当前文件夹条带、且拖拽组有成员在条带（混排护栏，同级排序）
function sameStripSiblings(targetId: string): boolean {
  if (!sess) return false;
  const stripIds = new Set(useContentStore().stripFolders.map((f) => f.id));
  return stripIds.has(targetId) && sess.folderIds.some((id) => stripIds.has(id));
}

// 素材落文件夹（树 / 文件夹卡片）：Ctrl = 直接建链接不弹窗（2026-08-18 用户定，与 Ctrl+V 同语义，
// 原"弹关联弹窗"废除——高频整理打扰）；否则移动
async function dropAssetsInto(items: PlacementRef[], folderId: string, ctrl: boolean): Promise<void> {
  if (items.length === 0) return;
  if (ctrl) {
    await ops.linkPlacements(items, folderId, items.map(() => null));
    return;
  }
  await ops.movePlacements(items, folderId);
}

// 素材落卡片间隙：reorder_placements（仅当前文件夹段内项参与；beforeItem = 间隙后项）
async function dropAssetsGap(dragged: PlacementRef[], gap: GapIndicator): Promise<void> {
  const content = useContentStore();
  const target = content.currentFolderId;
  if (target === null) return;
  const items = dragged.filter((it) => it.folderId === target);
  if (items.length === 0) return;
  let beforeItem: PlacementRef | null = null;
  if (gap.beforeKey) {
    if (items.some((it) => `${it.assetId}|${it.folderId}` === gap.beforeKey)) return; // 插到自身前 = 无操作
    const p = content.visiblePlacements.find((v) => `${v.assetId}|${v.folderId}` === gap.beforeKey);
    if (p) beforeItem = { assetId: p.assetId, folderId: p.folderId, origin: p.origin };
  }
  await ops.reorderPlacements(items, target, beforeItem);
}

function startDrag(e: PointerEvent): void {
  if (!sess || sess.started) return;
  sess.started = true;
  const selection = useSelectionStore();
  if (sess.kind === "asset") {
    // 拖拽未选中项 = 先单选它再拖（2026-07-28 定案）；多选拖拽 = 对全部选中项执行
    if (!selection.isSelected(sess.key)) selection.clickItem(sess.key);
    sess.items = selection.selectedRefs();
    dragView.assetKeys = new Set(selection.selected);
  } else if (sess.kind === "folder") {
    // 文件夹整组拖拽（2026-07-28 复验反馈 5）：拖拽未选中文件夹 = 先单选它再拖；
    // 多选拖拽 = 对全部选中文件夹执行（顺序按树可见展开态，2026-08-14 多区块：按源区块容器收窄）
    if (!selection.isFolderSelected(sess.key)) selection.clickFolder(sess.key);
    const srcScope =
      (document.elementFromPoint(sess.startX, sess.startY) as HTMLElement | null)?.closest(".block-tree") ?? null;
    sess.folderIds = selection.orderedSelectedFolderIds(visibleTreeFolderIds(srcScope));
    dragView.folderIds = new Set(sess.folderIds);
  }
  // ghost 取图锚点（2026-07-29 复验二轮 3）：elementFromPoint 命中最深元素（常是 img 自身 /
  // 信息区文字 span），原 dragGhost 只 querySelector 后代恒 miss 走纸片 fallback——
  // 先 closest 到卡片根；多选取首个选中项的卡片（卡片已渲染缩略图涵盖 PSD 缓存图 / 波形 /
  // 视频首帧 / 序列帧封面全部形态，最可靠）
  const hit = document.elementFromPoint(sess.startX, sess.startY) as HTMLElement | null;
  let anchor = hit?.closest<HTMLElement>(".asset-card, .list-row, .folder-card, .archive-card, .tree-item") ?? null;
  if (sess.kind === "asset" && sess.items.length > 0) {
    const k0 = `${sess.items[0].assetId}|${sess.items[0].folderId}`;
    anchor =
      document.querySelector<HTMLElement>(
        `.asset-card[data-key="${CSS.escape(k0)}"], .list-row[data-key="${CSS.escape(k0)}"]`,
      ) ?? anchor;
  }
  sess.ghost = makeGhost({
    anchor,
    kind: sess.kind,
    count: sess.kind === "asset" ? sess.items.length : sess.folderIds.length,
  });
  moveGhost(sess.ghost, e.clientX, e.clientY);
  dragView.active = true;
  dragView.kind = sess.kind;
  document.body.style.userSelect = "none"; // 拖拽期间禁止文本选择
  document.body.classList.add("dragging"); // 拖拽期间禁用悬停控件（2026-07-29 复验二轮 5，CSS 闸）
}

function onMove(e: PointerEvent): void {
  if (!sess) return;
  // 僵尸会话自清（2026-08-13 用户实机"点不动 / 双选中"根因）：指针已非按下态（buttons 无左键）
  // 说明 pointerup 曾丢失（窗口外松手等）——立即取消旧会话，否则旧 sess 会被后续按下静默复用，
  // 移动超阈值误 startDrag：吞 click（点不动、双击才行）+ clickFolder 误入域（双选中残留）+ 误执行 drop
  if ((e.buttons & 1) === 0) {
    endDrag(true);
    return;
  }
  if (!sess.started) {
    if (Math.hypot(e.clientX - sess.startX, e.clientY - sess.startY) < START_THRESHOLD) return;
    startDrag(e);
  }
  if (sess.ghost) moveGhost(sess.ghost, e.clientX, e.clientY);
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
  window.removeEventListener("blur", onWindowBlur);
  document.removeEventListener("mouseleave", onDocLeave);
  // 释放 pointer capture（元素已卸载等情况 capture 已自动释放，吞异常）
  if (s.capEl) {
    try {
      s.capEl.releasePointerCapture(s.pointerId);
    } catch {
      /* 已自动释放 */
    }
  }
  s.ghost?.remove();
  document.body.style.userSelect = "";
  document.body.classList.remove("dragging");
  const drop = s.drop;
  resetIndicators();
  dragView.active = false;
  dragView.ctrl = false;
  dragView.assetKeys = null;
  dragView.folderIds = null;
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
// 窗口失焦兜底（2026-08-10 用户实测：系统截图 / 分屏 snap 抢指针，pointerup·cancel 丢失，
// 拖拽 ghost 与落点指示会残留——失焦即取消，与框选 forceCancel 同根同治）
function onWindowBlur(): void {
  endDrag(true);
}
// 未挂 capture 的拖拽（区块标题，2026-08-14 走查"品牌色像素点残留"）：指针划出窗口即取消——
// 窗外松手 pointerup 丢失、onMove 不再触发，buttons 僵尸自清失去兜底机会，落点指示残留
function onDocLeave(): void {
  endDrag(true);
}
function onKey(e: KeyboardEvent): void {
  if (e.key === "Escape") {
    e.stopPropagation();
    endDrag(true); // 拖拽中 Esc = 取消（2026-07-28 定案）
  }
}

function begin(e: PointerEvent, kind: DragKind, key: string): void {
  if (e.button !== 0) return;
  // 僵尸会话不复用（2026-08-13 修复第二道防线，原 `if (sess) return` 静默复用是卡死帮凶）：
  // 新左键按下时旧会话必然已残缺（其 pointerup 丢失），先取消再建新
  if (sess) endDrag(true);
  sess = {
    kind,
    key,
    startX: e.clientX,
    startY: e.clientY,
    started: false,
    items: [],
    folderIds: [],
    ghost: null,
    drop: null,
    pointerId: e.pointerId,
    capEl: null,
  };
  // pointer capture 挂按下的卡片 / 树 item 根（治本：窗口外松手 pointerup 也能派发到 webview，
  // 僵尸会话不再产生；capture 到自身根元素不影响 click 派发——down/up 共同祖先仍是他）。
  // 区块标题排序不走引擎（LibraryView 本地让位式拖拽），引擎内全部拖类均挂 capture；
  // 未挂上的边缘情况靠 onMove buttons 僵尸自清 + document mouseleave 划出取消双兜底
  const capEl = (e.target as HTMLElement).closest<HTMLElement>(
    ".tree-item, .asset-card, .list-row, .folder-card, .archive-card, .block-head",
  );
  if (capEl) {
    try {
      capEl.setPointerCapture(e.pointerId);
      sess.capEl = capEl;
    } catch {
      /* capture 失败靠 onMove buttons 自清兜底 */
    }
  }
  window.addEventListener("pointermove", onMove);
  window.addEventListener("pointerup", onUp);
  window.addEventListener("pointercancel", onCancel);
  window.addEventListener("keydown", onKey, true);
  window.addEventListener("blur", onWindowBlur);
  // 未挂 capture 的拖拽（区块标题）：划出窗口即取消兜底（窗外松手 pointerup 丢失 → 指示残留）
  if (!sess.capEl) {
    document.addEventListener("mouseleave", onDocLeave);
  }
}

// 素材卡片拖拽起点（AssetCard / ListRowCard，卡片根带 data-key；按钮 / 输入框上不启动）
export function beginAssetDrag(e: PointerEvent, key: string): void {
  const t = e.target as HTMLElement;
  if (t.closest("button, input")) return;
  begin(e, "asset", key);
}

// 网格 pointerdown 分发（AssetGrid 模板一行接线，控制组件行数 ≤500，规范第 3 条）：
// 素材卡片 → 素材拖拽；文件夹卡片 → 文件夹拖拽；其余（空白，含卡片行 / 列间距）交回调（框选）
export function gridPointerDown(e: PointerEvent, onBlank: (e: PointerEvent) => void): void {
  const t = e.target as HTMLElement;
  // 重命名等输入态中点击网格任意处（卡片 / 空白）= 确认提交（2026-08-10 走查：框选回调的
  // preventDefault 会阻断焦点转移、input 永不 blur——统一在分发入口显式 blur 活跃输入框，
  // 其 @blur 提交逻辑随之触发；重命名 input 自身 pointerdown 已 stop 冒泡不会到此）
  const ae = document.activeElement;
  if (ae instanceof HTMLInputElement || ae instanceof HTMLTextAreaElement) ae.blur();
  // 卡片信息文字区（2026-07-29 复验二轮 4，对齐 Eagle）：.card-info 视为空白——
  // 按下拖动 = 起框选，未过阈值松开 = 清空选择；左键单击不选中卡片（刻意，
  // AssetGrid onCardClick 同拦；右键菜单 / 双击预览 / F2 重命名不受影响）。
  // 2026-07-29 复验三轮：名称（.title / .name）退出 card-info 回卡片本体分支
  //（可起拖拽 / 单击选中 / 双击改名），card-info 仅剩 meta 尺寸行 / .details
  if (t.closest(".card-info")) {
    onBlank(e);
    return;
  }
  const card = t.closest(".asset-card, .list-row");
  if (card) {
    // 回收站视口（6.1 二轮，2026-07-29）：卡片只读不可拖拽，按下交空白回调（框选 / 清空选择）
    if (useContentStore().isRecycleView) {
      onBlank(e);
      return;
    }
    const key = card.getAttribute("data-key");
    if (key) beginAssetDrag(e, key);
    return;
  }
  const folderCard = t.closest(".folder-card[data-folder-id], .archive-card[data-folder-id]");
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
