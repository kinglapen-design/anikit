// 03 稿 · 素材卡片右键菜单 spec（自 menuSpecs.ts 拆出，规范第 3 条行数控制）：
// 宽 320 + 搜索框（openGridContextMenu 调用处传 opts）；
// 二级菜单宽度按稿（submenuWidth）：链接位置 320 / 缩略图·导出 213 / 资源库·AI·复制数据 180
//（2026-08-11 二轮走查，原默认 175 文字截断）。
import { invoke } from "@tauri-apps/api/core";
import { useContentStore, type FolderNode, type PlacementView } from "../../stores/content";
import { useSelectionStore } from "../../stores/selection";
import { useTabsStore } from "../../stores/tabs";
import { usePreviewStore } from "../../stores/preview";
import type { CtxEntry } from "../contextMenus";
import { isSequence as isSequenceAsset } from "../formatBadge";
import {
  copyAssetLink,
  copyAssetPath,
  copyBase64,
  copyName,
  moveAssetsFlow,
  pickCustomThumb,
  pickExportAssets,
  revealWithWarning,
} from "../menuOps";
import {
  applyClipboardThumb,
  copySelection,
  deletePlacements,
  duplicateSelected,
  openWithDialog,
  pasteClipboard,
  pasteDuplicate,
  refreshThumb,
} from "../placementOps";

// 默认应用打开（AssetCard / preview store 同款流程：成功后重拉最近使用）
function openDefaultApp(assetId: string): void {
  void invoke("open_with_default_app", { assetId }).then(() => useContentStore().refreshRecent());
}

// 文件夹名（id → 名称；root = 未分类）
function folderName(id: string): string {
  if (id === "root") return "未分类";
  const walk = (nodes: FolderNode[]): string | null => {
    for (const n of nodes) {
      if (n.id === id) return n.name;
      const hit = walk(n.children);
      if (hit) return hit;
    }
    return null;
  };
  return walk(useContentStore().folders) ?? id;
}

