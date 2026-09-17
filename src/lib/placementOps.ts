// 素材基础操作命令封装（6.1 阶段 C，2026-07-28 用户定案）：UI 唯一调用入口。
// 每个写操作统一三段：invoke 命令 → content.refresh() 全量对齐（盘上是事实来源）→
// 操作反馈 toast（C7：成功带「撤销」动作按钮，文案与后端 undo 标签一致；失败中文人话 info 条）。
// 后端命令见 src-tauri/src/commands/operations.rs（全部已接撤销）。
// 2026-07-28 复验反馈修复：toast 计数文案改惰性求值（label 在 action 完成后才拼——
// 原先把模板串在 invoke 前求值，移动 / 关联 / 副本恒显示"0 个"，复验误判"没有实际事件执行"）；
// 新增 move_folders 入口（第 9 条命令，文件夹移动层级，替换原"暂不支持移动文件夹层级"toast）。
// 2026-07-29 二轮：回收站操作入口（restore_recycle 带撤销；delete_recycle / clear_recycle
// 两段式确认、不可恢复无撤销；后端契约并行开发，UI 直接 invoke）。
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useContentStore } from "../stores/content";
import { useSelectionStore, type PlacementRef } from "../stores/selection";
import { useToastStore } from "../stores/toast";
import { useUndoStore } from "../stores/undo";
import { treeExpansion } from "./treeExpansion";
import { DEFAULT_COLOR } from "./customColors";

export type { PlacementRef };

// 写操作统一收尾：刷新 + 成功 toast（带撤销）；业务失败 = 中文字符串，info 条原样展示。
// label 惰性求值：action 完成后才拼文案，计数类（移动 N 个）拿到真实返回值
async function runOp(action: () => Promise<void>, label: () => string): Promise<boolean> {
  const content = useContentStore();
  const toast = useToastStore();
  const undo = useUndoStore();
  try {
    await action();
    await content.refresh();
    void undo.refreshState();
    toast.notify({
      message: `已${label()}`,
      actionLabel: "撤销",
      onAction: () => void undo.undo(),
    });
    return true;
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
    return false;
  }
}

export async function movePlacements(items: PlacementRef[], targetFolder: string): Promise<boolean> {
  let moved = 0;
  return await runOp(async () => {
    moved = await invoke<number>("move_placements", { items, targetFolder });
  }, () => `移动 ${moved} 个项目`);
}

// 段内排序：items 须同属 targetFolder；beforeItem = null 排到段尾
export async function reorderPlacements(
  items: PlacementRef[],
  targetFolder: string,
  beforeItem: PlacementRef | null,
): Promise<boolean> {
  return await runOp(async () => {
    await invoke("reorder_placements", { items, targetFolder, beforeItem });
  }, () => "调整顺序");
}

// 文件夹同级排序（混排护栏：与素材排序各自独立，2026-07-28 定案）
export async function reorderFolders(
  folderIds: string[],
  parentId: string | null,
  beforeFolderId: string | null,
): Promise<boolean> {
  return await runOp(async () => {
    await invoke("reorder_folders", { folderIds, parentId, beforeFolderId });
  }, () => "调整文件夹顺序");
}

// 移动文件夹层级（2026-07-28 复验反馈 5）：targetParent null 或 "root" = 顶层；
// beforeFolderId 给了 = 插到该兄弟（targetParent 直接子级）之前——跨父级上/下 1/3 落点定位
// （2026-07-29 复验：嵌套文件夹拖出），null = 同级末尾；
// targetBlock（2026-08-14 区块子系统）三态：null = 保持原区块；落根级时给区块 id 写入归属
// （"dir" = 移回「目录」）；落进文件夹内（targetParent 非空）必须不传（后端拒绝）；
// 循环护栏与同父跳过在后端（撤销标签「移动 N 个文件夹」）；树 item 中间 1/3 与文件夹卡片并入走此
export async function moveFolders(
  folderIds: string[],
  targetParent: string | null,
  beforeFolderId: string | null = null,
  targetBlock: string | null = null,
): Promise<boolean> {
  let moved = 0;
  const ok = await runOp(async () => {
    moved = await invoke<number>("move_folders", { folderIds, targetParent, beforeFolderId, targetBlock });
  }, () => `移动 ${moved} 个文件夹`);
  // 并入子项后默认展开目标父级（2026-08-20 用户定：让用户直接看到移动结果，而不是折叠起来）
  if (ok && moved > 0 && targetParent && targetParent !== "root") {
    treeExpansion.openSet.add(targetParent);
  }
  return ok;
}

