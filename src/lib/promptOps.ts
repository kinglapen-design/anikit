// 提示词操作（提示词轮，2026-09-05 走查三态定案接业务）：文件 / 档案的提示词绑定、分析、
// 新建文档、删除文档与「打开编辑器」预览跳转。
// 三态（PlacementView.promptState / PromptBinding 派生）：none = 未指定 / scanned = 已扫描
// （分析快照落库，assetId null 且有 content）/ linked = 已指定文本文档（assetId 非空）。
// 展示语义：编辑框只读；linked 态实时经 read_text_asset 读源最新内容（失败回落快照 content）。
// 统一收尾：单目标 = customDataOps run() 范式（invoke → refresh / refreshArchives → bumpDetailRev
// → undo.refreshState → toast 带「撤销」）；多目标批量 = 既有批量惯例（pasteTags/clearTags）——
// 逐目标 try、失败计数不中断、跳过不兼容项，一次刷新 + 汇总 toast（不带撤销钮，Ctrl+Z 逐步回滚）。
// 指定流程在选择模式（stores/picker purpose "prompt"）——picker 不反向依赖本模块（防循环引用）。
import { invoke } from "@tauri-apps/api/core";
import { useContentStore, type PlacementView, type PromptBinding } from "../stores/content";
import { usePickerStore } from "../stores/picker";
import { useTabsStore } from "../stores/tabs";
import { useToastStore } from "../stores/toast";
import { useUndoStore } from "../stores/undo";
import { previewSession } from "../stores/preview";
import { readTextAsset } from "./textOps";
import { copyText } from "./menuOps";
import type { PickerTarget } from "./relationOps";

/** 提示词三态（与 PlacementView.promptState 同字面量） */
export type PromptState = "none" | "scanned" | "linked";

/** 提示词操作目标：文件 = assetId；档案 = folderId */
export type PromptTargetRef = { kind: "asset"; assetId: string } | { kind: "archive"; folderId: string };

/** 批量目标集（多选 = 全部选中资产按 assetId 去重 + 档案 folderId 清单；档案动作单目标） */
export interface PromptTargets {
  assets: { assetId: string; folderId: string }[];
  archives: string[];
}

// 分析提示词可用格式（2026-09-05 走查定案：仅 png/jpg/jpeg/webp）
const ANALYZABLE_EXTS = new Set(["png", "jpg", "jpeg", "webp"]);

/** 分析提示词格式闸（菜单禁用 + 批量跳过同一判定） */
export function isPromptAnalyzableExt(ext: string): boolean {
  return ANALYZABLE_EXTS.has(ext.toLowerCase());
}

/** 绑定 → 三态（契约：null/缺 = 未指定；assetId 非空 = 已指定；assetId null 且有 content = 已扫描） */
export function promptStateOfBinding(b: PromptBinding | null | undefined): PromptState {
  if (!b) return "none";
  if (b.assetId) return "linked";
  return b.content ? "scanned" : "none";
}

/** 文件三态：后端 views 同径带出 promptState；缺省 = 未指定（并行开发期降级） */
export function promptStateOfAsset(p: PlacementView): PromptState {
  return p.promptState ?? "none";
}

/** 档案三态：读 content.archives（list_archives 同径带出 prompt） */
export function promptStateOfArchive(folderId: string): PromptState {
  return promptStateOfBinding(useContentStore().archives[folderId]?.prompt);
}

// —— 绑定 / 文本读取 ——

// 取目标当前绑定（菜单层无 detail，先拉 get_asset_detail / get_archive_data）；失败上抛由调用方 toast
async function fetchPromptBinding(target: PromptTargetRef): Promise<PromptBinding | null> {
  if (target.kind === "asset") {
    const d = await invoke<{ prompt?: PromptBinding | null }>("get_asset_detail", { assetId: target.assetId });
    return d.prompt ?? null;
  }
  const d = await invoke<{ prompt?: PromptBinding | null }>("get_archive_data", { folderId: target.folderId });
  return d.prompt ?? null;
}

/** 读目标提示词文本：linked 且源可读走 read_text_asset（源最新内容），否则快照 content；
 *  无 prompt / 无内容 = null；拉取失败上抛（调用方 toast） */
export async function readPromptText(target: PromptTargetRef): Promise<string | null> {
  const prompt = await fetchPromptBinding(target);
  if (!prompt) return null;
  if (prompt.assetId) {
    try {
      return (await readTextAsset(prompt.assetId)).content;
    } catch {
      // 源不可读（回收站 / 已删）回落快照 content
    }
  }
  return prompt.content || null;
}

/** 复制提示词文本：系统剪贴板 + 成功 toast（3s，menuOps copyText 复用） */
export function copyPromptText(text: string): Promise<void> {
  return copyText(text, "已复制提示词");
}

