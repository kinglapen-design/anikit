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
//   ④ ~~信息区起框选~~（2026-08-18 用户定废除：card-info 退役——简介区有底色框后整卡可点
//      可选中、不作框选起点；卡片本体含名称 / 简介统一起拖拽）。
//   ⑤ 拖拽期间禁用悬停层：startDrag 置 body.dragging（CSS 闸）+ dragView.active（AssetCard JS 闸）。
//   ⑥ 双击名称变打开（2026-08-18 修复）：begin 即 pointer capture 会把双击第二击与 dblclick
//      重定向到卡片根元素，名称区 @dblclick.stop 收不到；capture 后移到 startDrag（拖拽全程仍有效）。
import { reactive } from "vue";
import { useContentStore, type PlacementView } from "../stores/content";
import { usePickerStore } from "../stores/picker";
import { useSelectionStore, type PlacementRef } from "../stores/selection";
import { useUiStore } from "../stores/ui";
import { isDisplaySorted } from "./sortPlacements";
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

// 卡片间距解析器：AssetGrid 挂载时注册（它持有 boxes / 滚动 pos），卸载注销。
// draggedKeys = 拖拽组 key 集（组约束判定用，2026-08-20 组内排序放开）；visibleKeys =
// 网格当前可见序（displayItems）key 序列——分组模式的物化可见序 = 组拼接序，非 placementsFor 原序
export type GridResolver = (clientX: number, clientY: number, draggedKeys: readonly string[]) => GapIndicator | null;
let gridResolver: GridResolver | null = null;
let gridVisibleKeys: (() => string[]) | null = null;
export function registerGridResolver(fn: GridResolver, visibleKeys?: () => string[]): void {
  gridResolver = fn;
  gridVisibleKeys = visibleKeys ?? null;
}
export function unregisterGridResolver(): void {
  gridResolver = null;
  gridVisibleKeys = null;
}