export async function linkPlacements(
  items: PlacementRef[],
  targetFolder: string,
  names: (string | null)[],
): Promise<boolean> {
  let linked = 0;
  return await runOp(async () => {
    linked = await invoke<number>("link_placements", { items, targetFolder, names });
    // 3.5 新增区：新链接登记置顶（排序状态下可见）
    if (linked > 0) useContentStore().markArrivals(items.map((it) => `${it.assetId}|${targetFolder}`));
  }, () => `关联 ${linked} 个项目`);
}

export async function duplicatePlacements(
  items: PlacementRef[],
  targetFolder: string,
  names: (string | null)[],
): Promise<boolean> {
  let created = 0;
  return await runOp(async () => {
    const ids = await invoke<string[]>("duplicate_placements", { items, targetFolder, names });
    created = ids.length;
    // 副本不进新增区（2026-08-20 用户定）：后端已紧随原件落点，置顶反而把副本拽回第一行第一列
  }, () => `创建 ${created} 个副本`);
}

export async function renamePlacement(ref: PlacementRef, newName: string): Promise<boolean> {
  return await runOp(async () => {
    await invoke("rename_placement", {
      assetId: ref.assetId,
      folderId: ref.folderId,
      origin: ref.origin,
      newName,
    });
  }, () => `重命名为 ${newName.trim()}`);
}

// 将此文件设为原始文件（6.2 四轮走查③，PRD 01 边界规则 8 手动 origin 转移）：
// 目标 Placement 置 origin、原 origin 摘标（库内 origin 恒 ≤ 1）；仅标记不动磁盘；
// name 可选 = 同文件夹重复链接精确到行
export async function setPlacementOrigin(assetId: string, folderId: string, name?: string | null): Promise<boolean> {
  const ok = await runOp(async () => {
    await invoke("set_placement_origin", { assetId, folderId, name: name ?? null });
  }, () => "设为原始文件");
  // 详情链接区块（origin 徽章所在行）随 refresh 重拉
  if (ok) useContentStore().bumpDetailRev();
  return ok;
}

// —— 链接管理（17 稿，6.2 三轮）：一个资产的位置增 / 删 / 改一次提交（单撤销步）——
// 详情链接区块 × 移除 = removes 单条即时提交；LinkManageDialog 确认 = 草稿 diff 三组
export interface SyncLinkAdd {
  folderId: string;
  name: string;
}
// 删除 / 改名定位带可选 name：同文件夹同 origin 的重复链接（2026-08-13 放开）精确到行，
// 缺省 = 三元组第一条（Placement 无自身 ID）
export interface SyncLinkRemove {
  folderId: string;
  origin: boolean;
  name?: string | null;
}
export interface SyncLinkRename {
  folderId: string;
  origin: boolean;
  oldName?: string | null;
  newName: string;
}
export async function syncAssetLinks(
  assetId: string,
  adds: SyncLinkAdd[],
  removes: SyncLinkRemove[],
  renames: SyncLinkRename[],
): Promise<boolean> {
  const ok = await runOp(async () => {
    await invoke("sync_asset_links", { assetId, adds, removes, renames });
  }, () => "完成链接管理");
  // 详情面板链接区块数据源 = get_asset_detail 反查，随 placements 变化重拉（bumpDetailRev 信号）
  if (ok) useContentStore().bumpDetailRev();
  return ok;
}

// 重命名文件夹（2026-07-29 复验三轮，第 10 条命令）：FolderCard 名称双击 / 新建后命名保存共用；
// 非法名后端拒绝走失败 toast，调用方不保留输入态
export async function renameFolder(folderId: string, newName: string): Promise<boolean> {
  return await runOp(async () => {
    await invoke("rename_folder", { folderId, newName });
  }, () => `重命名文件夹为 ${newName.trim()}`);
}

// 更改文件夹图标（2026-08-18，3.3 / 05 稿 §3）：icon = null 恢复默认；
// 同步语义（用户定，与颜色同口径）：链路一致的后代跟随调整，不一致的不动
export async function setFolderIcon(folderId: string, icon: string | null): Promise<boolean> {
  return await runOp(async () => {
    await invoke("set_folder_icon", { folderId, icon });
  }, () => "更改文件夹图标");
}

// 更改文件夹颜色（2026-08-18，3.3 / 05 稿 §4；2026-09-01 批量 + 显式默认修订）：
// 多选域内触发 → 整个选中集（folderIds 多个，单命令单撤销步）；单个 = 单元素数组。
// color = null（「默认」色块）= 显式默认——落盘存 DEFAULT_COLOR 哨兵断开父级链路
//（None 无值的语义是“跟随父级”，直接写 None 会被父级色盖回，永远退不回常态灰）；同步语义同图标
export async function setFoldersColor(folderIds: string[], color: string | null): Promise<boolean> {
  return await runOp(async () => {
    await invoke("set_folder_colors", { folderIds, color: color ?? DEFAULT_COLOR });
  }, () => "更改文件夹颜色");
}

