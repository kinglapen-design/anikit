// 13 稿 · 回收站右键菜单 spec（自 menuSpecs.ts 拆出，规范第 3 条行数控制）：菜单项按新稿纯文字口径（2026-09-01）。
// 2026-09-05 走查「回收站」节扩充（用户定）：
// - 文件菜单扩至全量（还原 / 打开类六件 / 移动·原始文件·链接位置 / 剪切·重命名 / 添加至其他资源库·
//   导出·AI 工具集 / 复制数据 / 彻底删除·清空）——「彻底删除文件」自此入菜单（旧"不入菜单"按稿口径废除）；
// - 回收站条目 = recycle.json 快照（folderId 恒 "recycle" 伪值），后端对快照的写操作零支持——
//   「移动文件 / 将此文件设为原始文件 / 剪切 / 重命名」四项按用户定列全但 disabled 占位，
//   回收站写能力随"档案进回收站"一轮后端统一立项（记 09 待办）；
// - 空白处 = 扫描两项（在回收站就是为了清理）+ 还原全部 / 清空。
import { invoke } from "@tauri-apps/api/core";
import { useContentStore, type PlacementView } from "../../stores/content";
import { useSelectionStore } from "../../stores/selection";
import { previewSession } from "../../stores/preview";
import { useTabsStore } from "../../stores/tabs";
import type { CtxEntry } from "../contextMenus";
import { copyAssetDescriptions, copyAssetTags, pasteDescriptions, pasteTags } from "../customDataOps";
import { copyAssetLink, copyAssetPath, copyBase64, copyName, pickExportAssets, revealWithWarning } from "../menuOps";
import { openPreviewWindow } from "../previewOps";
import {
  clearRecycle,
  deleteRecycle,
  openWithDialog,
  recycleSelectedAssetIds,
  restoreRecycle,
  sendToAnimate,
} from "../placementOps";
import { scanEntries } from "./addMenu";

// —— 13 稿 · 回收站 ——
// 空白 = 扫描重复文件 / 扫描文件/档案链接 / 还原全部文件 / 清空回收站（空时后两项禁用）；
// 扫描两项 = 新增菜单共用 scanEntries（当前目录项在回收站语境天然禁用——无当前文件夹）。
export function recycleBlankSpec(): CtxEntry[] {
  const content = useContentStore();
  return [
    ...scanEntries(),
    "divider",
    {
      label: "还原全部文件",
      disabled: content.recycleEntries.length === 0,
      action: () => void restoreRecycle([...new Set(content.recycleEntries.map((e) => e.assetId))]),
    },
    {
      label: "清空回收站",
      disabled: content.recycleEntries.length === 0,
      action: () => void clearRecycle(),
    },
  ];
}

// 默认应用打开（同 assetCard 口径：成功后重拉最近使用）
function openDefaultApp(assetId: string): void {
  void invoke("open_with_default_app", { assetId }).then(() => useContentStore().refreshRecent());
}

