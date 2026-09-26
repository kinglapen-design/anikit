// 提示词菜单三态构建器（2026-09-05 走查定案，提示词轮接业务；自 assetCard/archiveCard 骨架提取共享）：
// 三处调用 = 素材卡片右键（assetCard）/ 档案卡片右键（archiveCard，树档案节点同径）/ 菜单栏主菜单（appMenu）。
// 文件·未指定 = 分析提示词（仅 png/jpg/jpeg/webp 可用）/ 指定文本文件为提示词 / 新建 .md / 新建 .txt；
// 文件·已指定 = 打开文本编辑器 / 复制提示词 / 断开提示词文档链接 / 删除提示词文档；
// 文件·已扫描 = 保存提示词为 .md 文档 / 保存为 .txt 文档 / 复制提示词 / 重新分析提示词 / 清空提示词；
// 档案 = 无「分析」（走查原文），未指定 = 指定文本文档 / 新建 .md / 新建 .txt，已指定同文件四项。
// 多选（文件右键）：菜单态按右键那张卡的 promptState 出；动作对全部选中资产逐个执行、
// 跳过不兼容项（analyzePrompt 内过滤）、toast 汇总（promptOps 批量惯例）；档案动作单目标。
import { useContentStore, type PlacementView } from "../../stores/content";
import { useSelectionStore } from "../../stores/selection";
import type { CtxEntry } from "../contextMenus";
import type { PickerTarget } from "../relationOps";
import {
  analyzePrompt,
  assignViaPicker,
  clearPrompts,
  copyPrompt,
  deletePromptDocuments,
  isPromptAnalyzableExt,
  newPromptDocuments,
  openPromptEditor,
  promptStateOfAsset,
  promptStateOfArchive,
} from "../promptOps";

// 右键卡的 picker 目标（origin）
function assetTarget(p: PlacementView): PickerTarget {
  return { kind: "asset", id: p.assetId, name: p.name, ext: p.ext, folderId: p.folderId, origin: p.origin };
}

// 全部选中资产的目标集（assetId 去重——同资产多条链接位置只指定一次；右键未选中项调用方已先单选，
// 选中集必含 origin）；批量写命令入参（folderId = 各自所在文件夹，新建文档落点随源）
function selectedAssetRefs(): { assetId: string; folderId: string }[] {
  const out: { assetId: string; folderId: string }[] = [];
  const seen = new Set<string>();
  for (const r of useSelectionStore().selectedRefs()) {
    if (seen.has(r.assetId)) continue;
    seen.add(r.assetId);
    out.push({ assetId: r.assetId, folderId: r.folderId });
  }
  return out;
}

// 选中资产转 picker 目标集（name/ext 供备选区与横幅显示；placements 查不到的兜底 id 占位）
function selectedAssetTargets(): PickerTarget[] {
  const content = useContentStore();
  return selectedAssetRefs().map((r) => {
    const p = content.placements.find((x) => x.assetId === r.assetId && x.folderId === r.folderId);
    return { kind: "asset" as const, id: r.assetId, name: p?.name ?? r.assetId, ext: p?.ext, folderId: r.folderId, origin: p?.origin };
  });
}

// —— 文件（素材卡片）——
// panelId = 发起右键的 panel（打开编辑器 / 新建后自动打开进属主 panel 预览会话，2026-09-25 per-panel 口径）
export function assetPromptEntries(p: PlacementView, panelId: string): CtxEntry[] {
  const state = promptStateOfAsset(p);
  const refs = selectedAssetRefs();
  const ids = refs.map((r) => r.assetId);
  const ref = { kind: "asset" as const, assetId: p.assetId };
  const single = useSelectionStore().count === 1;
  if (state === "linked") {
    return [
      // 打开编辑器 = 单目标语义（一次开一个预览）；源无活 placement 由 openPromptEditor 兜底 toast
      { label: "打开文本编辑器", disabled: !single, action: () => void openPromptEditor(ref, panelId) },
      { label: "复制提示词", action: () => void copyPrompt(ref) },
      { label: "断开提示词文档链接", action: () => void clearPrompts({ assets: refs, archives: [] }, "断开提示词文档链接") },
      { label: "删除提示词文档", action: () => void deletePromptDocuments({ assets: refs, archives: [] }) },
    ];
  }
  if (state === "scanned") {
    return [
      // 保存为文档 = 快照内容写盘并绑定（new_*_prompt_document 已扫描语义）
      { label: "保存提示词为 .md 文档", action: () => void newPromptDocuments({ assets: refs, archives: [] }, "md") },
      { label: "保存提示词为 .txt 文档", action: () => void newPromptDocuments({ assets: refs, archives: [] }, "txt") },
      { label: "复制提示词", action: () => void copyPrompt(ref) },
      { label: "重新分析提示词", action: () => void analyzePrompt(ids) },
      { label: "清空提示词", action: () => void clearPrompts({ assets: refs, archives: [] }, "清空提示词") },
    ];
  }
  // 未指定：分析仅图片格式可用（按右键卡判定；批量动作内部再逐个跳过不兼容项）
  return [
    { label: "分析提示词", disabled: !isPromptAnalyzableExt(p.ext), action: () => void analyzePrompt(ids) },
    { label: "指定文本文件为提示词", action: () => assignViaPicker(assetTarget(p), selectedAssetTargets()) },
    { label: "新建 .md 提示词文档", action: () => void newPromptDocuments({ assets: refs, archives: [] }, "md") },
    { label: "新建 .txt 提示词文档", action: () => void newPromptDocuments({ assets: refs, archives: [] }, "txt") },
  ];
}

// —— 档案（单目标；无分析——档案不是文件，走查原文）——
export function archivePromptEntries(archive: { id: string; name: string }): CtxEntry[] {
  const state = promptStateOfArchive(archive.id);
  const ref = { kind: "archive" as const, folderId: archive.id };
  const archives = [archive.id];
  if (state === "linked") {
    return [
      { label: "打开文本编辑器", action: () => void openPromptEditor(ref) },
      { label: "复制提示词", action: () => void copyPrompt(ref) },
      { label: "断开提示词文档链接", action: () => void clearPrompts({ assets: [], archives }, "断开提示词文档链接") },
      { label: "删除提示词文档", action: () => void deletePromptDocuments({ assets: [], archives }) },
    ];
  }
  if (state === "scanned") {
    // 档案无「分析」入口，正常到不了已扫描——防御性覆盖（数据异常态可保存 / 复制 / 清空自救）
    return [
      { label: "保存提示词为 .md 文档", action: () => void newPromptDocuments({ assets: [], archives }, "md") },
      { label: "保存提示词为 .txt 文档", action: () => void newPromptDocuments({ assets: [], archives }, "txt") },
      { label: "复制提示词", action: () => void copyPrompt(ref) },
      { label: "清空提示词", action: () => void clearPrompts({ assets: [], archives }, "清空提示词") },
    ];
  }
  return [
    {
      label: "指定文本文档为提示词",
      action: () => {
        const origin: PickerTarget = { kind: "archive", id: archive.id, name: archive.name };
        assignViaPicker(origin, [origin]);
      },
    },
    { label: "新建 .md 提示词文档", action: () => void newPromptDocuments({ assets: [], archives }, "md") },
    { label: "新建 .txt 提示词文档", action: () => void newPromptDocuments({ assets: [], archives }, "txt") },
  ];
}