// 删除（C1；2026-07-29 用户修订：取消两段式询问——回收站不是危险操作，素材没丢，
// 撤销可找回，二次确认纯增操作负担）：直接 lastAction="recycle" 移入回收站，
// 不再有"保留为未分类"选项（入口不同而已）；后端两段式机制保留（lastAction=null 干跑）
// 删除分流（3.4 八轮 2026-08-18 两项化 + 2026-08-19 位置级模型"护栏拆干净"）：
// action "recycle" = 丢到回收站（Del）/ "uncategorized" = 从文件夹中移除（Shift+Del，挪未分类不删记录）；
// origin / 链接一视同仁同路径——无"最后一条"概念、无 origin 自动转移（手动转移归阶段六「设为原始文件」）
export async function deletePlacements(items: PlacementRef[], action: "recycle" | "uncategorized" = "recycle"): Promise<void> {
  const toast = useToastStore();
  try {
    await invoke("delete_placements", { items, lastAction: action });
    await runOp(async () => {}, () =>
      action === "uncategorized" ? `从文件夹中移除 ${items.length} 个项目` : `丢到回收站 ${items.length} 个项目`,
    );
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  }
}

// —— 回收站（6.1 二轮，2026-07-29；后端契约并行开发：restore_recycle / delete_recycle / clear_recycle）——

// 彻底删除返回（delete_recycle / clear_recycle 同构）：needConfirm = 有虚拟片段引用或快照链接需确认
interface RecycleDeleteResult {
  needConfirm: boolean;
  segmentRefs: number;
  linkRefs: number; // 快照内链接 Placement 数（2026-08-18 警示口径）
  deleted: number;
}

// 硬删除统一收尾：刷新 + 成功 toast（不可恢复，无撤销按钮——与 runOp 的区别仅此）
async function runHardDelete(label: () => string): Promise<void> {
  const content = useContentStore();
  await content.refresh();
  useToastStore().notify({ message: `已${label()}` });
}

// 回收站视口选中项的 assetId 清单（右键菜单 / Del 键入参；按回收站条目顺序去重输出）
export function recycleSelectedAssetIds(): string[] {
  const selection = useSelectionStore();
  const content = useContentStore();
  const out: string[] = [];
  for (const e of content.recycleEntries) {
    if (selection.selected.has(`${e.assetId}|${e.folderId}`) && !out.includes(e.assetId)) {
      out.push(e.assetId);
    }
  }
  return out;
}

// 还原（restore_recycle）：成功 toast 带撤销（后端撤销标签「还原 N 个项目」）
export async function restoreRecycle(assetIds: string[]): Promise<boolean> {
  if (assetIds.length === 0) return false;
  let restored = 0;
  return await runOp(async () => {
    restored = await invoke<number>("restore_recycle", { assetIds });
  }, () => `还原 ${restored} 个项目`);
}

// 彻底删除两段式：confirmed=false 干跑——needConfirm 时弹确认窗（虚拟片段引用警示，
// RecycleConfirmDialog），确认后 confirmRecycleDelete 带 confirmed=true 再调
export async function deleteRecycle(assetIds: string[]): Promise<void> {
  if (assetIds.length === 0) return;
  const selection = useSelectionStore();
  const toast = useToastStore();
  try {
    const r = await invoke<RecycleDeleteResult>("delete_recycle", { assetIds, confirmed: false });
    if (r.needConfirm) {
      selection.recyclePrompt = { kind: "delete", assetIds, segmentRefs: r.segmentRefs, linkRefs: r.linkRefs };
      return;
    }
    await runHardDelete(() => `彻底删除 ${r.deleted} 个项目`);
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  }
}

// 清空回收站两段式（同 deleteRecycle 口径）
export async function clearRecycle(): Promise<void> {
  const selection = useSelectionStore();
  const toast = useToastStore();
  try {
    const r = await invoke<RecycleDeleteResult>("clear_recycle", { confirmed: false });
    if (r.needConfirm) {
      selection.recyclePrompt = { kind: "clear", assetIds: [], segmentRefs: r.segmentRefs, linkRefs: r.linkRefs };
      return;
    }
    await runHardDelete(() => `清空回收站（${r.deleted} 个项目）`);
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  }
}

// 确认窗确认后执行（RecycleConfirmDialog 唯一入口）
export async function confirmRecycleDelete(): Promise<void> {
  const selection = useSelectionStore();
  const prompt = selection.recyclePrompt;
  selection.recyclePrompt = null;
  if (!prompt) return;
  const toast = useToastStore();
  try {
    const r =
      prompt.kind === "delete"
        ? await invoke<RecycleDeleteResult>("delete_recycle", {
            assetIds: prompt.assetIds,
            confirmed: true,
          })
        : await invoke<RecycleDeleteResult>("clear_recycle", { confirmed: true });
    await runHardDelete(() =>
      prompt.kind === "delete" ? `彻底删除 ${r.deleted} 个项目` : `清空回收站（${r.deleted} 个项目）`,
    );
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  }
}