// 回收站文件菜单（2026-09-05 走查全量版）：p = 被右键条目（PlacementView 同构，folderId 恒 "recycle"）；
// 键解析失败兜底（p 缺失）= 只留还原/清空基础集（旧三行口径）；
// panelId = 发起右键的 panel（2026-09-25 per-panel 会话化：「打开预览」开进属主 panel 会话）
export function recycleCardSpec(p: PlacementView | undefined, panelId: string): CtxEntry[] {
  const selection = useSelectionStore();
  if (!p) {
    return [
      { label: "还原文件", action: () => void restoreRecycle(recycleSelectedAssetIds()) },
      "divider",
      ...recycleBlankSpec(),
    ];
  }
  const assetId = p.assetId;
  const assetKey = `${p.assetId}|${p.folderId}`;
  return [
    { label: "还原文件", action: () => void restoreRecycle(recycleSelectedAssetIds()) },
    "divider",
    { label: "打开预览", shortcut: "Enter", action: () => void previewSession(panelId).open(assetKey) },
    {
      label: "新标签页打开",
      shortcut: "Ctrl+Enter",
      action: () => useTabsStore().openPreviewInNewTab(panelId, assetKey), // 2026-09-25 per-panel 会话化接入
    },
    { label: "新窗口打开", action: () => void openPreviewWindow(assetKey) },
    { label: "在默认应用打开", shortcut: "Shift+Enter", action: () => openDefaultApp(assetId) },
    { label: "在其他应用打开", action: () => void openWithDialog(assetId) },
    // 发送至 Animate（同 03 稿口径：仅 fla + 单选）——回收站场景 = 发送查看确认
    {
      label: "发送至 Animate",
      shortcut: "Ctrl+E",
      disabled: selection.count !== 1 || p.ext !== "fla",
      action: () => void sendToAnimate(assetId),
    },
    { label: "在资源管理器打开", action: () => revealWithWarning(assetId) },
    "divider",
    // 回收站写能力占位四件（后端 recycle 快照写操作另轮统一立项，2026-09-05 用户定列全占位）：
    // 移动文件 = 还原到新位置（不回原始位置的恢复方式）；设为原始文件 = 原地转换 + 删全部活链接（走查心智模型）
    { label: "移动文件", shortcut: "Ctrl+Y", disabled: true },
    { label: "将此文件设为原始文件", disabled: true },
    // 打开文件链接所在位置 = linkList 面板（与徽章 hover 弹窗同构；只读反查活位置，回收站条目本体在站时
    // 库内可能只剩链接或为空——空态「（无）」）
    { label: "打开文件链接所在位置", panel: { kind: "linkList", assetId } },
    "divider",
    { label: "剪切", shortcut: "Ctrl+X", disabled: true },
    { label: "重命名", shortcut: "F2", disabled: true },
    "divider",
    { label: "添加至其他资源库", submenu: [{ label: "（暂无其他资源库）", disabled: true }] },
    {
      label: "导出文件",
      submenu: [
        { label: "导出为素材包", disabled: true },
        // 删之前手动导一份（走查）：导出选中集（回收站条目按 assetId 去重）
        { label: "导出到本地磁盘", action: () => void pickExportAssets(recycleSelectedAssetIds()) },
      ],
    },
    {
      label: "AI 工具集",
      submenu: [
        { label: "以图搜图", disabled: true },
        "divider",
        { label: "超清放大", disabled: true },
        { label: "自动抠图", disabled: true },
        { label: "裁剪", disabled: true },
        "divider",
        { label: "分析文件向量", disabled: true },
        "divider",
        { label: "引用至 AI 对话框", disabled: true },
      ],
    },
    "divider",
    {
      label: "复制数据",
      submenu: [
        { label: "复制链接", action: () => void copyAssetLink(assetId) },
        { label: "复制文件路径", action: () => void copyAssetPath(assetId) },
        "divider",
        { label: "复制名称", action: () => copyName(p.name) },
        { label: "复制 base64", action: () => copyBase64(assetId) },
        "divider",
        { label: "复制标签", action: () => void copyAssetTags(assetId) },
        { label: "复制描述", action: () => void copyAssetDescriptions(assetId) },
        // 粘贴目标 = 回收站选中集（合并追加，同 03 稿口径）
        ...(selection.copiedData.tags?.length
          ? [
              "divider" as CtxEntry,
              { label: "粘贴标签", action: () => void pasteTags({ assets: recycleSelectedAssetIds(), archives: [] }) } as CtxEntry,
            ]
          : []),
        ...(selection.copiedData.descriptions?.length
          ? [
              ...(selection.copiedData.tags?.length ? [] : ["divider" as CtxEntry]),
              { label: "粘贴描述", action: () => void pasteDescriptions({ assets: recycleSelectedAssetIds(), archives: [] }) } as CtxEntry,
            ]
          : []),
      ],
    },
    "divider",
    // 彻底删除（两段式确认业务在 placementOps；回收站视口 Del 键同入口）——2026-09-05 起入菜单
    { label: "彻底删除文件", shortcut: "Delete", action: () => void deleteRecycle(recycleSelectedAssetIds()) },
    {
      label: "清空回收站",
      disabled: useContentStore().recycleEntries.length === 0,
      action: () => void clearRecycle(),
    },
  ];
}