/** 菜单「复制提示词」流程（菜单层无 detail：先拉绑定再读文本） */
export async function copyPrompt(target: PromptTargetRef): Promise<void> {
  const toast = useToastStore();
  let text: string | null = null;
  try {
    text = await readPromptText(target);
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
    return;
  }
  if (!text) {
    toast.notify({ message: "没有提示词可复制", icon: "fail" });
    return;
  }
  await copyPromptText(text);
}

// —— 写操作统一收尾 ——

// 刷新选择：碰过资产（promptState 在 PlacementView 上）→ 全量 refresh；纯档案 → refreshArchives
async function refreshAfter(touchedAssets: boolean): Promise<void> {
  const content = useContentStore();
  if (touchedAssets) await content.refresh();
  else await content.refreshArchives();
  content.bumpDetailRev();
  void useUndoStore().refreshState();
}

// 批量收尾 toast：单目标带「撤销」（单撤销步命令），批量不带（多步，Ctrl+Z 逐步回滚，同 clearTags 惯例）
function notifyDone(message: string, opts: { failed?: number; singleUndo?: boolean } = {}): void {
  const toast = useToastStore();
  const suffix = opts.failed ? `，${opts.failed} 个失败` : "";
  const undo = useUndoStore();
  toast.notify({
    message: `${message}${suffix}`,
    ...(opts.singleUndo ? { actionLabel: "撤销", onAction: () => void undo.undo() } : {}),
  });
}

// —— 分析提示词（仅 png/jpg/jpeg/webp；已扫描态 = 重新分析同命令）——

export async function analyzePrompt(assetIds: string[]): Promise<void> {
  const content = useContentStore();
  const toast = useToastStore();
  // 跳过不兼容项（多选批量逐个过滤；单选入口菜单已按格式禁用，此处双保险）
  const analyzable = assetIds.filter((id) => {
    const p = content.placements.find((x) => x.assetId === id);
    return !!p && isPromptAnalyzableExt(p.ext);
  });
  const skipped = assetIds.length - analyzable.length;
  if (analyzable.length === 0) {
    toast.notify({ message: "所选文件不支持分析提示词（仅 png/jpg/jpeg/webp）", icon: "fail" });
    return;
  }
  let done = 0;
  let failed = 0;
  let firstError: string | null = null;
  for (const assetId of analyzable) {
    try {
      await invoke("analyze_asset_prompt", { assetId });
      done++;
    } catch (e) {
      failed++;
      firstError = firstError ?? String(e);
    }
  }
  if (done === 0) {
    toast.notify({ message: firstError ?? "分析提示词失败", icon: "fail" });
    return;
  }
  await refreshAfter(true);
  const single = analyzable.length === 1 && failed === 0;
  notifyDone(
    done > 1 || skipped > 0
      ? `已分析 ${done} 个文件的提示词${skipped > 0 ? `，跳过 ${skipped} 个不兼容` : ""}`
      : "已分析提示词",
    { failed, singleUndo: single },
  );
}

// —— 指定提示词（选择模式入口；执行在 stores/picker pickPromptAsset）——

/** 进入指定提示词选择模式：origin = 右键卡 / 详情目标，batch = 全部目标（含 origin） */
export function assignViaPicker(origin: PickerTarget, batch: PickerTarget[]): void {
  usePickerStore().enterPrompt(origin, batch);
}

// —— 断开 / 清空（共用 clear_*_prompt；已指定「断开链接」与已扫描「清空」同命令）——

export async function clearPrompts(
  targets: PromptTargets,
  verb: "断开提示词文档链接" | "断开提示词链接" | "清空提示词" | "删除提示词" = "清空提示词",
): Promise<void> {
  const toast = useToastStore();
  const total = targets.assets.length + targets.archives.length;
  if (total === 0) return;
  let done = 0;
  let failed = 0;
  let firstError: string | null = null;
  for (const a of targets.assets) {
    try {
      // label 传菜单语义（断开/清空），后端缺省「移除提示词」（2026-09-26 提示词轮）
      await invoke("clear_asset_prompt", { assetId: a.assetId, label: verb });
      done++;
    } catch (e) {
      failed++;
      firstError = firstError ?? String(e);
    }
  }
  for (const folderId of targets.archives) {
    try {
      await invoke("clear_archive_prompt", { folderId, label: verb });
      done++;
    } catch (e) {
      failed++;
      firstError = firstError ?? String(e);
    }
  }
  if (done === 0) {
    toast.notify({ message: firstError ?? `${verb}失败`, icon: "fail" });
    return;
  }
  await refreshAfter(targets.assets.length > 0);
  notifyDone(done > 1 ? `已${verb}（${done} 个目标）` : `已${verb}`, {
    failed,
    singleUndo: total === 1 && failed === 0,
  });
}

// —— 同步快照（已指定态重抓源内容进快照；详情面板「保存提示词」= 本操作）——