// 新建文件夹（2026-07-29 复验三轮改"先建后命名"）：直接以默认名建库，成功返回新文件夹 id
// （调用方据此让树新节点进命名态）；失败 = null + 中文 toast——不再有"命名中临时节点"，
// 也就不再有命名行内报错（旧流程的取消不入库 / 行内提示一并移除）。
// blockId（2026-08-14 区块子系统）：仅顶级新建（parentId null）生效——写入区块归属并继承
// 区块默认图标；非顶级新建忽略（传 "dir" = 「目录」，与后端缺省同语义）
export async function createFolder(
  parentId: string | null,
  name: string,
  afterFolderId: string | null,
  blockId: string | null = null,
): Promise<string | null> {
  const content = useContentStore();
  const toast = useToastStore();
  const undo = useUndoStore();
  try {
    const folder = await invoke<{ id: string }>("create_folder", { parentId, name, afterFolderId, blockId });
    await content.refresh();
    void undo.refreshState();
    toast.notify({ message: "已新建文件夹", actionLabel: "撤销", onAction: () => void undo.undo() });
    return folder.id;
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
    return null;
  }
}

// 新建档案（2026-08-15 档案子系统一轮，同 createFolder 先建后命名口径）：kind = archive 的
// 文件夹，afterFolderId / blockId 口径同 create_folder；成功返回新 id 供调用方进命名态
export async function createArchive(
  parentId: string | null,
  name: string,
  afterFolderId: string | null,
  blockId: string | null = null,
): Promise<string | null> {
  const content = useContentStore();
  const toast = useToastStore();
  const undo = useUndoStore();
  try {
    const folder = await invoke<{ id: string }>("create_archive", { parentId, name, afterFolderId, blockId });
    await content.refresh();
    void undo.refreshState();
    toast.notify({ message: "已新建档案", actionLabel: "撤销", onAction: () => void undo.undo() });
    return folder.id;
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
    return null;
  }
}

// 文件夹转换为档案（3.2 一轮两段式）：confirmed = false 为询问段——有子文件夹返回
// "need_confirm"（后端未动数据，确认弹窗编排在 menuOps 流程层）；"done" = 已转换
// （refresh + 成功 toast）；失败 = null + 失败 toast
export async function convertToArchive(
  folderId: string,
  confirmed: boolean,
): Promise<"done" | "need_confirm" | null> {
  const content = useContentStore();
  const toast = useToastStore();
  const undo = useUndoStore();
  try {
    const r = await invoke<"done" | "need_confirm">("convert_to_archive", { folderId, confirmed });
    if (r === "done") {
      await content.refresh();
      void undo.refreshState();
      toast.notify({ message: "已转换为档案", actionLabel: "撤销", onAction: () => void undo.undo() });
    }
    return r;
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
    return null;
  }
}

// 解散档案为文件夹（3.2 一轮）：摘 kind 标记 + 清档案数据记录（后端幂等）
export async function convertToFolder(folderId: string): Promise<boolean> {
  return await runOp(async () => {
    await invoke("convert_to_folder", { folderId });
  }, () => "解散档案为文件夹");
}

// 新建文档入库（3.1 二轮通用化，2026-08-15 用户定：固定七类 md/txt/fla/psd/ai/docx/xlsx，
// 二进制类以后端内嵌空白模板建档）：
// 落当前视口目录（同导入位置口径），先建后命名——成功返回 PlacementView 后 renamingKey 进重命名态
export type NewDocKind = "md" | "txt" | "fla" | "psd" | "ai" | "docx" | "xlsx";
export async function createDocument(kind: NewDocKind): Promise<void> {
  const content = useContentStore();
  const selection = useSelectionStore();
  const toast = useToastStore();
  const undo = useUndoStore();
  try {
    const view = await invoke<{ assetId: string; folderId: string }>("create_document", {
      kind,
      targetFolder: content.currentFolderId,
    });
    content.markArrivals([`${view.assetId}|${view.folderId}`]); // 3.5 新增区：新建文档置顶可见
    await content.refresh();
    void undo.refreshState();
    selection.renamingKey = `${view.assetId}|${view.folderId}`;
    toast.notify({ message: "已新建文档", actionLabel: "撤销", onAction: () => void undo.undo() });
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  }
}

// —— 内部剪贴板（C1）——