// 间距解析器工厂（C4a；主体落 lib 控制 AssetGrid 行数 ≤500，规范第 3 条）：
// 先定行再定列（2026-07-29 复验二轮 1）——行底 + 半行距之上入目标行，行内按 x 中点定间隙，
// 行内未命中 = 插到下一行行首（竖块画在指针所在行的行末，贴指针）；无目标行 = 段尾。
// 返回 cards-layer 坐标系的色块矩形（宽 = 卡片间距 12 / 高 = 行高）与间隙后项 key。
// 仅文件夹视图可排序（快速访问视图无"当前文件夹段"概念，不出间隙落点）。
// 2026-08-18 修复"第一列前插竖块盖住卡片"：原 Math.max(0, b.left - GAP) 钳位把行首间隙
// 拉回卡片左缘（盖住卡片 12px）——行首间隙 = 卡片左侧容器留白区，允许负 left（渲染侧
// 已 Teleport 到 body 按 client 坐标画，不再受 grid-viewport overflow 裁剪）
// 组约束（2026-08-20 用户定，替换 2026-08-19"分组模式整体禁排"）：ctx.groupOfKey 存在时
// 按组判定——同组内允许互相排序；跨分组（落点与拖拽组不同组、或多选跨组）禁止出落点，
// 因为组归属由规则定，拖过去也不改变归组，给落点是误导
export function makeGridResolver(ctx: {
  isFolderView: () => boolean;
  layer: () => HTMLElement | undefined;
  boxes: () => { top: number; left: number; width: number; height: number; i: number }[];
  keyAt: (i: number) => string;
  groupOfKey?: (key: string) => string | null | undefined; // 分组模式：key → 组 id（null = 未分组组）；非分组 = undefined
}): GridResolver {
  const GAP = COL_GAP; // 卡片间距（layouts.ts COL_GAP，figma 42 文件卡片列表 gap 12）
  return (clientX, clientY, draggedKeys) => {
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
    let gap: GapIndicator;
    if (idx < 0) {
      const last = bs[bs.length - 1];
      gap = { left: last.left + last.width, top: last.top, width: GAP, height: last.height, beforeKey: null };
    } else {
      const b = bs[idx];
      const prev = bs[idx - 1];
      if (rowBreak && prev) {
        // 行末间隙：竖块贴在指针所在行的末卡之后（beforeKey 仍是下一行行首项，语义不变）
        gap = {
          left: prev.left + prev.width,
          top: prev.top,
          width: GAP,
          height: prev.height,
          beforeKey: ctx.keyAt(b.i),
        };
      } else {
        const sameRow = prev && prev.top === b.top;
        gap = {
          left: sameRow ? prev.left + prev.width : b.left - GAP,
          top: b.top,
          width: GAP,
          height: b.height,
          beforeKey: ctx.keyAt(b.i),
        };
      }
    }
    // 组约束：分组模式下（groupOfKey 有定义）拖拽组与落点须同组
    if (ctx.groupOfKey && draggedKeys.length > 0) {
      const g0 = ctx.groupOfKey(draggedKeys[0]);
      if (g0 !== undefined) {
        for (const k of draggedKeys) {
          if (ctx.groupOfKey(k) !== g0) return null; // 多选跨组：禁止
        }
        // 落点组 = 间隙后项的组；段尾（beforeKey null）= 最末可见项的组
        const anchorKey = gap.beforeKey ?? ctx.keyAt(bs[bs.length - 1].i);
        if (ctx.groupOfKey(anchorKey) !== g0) return null;
      }
    }
    return gap;
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
  capEl: HTMLElement | null; // capture 候选根元素（按下的卡片 / 树 item 根，begin 时记）
  captured: boolean; // 是否已实际 capture（startDrag 才挂，2026-08-18——begin 即挂会把双击第二击重定向到卡片根）
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
    const gap = gridResolver(e.clientX, e.clientY, items.map((it) => `${it.assetId}|${it.folderId}`));
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
  const ui = useUiStore();
  if (isDisplaySorted(ui.sortBy)) {
    // 显示排序下拖拽排序（3.5，02 整理"物化 + 切回自定义"口径的素材侧延伸）：把该层可见序
    // （新增区置顶段 + 排序流）物化为存储序、拖拽组按落点并入，整层一次重写并切回自定义——
    // 否则 refresh 后排序键盖回，拖拽等于没执行。物化静默（不单独 toast，undo 快照仍在）。
    // 可见序以网格注册序为准（2026-08-20 组内排序放开：分组模式 = 组拼接序 displayItems，
    // 非 placementsFor 原序——物化组拼接序才能切回自定义后屏幕位置不变）；无注册序退回 placementsFor
    if (gap.beforeKey && items.some((it) => `${it.assetId}|${it.folderId}` === gap.beforeKey)) return; // 插到自身前 = 无操作
    const base = content.placementsFor(`folder:${target}`).filter((p) => p.folderId === target);
    let visible = base;
    const keys = gridVisibleKeys?.();
    if (keys) {
      const order = new Map(keys.map((k, i) => [k, i] as const));
      visible = [...base].sort(
        (a, b) =>
          (order.get(`${a.assetId}|${a.folderId}`) ?? Number.MAX_SAFE_INTEGER) -
          (order.get(`${b.assetId}|${b.folderId}`) ?? Number.MAX_SAFE_INTEGER),
      );
    }
    const inDrag = (p: PlacementView): boolean =>
      items.some((it) => it.assetId === p.assetId && it.folderId === p.folderId && it.origin === p.origin);
    const rest = visible.filter((p) => !inDrag(p));
    let insert = rest.length;
    if (gap.beforeKey) {
      const at = rest.findIndex((p) => `${p.assetId}|${p.folderId}` === gap.beforeKey);
      if (at >= 0) insert = at;
    }
    const ordered = visible.filter(inDrag); // 拖拽组保持可见序中的相对顺序
    const seq = [...rest.slice(0, insert), ...ordered, ...rest.slice(insert)];
    const ok = await ops.reorderPlacements(
      seq.map((p) => ({ assetId: p.assetId, folderId: p.folderId, origin: p.origin })),
      target,
      null, // 整层序列全传 + 段尾锚点 = 全层重写（reorder_folders 物化同模式）
    );
    if (ok) {
      ui.sortBy = "";
      content.clearArrivals(); // 可见序（含新增区位置）已物化进存储序，新增区概念由自定义序承接
    }
    return;
  }
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
  // pointer capture 在拖拽真正启动时才挂（2026-08-18 修复"双击名称变打开"：begin 即 capture 会把
  // 双击序列的第二击与 dblclick 重定向到卡片根元素，名称区 @dblclick.stop 永远收不到——
  // click 仍派发到卡片根所以单击无碍，dblclick 被子元素 .stop 的链路被 capture 截断）。
  // 拖到窗外松手 pointerup 仍能派发（capture 拖拽全程有效）；按下即划出窗口的窗口期由
  // begin 先取消旧会话 + onMove buttons 自清 + docLeave 划出取消三兜底
  if (sess.capEl) {
    try {
      sess.capEl.setPointerCapture(sess.pointerId);
      sess.captured = true;
    } catch {
      /* capture 失败靠 onMove buttons 自清兜底 */
    }
  }
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
  if (s.capEl && s.captured) {
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
// 未 capture 阶段（按下未起拖）划出窗口即取消——窗外松手 pointerup 丢失、onMove 不再触发，
// buttons 僵尸自清失去兜底机会，落点指示/悬挂会话残留；capture 生效后（拖拽中）划出不动
//（2026-08-14 走查"品牌色像素点残留"；2026-08-18 随 capture 后移到 startDrag 改恒挂 + captured 判定）
function onDocLeave(): void {
  if (sess?.captured) return;
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
    captured: false,
  };
  // capture 候选根元素只记不挂（capture 后移到 startDrag，2026-08-18——begin 即挂会把
  // 双击序列第二击与 dblclick 重定向到卡片根，名称区双击改名被吞成"打开"，见 startDrag 注释）；
  // 僵尸窗口期（按下未起拖就划出/窗外松手）靠三道兜底：begin 先取消旧会话 + onMove buttons
  // 自清 + docLeave 划出取消（恒挂）
  sess.capEl = (e.target as HTMLElement).closest<HTMLElement>(
    ".tree-item, .asset-card, .list-row, .folder-card, .archive-card, .block-head",
  );
  window.addEventListener("pointermove", onMove);
  window.addEventListener("pointerup", onUp);
  window.addEventListener("pointercancel", onCancel);
  window.addEventListener("keydown", onKey, true);
  window.addEventListener("blur", onWindowBlur);
  document.addEventListener("mouseleave", onDocLeave);
}

// 素材卡片拖拽起点（AssetCard / ListRowCard，卡片根带 data-key；按钮 / 输入框上不启动）
export function beginAssetDrag(e: PointerEvent, key: string): void {
  const t = e.target as HTMLElement;
  if (t.closest("button, input")) return;
  begin(e, "asset", key);
}

// 网格 pointerdown 分发（AssetGrid 模板一行接线，控制组件行数 ≤500，规范第 3 条）：
// 素材卡片（含名称 / 简介区，整卡）→ 素材拖拽；文件夹卡片 → 文件夹拖拽；其余（空白，
// 含卡片行 / 列间距）交回调（框选）。简介区 2026-08-18 起不再视为空白（card-info 退役）：
// 名称+简介有底色框后用户预期整卡可点可选中、且不作框选起点（用户定）
export function gridPointerDown(e: PointerEvent, onBlank: (e: PointerEvent) => void): void {
  const t = e.target as HTMLElement;
  // 重命名等输入态中点击网格任意处（卡片 / 空白）= 确认提交（2026-08-10 走查：框选回调的
  // preventDefault 会阻断焦点转移、input 永不 blur——统一在分发入口显式 blur 活跃输入框，
  // 其 @blur 提交逻辑随之触发；重命名 input 自身 pointerdown 已 stop 冒泡不会到此）
  const ae = document.activeElement;
  if (ae instanceof HTMLInputElement || ae instanceof HTMLTextAreaElement) ae.blur();
  // 选择模式（6.2 一轮，PRD 03_数据链接 04：拖拽禁用）——卡片按下不启动手势（同回收站守卫口径：
  // 交框选会吞 click 合成）；框选同为选择操作本模式无意义。单击加入备选走 click 链不受影响
  if (usePickerStore().active) return;
  const card = t.closest(".asset-card, .list-row");
  if (card) {
    // 回收站视口（6.1 二轮，2026-07-29）：卡片只读不可拖拽——按下不启动任何手势直接放行：
    // 原实现交空白回调（框选），其 preventDefault 会吞掉 click 合成、未过阈值松开又按
    //"点击空白"清空选择——回收站卡片单击选中 / 双击预览全灭（2026-08-21 用户实机报修）。
    // 框选不受影响：空白处按下照常起框选（onBlank 只在非卡片命中时调用）
    if (useContentStore().isRecycleView) return;
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
  if (usePickerStore().active) return; // 选择模式（6.2）：拖拽禁用（树 item 直调本函数，双保险）
  const t = e.target as HTMLElement;
  if (t.closest(".expand")) return;
  begin(e, "folder", folderId);
}