export async function refreshPromptSnapshot(target: PromptTargetRef): Promise<void> {
  const toast = useToastStore();
  const undo = useUndoStore();
  try {
    if (target.kind === "asset") {
      await invoke("refresh_asset_prompt_snapshot", { assetId: target.assetId });
    } else {
      await invoke("refresh_archive_prompt_snapshot", { folderId: target.folderId });
    }
    await refreshAfter(target.kind === "asset");
    toast.notify({ message: "已同步提示词快照", actionLabel: "撤销", onAction: () => void undo.undo() });
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  }
}

// —— 新建提示词文档（未指定 = 空文档；已扫描 = 快照内容写盘并绑定——「保存为文档」同命令）——

// 打开提示词文档进内置预览编辑器（per-panel 会话，panelId 缺省 = 激活面板）：
// md 先置 pendingMdEdit 一次性信号（PreviewText 内容就绪后消费直进编辑态）；txt 恒编辑态无需处理
function openPromptPlacement(p: PlacementView, panelId?: string): void {
  const id = panelId ?? useTabsStore().activeTabId;
  if (!id) return;
  const session = previewSession(id);
  if (p.ext.toLowerCase() === "md") session.pendingMdEdit = true;
  void session.open(`${p.assetId}|${p.folderId}`);
}

export async function newPromptDocuments(targets: PromptTargets, kind: "md" | "txt"): Promise<void> {
  const content = useContentStore();
  const toast = useToastStore();
  const total = targets.assets.length + targets.archives.length;
  if (total === 0) return;
  let done = 0;
  let failed = 0;
  let firstError: string | null = null;
  const created: PlacementView[] = [];
  for (const a of targets.assets) {
    try {
      created.push(
        await invoke<PlacementView>("new_asset_prompt_document", { assetId: a.assetId, folderId: a.folderId, kind }),
      );
      done++;
    } catch (e) {
      failed++;
      firstError = firstError ?? String(e);
    }
  }
  for (const folderId of targets.archives) {
    // 档案落点 = 档案自身（文档名 = 档案名），命令无 assetId
    try {
      created.push(await invoke<PlacementView>("new_archive_prompt_document", { folderId, kind }));
      done++;
    } catch (e) {
      failed++;
      firstError = firstError ?? String(e);
    }
  }
  if (done === 0) {
    toast.notify({ message: firstError ?? "新建提示词文档失败", icon: "fail" });
    return;
  }
  // 新文档插入列表 = createDocument 同款：3.5 新增区登记置顶 + 全量 refresh 对齐
  content.markArrivals(created.map((v) => `${v.assetId}|${v.folderId}`));
  await content.refresh();
  content.bumpDetailRev();
  void useUndoStore().refreshState();
  const single = total === 1 && failed === 0;
  notifyDone(done > 1 ? `已新建 ${done} 个提示词文档` : "已新建提示词文档", { failed, singleUndo: single });
  // 单目标新建后自动打开新文档进内置编辑器（refresh 后 placements 已就位，md 直进编辑态）
  if (single && created[0]) openPromptPlacement(created[0]);
}

// —— 删除提示词文档（源文档丢回收站 + prompt 清除，单撤销步）——

export async function deletePromptDocuments(targets: PromptTargets): Promise<void> {
  const toast = useToastStore();
  const total = targets.assets.length + targets.archives.length;
  if (total === 0) return;
  let done = 0;
  let failed = 0;
  let firstError: string | null = null;
  for (const a of targets.assets) {
    try {
      await invoke("delete_asset_prompt_document", { assetId: a.assetId });
      done++;
    } catch (e) {
      failed++;
      firstError = firstError ?? String(e);
    }
  }
  for (const folderId of targets.archives) {
    try {
      await invoke("delete_archive_prompt_document", { folderId });
      done++;
    } catch (e) {
      failed++;
      firstError = firstError ?? String(e);
    }
  }
  if (done === 0) {
    toast.notify({ message: firstError ?? "删除提示词文档失败", icon: "fail" });
    return;
  }
  await refreshAfter(true); // 源文档丢回收站动了 placements，恒全量
  notifyDone(done > 1 ? `已删除 ${done} 个提示词文档` : "已删除提示词文档", {
    failed,
    singleUndo: total === 1 && failed === 0,
  });
}

// —— 打开编辑器（已指定态：打开源文档内置预览编辑器；md 直进编辑态）——

export async function openPromptEditor(target: PromptTargetRef, panelId?: string): Promise<void> {
  const toast = useToastStore();
  let prompt: PromptBinding | null = null;
  try {
    prompt = await fetchPromptBinding(target);
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
    return;
  }
  if (!prompt?.assetId) {
    toast.notify({ message: "没有提示词文档可打开", icon: "fail" });
    return;
  }
  // 源无活 placement（回收站 / 已删）——调用方按此禁用兜底，此处再护一道
  const content = useContentStore();
  const p =
    content.placements.find((x) => x.assetId === prompt!.assetId && x.origin) ??
    content.placements.find((x) => x.assetId === prompt!.assetId);
  if (!p) {
    toast.notify({ message: "提示词文档已不在库中（可能在回收站）", icon: "fail" });
    return;
  }
  openPromptPlacement(p, panelId);
}