// Ctrl+C / Ctrl+X：写入内部剪贴板（仅素材；空选择不动作）；
// 复制 / 剪切即时反馈（2026-07-28 复验反馈 7）：成功型、无动作按钮、3 秒
//（2026-08-20 修复：原 icon:"fail" 是 info 型废除时的误改——成功操作用失败提示，用户实机报）；
// 复制时同步写系统剪贴板（2026-09-17 实机走查：复制图片要能粘到 APP 外——网页 AI 对话框等，
// CF_HDROP 文件 + 单张静态图附位图；后端解析 origin 真实路径，ws:/解析失败项跳过，静默不打扰）
export function copySelection(action: "copy" | "cut"): void {
  const selection = useSelectionStore();
  const items = selection.selectedRefs();
  if (items.length === 0) return;
  selection.clipboard = { items, action };
  // 记录写入时刻的系统剪贴板序号（复制/剪切都记——粘贴仲裁基准：序号变了 = 外部覆盖）
  void invoke<number>("get_clipboard_seq")
    .then((s) => {
      selection.clipboardSeq = s;
    })
    .catch(() => {});
  if (action === "copy") {
    const assetIds = [...new Set(items.map((i) => i.assetId))];
    void invoke("copy_files_to_system_clipboard", { assetIds }).catch((e) =>
      console.debug("系统剪贴板写入不可用：", e),
    );
  }
  useToastStore().notify({
    message: `已${action === "copy" ? "复制" : "剪切"} ${items.length} 个项目`,
    duration: 3000,
  });
}

// 粘贴落点判定（2026-07-28 定案；2026-09-09 入口收窄修订）：快速访问视图不作粘贴目标——
// **未分类豁免**（候车区总入口，落点 "root" 所见即所得）；全部 / 最近使用 / 自动导入 /
// 缩略图 / 回收站仍拦截（虚拟视图不是位置、自动导入只走自动通道、回收站只读）
function pasteTarget(): string | null {
  const content = useContentStore();
  if (content.sidebarSelected.startsWith("folder:")) return content.currentFolderId;
  if (content.sidebarSelected === "quick:uncategorized") return "root";
  useToastStore().notify({ message: "当前视口不支持粘贴", icon: "fail" });
  return null;
}

// 剪切文件夹（3.4，2026-08-18）：写内部剪贴板（folders 分支，与素材互斥由选择域保证）；
// 粘贴 = move_folders 移动（一次性，粘贴后清空）；剪切期间源项半透明（selection.isFolderCut）
export function cutFolders(folderIds: string[]): void {
  if (folderIds.length === 0) return;
  const selection = useSelectionStore();
  selection.clipboard = { items: [], action: "cut", folders: folderIds };
  useToastStore().notify({ message: `已剪切 ${folderIds.length} 个文件夹`, duration: 3000 });
}

// Ctrl+V 仲裁（内部剪贴板非空分支；空 → App.vue 走系统剪贴板导入）：
// 文件夹剪切 = move_folders 落当前视口文件夹（复用后端护栏：禁入自身/子级、档案不作父级——
// 失败后端中文 toast 直出）；源失效预检 toast（PRD 02-13）；
// copy = 静默链接粘贴（2026-08-18 用户定：不弹窗——高频整理打扰；命名导入后手动 F2 / 批量重命名，
// 原"关联弹窗 / 选择性粘贴清单弹窗"设计整体废除）；cut = 直接移动到当前文件夹（粘贴一次后清空剪贴板）
export async function pasteClipboard(): Promise<void> {
  const selection = useSelectionStore();
  const clip = selection.clipboard;
  if (!clip) return;
  const target = pasteTarget();
  if (target === null) return;
  if (clip.folders && clip.folders.length > 0) {
    await pasteFoldersInto(target);
    return;
  }
  if (clip.action === "copy") {
    // 链接发起不设限（2026-08-13 用户定案：不替用户做整理决定，候车区拦截废除）
    await linkPlacements(clip.items, target, clip.items.map(() => null));
    return;
  }
  const ok = await movePlacements(clip.items, target);
  if (ok) selection.clipboard = null;
}

// 文件夹剪切粘贴落点执行（3.4）：源失效预检 toast（PRD 02-13）；复用 move_folders 护栏
//（禁入自身/子级、档案不作父级——失败后端中文 toast 直出）；成功清剪贴板（一次性）
export async function pasteFoldersInto(target: string): Promise<void> {
  const selection = useSelectionStore();
  const clip = selection.clipboard;
  if (!clip?.folders?.length) return;
  const content = useContentStore();
  const alive = clip.folders.filter((id) => content.folderName(id) !== null);
  if (alive.length === 0) {
    selection.clipboard = null; // 源全部失效：清剪贴板并提示
    useToastStore().notify({ message: "剪切的文件夹已不存在", icon: "fail" });
    return;
  }
  const ok = await moveFolders(alive, target);
  if (ok) selection.clipboard = null;
}