// —— 03 稿 · 素材卡片（右键未选中项调用方已先单选；批量项对全部选中项执行）——
// 上下文规则（稿原文）：无复制/剪切 → 三个粘贴项不出现；非序列帧 → 序列帧两项不出现；
// 链接文件删除组只出「从文件夹中移除文件链接」，非链接出「从文件夹中移除 / 丢到回收站」
//（稿 prose 另提及"去到未分类"，2026-08-11 用户确认无此项，不列）；
// 无提示词 → 「打开提示词编辑器」改「保存提示词」（提示词数据未落地，恒显示保存提示词占位，记 09 待办）
export function assetCardSpec(p: PlacementView): CtxEntry[] {
  const content = useContentStore();
  const selection = useSelectionStore();
  const assetId = p.assetId;
  const assetKey = `${p.assetId}|${p.folderId}`;
  const isSequence = isSequenceAsset(p); // 序列帧（动图同 frameCount 字段，靠 animated 区分）
  const hasClipboard = selection.clipboard !== null;
  // 打开文件链接所在位置：同 assetId 的全部 Placement 所在文件夹去重（含关联文件夹）
  const locations: CtxEntry[] = [
    ...new Set(content.placements.filter((q) => q.assetId === assetId).map((q) => q.folderId)),
  ].map((fid) => ({
    label: folderName(fid),
    action: () => {
      useTabsStore().openView(`folder:${fid}`);
    },
  }));
  const entries: CtxEntry[] = [
    { label: "打开预览", icon: "open-preview", shortcut: "Enter", action: () => void usePreviewStore().open(assetKey) },
  ];
  if (isSequence) {
    entries.push(
      { label: "打开序列帧包", icon: "open-sequence-pack", disabled: true },
      { label: "解散序列帧包导入", icon: "import-sequence-unpack", disabled: true },
    );
  }
  entries.push(
    {
      label: "新标签页打开",
      icon: "open-in-new-tab",
      shortcut: "Ctrl+Enter",
      disabled: true, // 文件无独立标签页视口业务（标签页 = 文件夹视口），占位
    },
    { label: "新窗口打开", icon: "open-new-window", action: () => void invoke("open_preview_window", { assetKey }) },
    { label: "在默认应用打开", icon: "open-default", shortcut: "Shift+Enter", action: () => openDefaultApp(assetId) },
    { label: "在其他应用打开", icon: "open-in-other-app", action: () => void openWithDialog(assetId) },
    { label: "发送至 Animate", icon: "send-to-animate", shortcut: "Ctrl+E", disabled: true },
    { label: "在资源管理器打开", icon: "open-in-explorer", action: () => revealWithWarning(assetId) },
    "divider",
    { label: "移动文件", icon: "move-file", shortcut: "Ctrl+Y", action: () => moveAssetsFlow() },
    { label: "文件链接管理", icon: "file-link-manage", shortcut: "Ctrl+J", disabled: true },
    {
      label: "打开文件链接所在位置",
      icon: "reveal-file-link",
      submenuWidth: 320,
      submenu: locations.length > 0 ? locations : [{ label: "（无）", disabled: true }],
    },
    "divider",
    { label: "剪切文件", icon: "cut-file", shortcut: "Ctrl+X", action: () => copySelection("cut") },
    { label: "复制文件", icon: "copy-file", shortcut: "Ctrl+C", action: () => copySelection("copy") },
  );
  if (hasClipboard) {
    entries.push(
      { label: "作为链接粘贴", icon: "paste-as-link", shortcut: "Ctrl+V", action: () => void pasteClipboard() },
      { label: "作为副本粘贴", icon: "paste-as-copy", shortcut: "Ctrl+Shift+V", action: () => void pasteDuplicate() },
    );
  }
  entries.push(
    {
      // 多选分流批量重命名（3.4，37 稿）：弹窗逐条改名；单选 = 卡片就地重命名（F2 同径）
      label: selection.count > 1 ? "批量重命名" : "重命名",
      icon: "rename",
      shortcut: "F2",
      action: () => {
        if (selection.count > 1) selection.renameDraft = selection.selectedRefs();
        else selection.renamingKey = assetKey;
      },
    },
    "divider",
    { label: "创建副本", icon: "duplicate", shortcut: "Ctrl+D", action: () => void duplicateSelected() },
    "divider",
    {
      label: "缩略图",
      icon: "thumbnail",
      submenuWidth: 213,
      submenu: [
        { label: "打开 Animate 生成缩略图", disabled: true },
        { label: "复制缩略图", disabled: true },
        { label: "转换为缩略图链接并复制", disabled: true },
        { label: "将「xx」设置为缩略图", disabled: true },
        "divider",
        {
          label: "自定义缩略图 - 从剪贴板",
          disabled: selection.count !== 1,
          action: () => void applyClipboardThumb(assetId),
        },
        {
          label: "自定义缩略图 - 选择文件",
          disabled: selection.count !== 1,
          action: () => void pickCustomThumb(assetId),
        },
        "divider",
        { label: "刷新缩略图", action: () => void refreshThumb(assetId) },
      ],
    },
    "divider",
    {
      label: "添加至其他资源库",
      icon: "add-to-library",
      submenuWidth: 180,
      submenu: [{ label: "（暂无其他资源库）", disabled: true }],
    },
    {
      label: "导出文件",
      icon: "export-file",
      submenuWidth: 213,
      submenu: [
        { label: "导出为素材包", icon: "library", disabled: true },
        {
          label: "导出到本地磁盘",
          icon: "export-to-disk",
          action: () => void pickExportAssets(selection.selectedRefs().map((r) => r.assetId)),
        },
        { label: "导出提示词为 .MD 文档", icon: "set-markdown-as-prompt", disabled: true },
      ],
    },
    "divider",
    {
      label: "AI 工具集",
      icon: "ai-tools",
      submenuWidth: 180,
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
    { label: "保存提示词", icon: "save-prompt", disabled: true },
    {
      label: "复制数据",
      icon: "copy-data",
      submenuWidth: 180,
      submenu: [
        { label: "复制链接", action: () => void copyAssetLink(assetId) },
        { label: "复制文件路径", action: () => void copyAssetPath(assetId) },
        "divider",
        { label: "复制名称", action: () => void copyName(p.name) },
        { label: "复制 base64", action: () => void copyBase64(assetId) },
        { label: "复制提示词", disabled: true },
      ],
    },
    "divider",
    { label: "置顶", icon: "window-pin-on", disabled: true },
    "divider",
  );
  // 删除组两项恒显（3.4 八轮，2026-08-18 用户定：废除链接分流项——链接删除本就只移除记录，
  // 不进回收站不去未分类；从文件夹中移除 = 去未分类（"uncategorized"），丢到回收站 = "recycle"）
  entries.push(
    {
      label: "从文件夹中移除",
      icon: "remove-from-folder",
      shortcut: "Shift+Delete",
      action: () => void deletePlacements(selection.selectedRefs(), "uncategorized"),
    },
    {
      label: "丢到回收站",
      icon: "drop-to-trash",
      shortcut: "Delete",
      action: () => void deletePlacements(selection.selectedRefs()),
    },
  );
  return entries;
}