// Ctrl+Shift+V：直接副本不弹窗（名称同源 = names 全 null）
export async function pasteDuplicate(): Promise<void> {
  const selection = useSelectionStore();
  const clip = selection.clipboard;
  if (!clip) return;
  const target = pasteTarget();
  if (target === null) return;
  await duplicatePlacements(clip.items, target, clip.items.map(() => null));
}

// 创建副本（素材卡片右键 / Ctrl+D，2026-08-18 自 assetCard spec 提为共用）：各选中项复制进其
// 自身所在文件夹（副本紧随原件，名称同源 = names 全 null）；跨文件夹选中按文件夹分组逐组执行
export async function duplicateSelected(): Promise<void> {
  const selection = useSelectionStore();
  const byFolder = new Map<string, PlacementRef[]>();
  for (const ref of selection.selectedRefs()) {
    const list = byFolder.get(ref.folderId) ?? [];
    list.push(ref);
    byFolder.set(ref.folderId, list);
  }
  for (const [folderId, items] of byFolder) {
    await duplicatePlacements(items, folderId, items.map(() => null));
  }
}

// —— 6.3 右键菜单配套（2026-07-29，设计稿 `00_修改与需求/05_右键菜单`）——

// 其他应用打开（系统"打开方式"对话框）：失败中文 toast
export async function openWithDialog(assetId: string): Promise<void> {
  try {
    await invoke("open_with_dialog", { assetId });
  } catch (e) {
    useToastStore().notify({ message: String(e), icon: "fail" });
  }
}

// 在文件管理器中打开（前端先弹警示窗再调——菜单流程见 menuOps.revealWithWarning）
export async function revealInExplorer(assetId: string): Promise<void> {
  try {
    await invoke("reveal_in_explorer", { assetId });
  } catch (e) {
    useToastStore().notify({ message: String(e), icon: "fail" });
  }
}

// 缩略图三件套：不接撤销（同设封面帧既有口径）；成功刷新 + 普通 toast
async function runThumbOp(action: () => Promise<unknown>, okMessage: string): Promise<void> {
  const content = useContentStore();
  const toast = useToastStore();
  try {
    await action();
    await content.refresh();
    toast.notify({ message: okMessage, icon: "success" });
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  }
}

// 复制缩略图（5.4）：缩略图位图进剪贴板，全类型含 shell 格式图标——纯读不改数据，不 refresh
export async function copyThumb(assetId: string): Promise<void> {
  const toast = useToastStore();
  try {
    await invoke("copy_thumbnail_to_clipboard", { assetId });
    toast.notify({ message: "已复制缩略图", icon: "success" });
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  }
}

// 将剪贴板应用至缩略图（2026-07-23 待办落地）
export async function applyClipboardThumb(assetId: string): Promise<void> {
  await runThumbOp(() => invoke("set_thumbnail_from_clipboard", { assetId }), "已将剪贴板应用至缩略图");
}

// 自定义缩略图（系统文件选择器选定后调用）
export async function setCustomThumb(assetId: string, sourcePath: string): Promise<void> {
  await runThumbOp(() => invoke("set_thumbnail_custom", { assetId, sourcePath }), "已自定义缩略图");
}

// 刷新缩略图（强制重生成，防自检失败）
export async function refreshThumb(assetId: string): Promise<void> {
  await runThumbOp(() => invoke("refresh_thumbnail", { assetId }), "已刷新缩略图");
}

// —— 缩略图链接（批次 4，2026-09-02，PRD 04-03 缩略图组）：「转换为缩略图链接并复制」= 纯引用复制
//（会话级 thumbLink，不动库数据——hidden 在实际绑定时由 set_thumb_asset 打）；「将 xx 设置为缩略图」= 绑定。
// 重复绑定 = 换绑语义（用户定保守路径）：工程改指向新资产，旧 hidden 不动（手动「解除缩略图」或删除）。
export function copyThumbLink(assetId: string, name: string): void {
  useSelectionStore().thumbLink = { assetId, name };
  useToastStore().notify({ message: `已复制缩略图链接：${name}`, duration: 3000 });
}

export async function applyThumbLink(targetAssetId: string): Promise<void> {
  const link = useSelectionStore().thumbLink;
  if (!link) return;
  await runThumbOp(
    () => invoke("set_thumb_asset", { assetId: targetAssetId, thumbId: link.assetId }),
    `已将「${link.name}」设置为缩略图`,
  );
}

// 还原为素材（PRD 02-15；批次 4 挂前端入口，仅缩略图视图 hidden 资产右键可见）：
// 取消 hidden（链接位置原样恢复显示）+ 清除全部工程侧 thumb_asset_id 引用；不接撤销（同三件套口径）
export async function restoreThumbAsset(assetId: string): Promise<void> {
  const content = useContentStore();
  const toast = useToastStore();
  try {
    const cleared = await invoke<number>("restore_thumb_asset", { assetId });
    await content.refresh();
    toast.notify({ message: `已还原为素材${cleared > 0 ? `（清除 ${cleared} 处绑定）` : ""}`, icon: "success" });
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  }
}

// 发送至 Animate（8.3，PRD 07-04 快速发送）：fla → 剪贴板载荷 → WS 下发 jsx 原位粘贴；
// 转换失败后端自动回退兜底（打开-复制-粘贴-关闭）。面板未连接 / 在途重复发送由后端报错，此处透传。
// 点击即最小化主窗口（2026-09-08 实机走查第 20 条）：用户要看的是 AN 侧有没有导入，
// 且 AN 弹窗（重复元件选择框等）不能被 APP 挡住——不等发送结果；结果 toast 回窗后可见。
export async function sendToAnimate(assetId: string): Promise<void> {
  const toast = useToastStore();
  void getCurrentWindow().minimize();
  try {
    const name = await invoke<string>("send_to_animate", { assetId });
    toast.notify({ message: `已发送至 Animate：${name}`, icon: "success" });
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  }
}

// 导出（库外写操作，不接撤销）：目录由系统对话框选定（menuOps 流程）。
// 任务化（2026-08-14 用户定：导出用导入的进度条样式与位置）——进度条 / 失败弹窗 /
// 完成 toast 全部走 content.runExportTask 编排（Channel 事件收尾，取消后不提示）
export async function exportAssets(assetIds: string[], targetDir: string): Promise<void> {
  await useContentStore().runExportTask((onEvent) => invoke<number>("export_assets", { assetIds, targetDir, onEvent }));
}

export async function exportFolder(folderId: string, targetDir: string): Promise<void> {
  await useContentStore().runExportTask((onEvent) => invoke<number>("export_folder", { folderId, targetDir, onEvent }));
}

// 删除文件夹报告（后端 DeleteFolderReport 同构）
export interface DeleteFolderResult {
  needConfirm: boolean;
  segmentRefs: number;
  assetCount: number;
  folderCount: number;
  movedPlacements: number;
  deletedFolders: number;
  deleted: number;
}

// 删除文件夹（素材移未分类，可撤销）：成功 toast 带撤销
export async function deleteFolder(folderId: string): Promise<boolean> {
  let report: DeleteFolderResult | null = null;
  return await runOp(async () => {
    report = await invoke<DeleteFolderResult>("delete_folder", { folderId, permanent: false, confirmed: false });
  }, () => `删除文件夹（${report?.movedPlacements ?? 0} 个素材移入未分类）`);
}

// 批量删除文件夹（3.4，2026-08-18）：一次确认单命令单撤销步（后端 delete_folders 单趟，
// 祖先同批入选的后代去重、陈旧 id 容错）；报告聚合，toast 计数与单删同口径
export async function deleteFolders(folderIds: string[]): Promise<boolean> {
  let report: DeleteFolderResult | null = null;
  return await runOp(async () => {
    report = await invoke<DeleteFolderResult>("delete_folders", { folderIds });
  }, () => `删除 ${folderIds.length} 个文件夹（${report?.movedPlacements ?? 0} 个素材移入未分类）`);
}

// 批量重命名（3.4，37 稿）：逐条改名（链接仅显示名 / 原始同步磁盘 originName），
// 撞名不拦截（后端不唯一化）、失败项跳过不整体回滚（toast 汇总）——runOp 不适用：
// 部分失败要换 fail 图标文案，toast 自组（refresh / 撤销入口同 runOp 口径）
export interface BatchRenameReport {
  renamed: number;
  failed: { name: string; reason: string }[];
}
export interface RenameItemInput {
  assetId: string;
  folderId: string;
  origin: boolean;
  newName: string;
}
export async function renamePlacementsBatch(items: RenameItemInput[]): Promise<boolean> {
  const content = useContentStore();
  const toast = useToastStore();
  const undo = useUndoStore();
  try {
    const r = await invoke<BatchRenameReport>("rename_placements", { items });
    await content.refresh();
    void undo.refreshState();
    if (r.failed.length > 0) {
      toast.notify({
        message: `已重命名 ${r.renamed} 个项目，${r.failed.length} 个失败（${r.failed[0].reason}）`,
        icon: "fail",
        actionLabel: "撤销",
        onAction: () => void undo.undo(),
      });
    } else {
      toast.notify({
        message: `已批量重命名 ${r.renamed} 个项目`,
        actionLabel: "撤销",
        onAction: () => void undo.undo(),
      });
    }
    return true;
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
    return false;
  }
}

// 克隆文件夹报告（后端 CloneReport 同构）
export interface CloneFoldersResult {
  folderIds: string[];
  folderCount: number;
  assetCount: number;
}

// 克隆文件夹（3.4，2026-08-18）：复制克隆无弹窗——子树结构 + 资产物理副本（新 ID）+ 档案记录随迁，
// 后端 clone_folders 单命令单撤销步；批量 = ids 多棵
export async function cloneFolders(folderIds: string[]): Promise<boolean> {
  let report: CloneFoldersResult | null = null;
  return await runOp(async () => {
    report = await invoke<CloneFoldersResult>("clone_folders", { folderIds });
  }, () =>
    folderIds.length > 1
      ? `克隆 ${folderIds.length} 个文件夹（${report?.assetCount ?? 0} 个素材生成副本）`
      : `克隆文件夹（${report?.assetCount ?? 0} 个素材生成副本）`,
  );
}

// 复制 base64（后端守卫：静态图片原字节 ≤10MB；psd/psb/svg 解码 PNG ≤10MB）
export async function getAssetBase64(assetId: string): Promise<string | null> {
  try {
    return await invoke<string>("get_asset_base64", { assetId });
  } catch (e) {
    useToastStore().notify({ message: String(e), icon: "fail" });
    return null;
  }
}

// 资产源文件真实绝对路径（复制文件路径用；后端恒返回 info 内 origin，与预览路径解析无关）
export async function getAssetOriginPath(assetId: string): Promise<string | null> {
  try {
    return await invoke<string>("get_asset_origin_path", { assetId });
  } catch (e) {
    useToastStore().notify({ message: String(e), icon: "fail" });
    return null;
  }
}

// —— 序列帧包（5.2 包内视图，PRD 04-序列帧 / 05-序列帧打包）——

// 解散序列帧包导入（危险操作，不可撤销——确认弹窗编排在 menuOps.dissolveSequenceFlow）：
// 每帧移动出库为独立素材、原包整体删除（逐帧笔记随之删除）；后端不进 undo，硬删除收尾（无撤销按钮）
export async function dissolveSequence(assetId: string): Promise<void> {
  const toast = useToastStore();
  try {
    const count = await invoke<number>("dissolve_sequence", { assetId });
    await runHardDelete(() => `解散序列帧包为 ${count} 个独立素材`);
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  }
}

// 删除序列帧包的帧（抽帧调节奏）：帧移 cache/seq-trash 暂存，会话内可撤销（toast 带撤销按钮）；
// 封面帧被删改指剩余首帧并重建缩略图；至少保留 1 帧（后端护栏）。
// 不走 runOp：0 删除（全陈旧名）时后端不登记 undo 事务，toast 带撤销按钮会误关联到别的历史操作
export async function deleteSequenceFrames(assetId: string, frameNames: string[]): Promise<void> {
  if (frameNames.length === 0) return;
  const toast = useToastStore();
  try {
    const deleted = await invoke<number>("delete_sequence_frames", { assetId, frameNames });
    if (deleted === 0) return;
    await useContentStore().refresh();
    const undo = useUndoStore();
    void undo.refreshState();
    toast.notify({ message: `已删除 ${deleted} 张序列帧`, actionLabel: "撤销", onAction: () => void undo.undo() });
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  }
}

// 重排序号：剩余帧按当前顺序重排连续序号（词干-001..N），帧笔记 key 随迁移；可撤销。
// 不走 runOp 同上：0 改名（序号已连续）时无 undo 事务，提示语也不带撤销按钮
export async function renumberSequenceFrames(assetId: string): Promise<void> {
  const toast = useToastStore();
  try {
    const renamed = await invoke<number>("renumber_sequence_frames", { assetId });
    if (renamed === 0) {
      toast.notify({ message: "序号已连续，无需重排" });
      return;
    }
    await useContentStore().refresh();
    const undo = useUndoStore();
    void undo.refreshState();
    toast.notify({ message: `已重排 ${renamed} 张序列帧序号`, actionLabel: "撤销", onAction: () => void undo.undo() });
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  }
}

// 将当前帧设为预览图（包内视图帧卡片菜单，2026-08-21 用户定）：直复用播放条同款 set_thumb_frame
//（序列帧分支传 frames/ 内帧名）；refresh 全量重拉——modifiedAt 版本戳驱动卡片缩略图热刷新；
// toast 反馈与播放条同款（已设为预览图 / 错误文案）
export async function setSequenceFrameThumb(assetId: string, frameName: string): Promise<void> {
  const toast = useToastStore();
  try {
    await invoke("set_thumb_frame", { assetId, frameName });
    await useContentStore().refresh();
    toast.notify({ message: "已设为预览图", icon: "success" });
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  }
}
