// 03 稿 · 素材卡片右键菜单 spec（自 menuSpecs.ts 拆出，规范第 3 条行数控制）：
// 菜单项按新稿纯文字口径（2026-09-01）。
import { invoke } from "@tauri-apps/api/core";
import { useContentStore, type FolderNode, type PlacementView } from "../../stores/content";
import { usePickerStore } from "../../stores/picker";
import { useSelectionStore } from "../../stores/selection";
import { useTabsStore } from "../../stores/tabs";
import { usePreviewStore } from "../../stores/preview";
import type { CtxEntry } from "../contextMenus";
import { isSequence as isSequenceAsset } from "../formatBadge";
import { jumpToAssetPlacement } from "../relationOps";
import {
  copyAssetLink,
  copyAssetPath,
  copyBase64,
  copyName,
  dissolveSequenceFlow,
  moveAssetsFlow,
  pickCustomThumb,
  pickExportAssets,
  revealWithWarning,
} from "../menuOps";
import {
  applyClipboardThumb,
  applyThumbLink,
  copySelection,
  copyThumb,
  copyThumbLink,
  deletePlacements,
  duplicateSelected,
  openWithDialog,
  pasteClipboard,
  pasteDuplicate,
  refreshThumb,
  restoreThumbAsset,
  sendToAnimate,
  setPlacementOrigin,
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
  // 选择模式（6.2 一轮，PRD 03_数据链接 04）：右键仅留查看类操作，编辑类不出现
  if (usePickerStore().active) return pickerAssetCardSpec(p);
  const content = useContentStore();
  const selection = useSelectionStore();
  const assetId = p.assetId;
  const assetKey = `${p.assetId}|${p.folderId}`;
  const isSequence = isSequenceAsset(p); // 序列帧（动图同 frameCount 字段，靠 animated 区分）
  const hasClipboard = selection.clipboard !== null;
  // 打开文件链接所在位置（6.2 四轮走查④接入）：同 assetId 的全部 Placement 所在文件夹去重
  //（含关联文件夹）；点击 = 跳转该位置视口 + 选中该卡片（同链接区块 item 跳转口径）
  const locations: CtxEntry[] = [
    ...new Set(content.placements.filter((q) => q.assetId === assetId).map((q) => q.folderId)),
  ].map((fid) => ({
    label: folderName(fid),
    action: () => jumpToAssetPlacement(assetId, fid),
  }));
  const entries: CtxEntry[] = [
    { label: "打开预览", shortcut: "Enter", action: () => void usePreviewStore().open(assetKey) },
  ];
  if (isSequence) {
    entries.push(
      // 打开序列帧包（5.2 包内视图）：视口切到包内卡片列表（seq:{assetId}），面包屑后退可返回；
      // 进包前清全局选择域——包内视口无素材卡片，残留选中会让全局 Delete 误删素材/文件夹
      {
        label: "打开序列帧包",
        action: () => {
          selection.clear();
          useTabsStore().openView(`seq:${assetId}`);
        },
      },
      // 解散序列帧包导入（5.2）：危险操作，alert warning 弹窗明示不可撤销（流程在 menuOps）
      { label: "解散序列帧包导入", action: () => dissolveSequenceFlow(p) },
    );
  }
  entries.push(
    {
      label: "新标签页打开",
      shortcut: "Ctrl+Enter",
      disabled: true, // 文件无独立标签页视口业务（标签页 = 文件夹视口），占位
    },
    { label: "新窗口打开", action: () => void invoke("open_preview_window", { assetKey }) },
    { label: "在默认应用打开", shortcut: "Shift+Enter", action: () => openDefaultApp(assetId) },
    { label: "在其他应用打开", action: () => void openWithDialog(assetId) },
    // 发送至 Animate（8.3，PRD 07-04 点亮）：仅 fla + 单选（多选发送记 09 待办）；
    // 面板未连接 / 在途重复由后端报错 toast（无队列不积压）
    { label: "发送至 Animate", shortcut: "Ctrl+E", disabled: selection.count !== 1 || p.ext !== "fla", action: () => void sendToAnimate(assetId) },
    { label: "在资源管理器打开", action: () => revealWithWarning(assetId) },
    "divider",
    { label: "移动文件", shortcut: "Ctrl+Y", action: () => moveAssetsFlow() },
    // 文件链接管理（17 稿，6.2 三轮点亮）：单选才可（面板 = 管理一个文件的链接分布）
    {
      label: "文件链接管理",
      shortcut: "Ctrl+J",
      disabled: selection.count !== 1,
      action: () => {
        selection.linkManageDraft = { assetId, name: p.name };
      },
    },
    // 将此文件设为原始文件（6.2 四轮走查③，PRD 01 边界规则 8 手动 origin 转移）：
    // 仅链接卡片（origin=false）可点——origin 卡已是原始文件；单选才可
    {
      label: "将此文件设为原始文件",
      disabled: selection.count !== 1 || p.origin,
      action: () => void setPlacementOrigin(assetId, p.folderId, p.name),
    },
    {
      label: "打开文件链接所在位置",
      submenu: locations.length > 0 ? locations : [{ label: "（无）", disabled: true }],
    },
    "divider",
    { label: "剪切文件", shortcut: "Ctrl+X", action: () => copySelection("cut") },
    { label: "复制文件", shortcut: "Ctrl+C", action: () => copySelection("copy") },
  );
  if (hasClipboard) {
    entries.push(
      { label: "作为链接粘贴", shortcut: "Ctrl+V", action: () => void pasteClipboard() },
      { label: "作为副本粘贴", shortcut: "Ctrl+Shift+V", action: () => void pasteDuplicate() },
    );
  }
  entries.push(
    {
      // 多选分流批量重命名（3.4，37 稿）：弹窗逐条改名；单选 = 卡片就地重命名（F2 同径）
      label: selection.count > 1 ? "批量重命名" : "重命名",
      shortcut: "F2",
      action: () => {
        if (selection.count > 1) selection.renameDraft = selection.selectedRefs();
        else selection.renamingKey = assetKey;
      },
    },
    "divider",
    { label: "创建副本", shortcut: "Ctrl+D", action: () => void duplicateSelected() },
    "divider",
    {
      label: "缩略图",
      submenu: [
        { label: "打开 Animate 生成缩略图", disabled: true },
        { label: "复制缩略图", disabled: selection.count !== 1, action: () => void copyThumb(assetId) },
        // 缩略图链接（批次 4 点亮）：转换 = 纯引用复制不动库数据；设置 = set_thumb_asset 绑定（hidden 随绑定打）
        { label: "转换为缩略图链接并复制", disabled: selection.count !== 1, action: () => copyThumbLink(assetId, p.name) },
        selection.thumbLink
          ? {
              label: `将「${selection.thumbLink.name}」设置为缩略图`,
              disabled: selection.thumbLink.assetId === assetId, // 自绑无意义（后端同护栏）
              action: () => void applyThumbLink(assetId),
            }
          : { label: "将「xx」设置为缩略图", disabled: true },
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
        // 还原为素材（PRD 02-15，批次 4）：仅 hidden 伴生缩略图资产显示（缩略图视图右键）
        ...(p.hidden
          ? [{ label: "还原为素材", action: () => void restoreThumbAsset(assetId) } as CtxEntry]
          : []),
      ],
    },
    "divider",
    {
      label: "添加至其他资源库",
      submenu: [{ label: "（暂无其他资源库）", disabled: true }],
    },
    {
      label: "导出文件",
      submenu: [
        { label: "导出为素材包", disabled: true },
        {
          label: "导出到本地磁盘",
          action: () => void pickExportAssets(selection.selectedRefs().map((r) => r.assetId)),
        },
        { label: "导出提示词为 .MD 文档", disabled: true },
      ],
    },
    "divider",
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
    { label: "保存提示词", disabled: true },
    {
      label: "复制数据",
      submenu: [
        { label: "复制链接", action: () => void copyAssetLink(assetId) },
        { label: "复制文件路径", action: () => void copyAssetPath(assetId) },
        "divider",
        { label: "复制名称", action: () => copyName(p.name) },
        { label: "复制 base64", action: () => copyBase64(assetId) },
        { label: "复制提示词", disabled: true },
      ],
    },
    "divider",
    { label: "置顶", disabled: true },
    "divider",
  );
  // 删除组两项恒显（3.4 八轮，2026-08-18 用户定：废除链接分流项——链接删除本就只移除记录，
  // 不进回收站不去未分类；从文件夹中移除 = 去未分类（"uncategorized"），丢到回收站 = "recycle"）
  entries.push(
    {
      label: "从文件夹中移除",
      shortcut: "Shift+Delete",
      action: () => void deletePlacements(selection.selectedRefs(), "uncategorized"),
    },
    {
      label: "丢到回收站",
      shortcut: "Delete",
      action: () => void deletePlacements(selection.selectedRefs()),
    },
  );
  return entries;
}

// 打开文件链接所在位置（子菜单条目组）：同 assetId 全部 Placement 所在文件夹去重——
// assetCardSpec 与选择模式精简集共用
function linkLocationEntries(assetId: string): CtxEntry[] {
  const locations: CtxEntry[] = [
    ...new Set(useContentStore().placements.filter((q) => q.assetId === assetId).map((q) => q.folderId)),
  ].map((fid) => ({
    label: folderName(fid),
    action: () => {
      useTabsStore().openView(`folder:${fid}`);
    },
  }));
  return locations.length > 0 ? locations : [{ label: "（无）", disabled: true }];
}

// 选择模式精简集（6.2 一轮，PRD 03_数据链接 04"右键仅留查看类操作，编辑类禁用"）：
// 预览 / 打开类 / 导航 / 复制（缩略图 / 数据）——均只读，不写库不动选择域
function pickerAssetCardSpec(p: PlacementView): CtxEntry[] {
  const assetId = p.assetId;
  const assetKey = `${p.assetId}|${p.folderId}`;
  const entries: CtxEntry[] = [
    { label: "打开预览", action: () => void usePreviewStore().open(assetKey) },
  ];
  if (isSequenceAsset(p)) {
    entries.push({
      label: "打开序列帧包",
      action: () => {
        useSelectionStore().clear();
        useTabsStore().openView(`seq:${assetId}`);
      },
    });
  }
  entries.push(
    { label: "新窗口打开", action: () => void invoke("open_preview_window", { assetKey }) },
    { label: "在默认应用打开", action: () => openDefaultApp(assetId) },
    { label: "在其他应用打开", action: () => void openWithDialog(assetId) },
    { label: "在资源管理器打开", action: () => revealWithWarning(assetId) },
    "divider",
    {
      label: "打开文件链接所在位置",
      submenu: linkLocationEntries(assetId),
    },
    "divider",
    { label: "复制缩略图", action: () => void copyThumb(assetId) },
    {
      label: "复制数据",
      submenu: [
        { label: "复制链接", action: () => void copyAssetLink(assetId) },
        { label: "复制文件路径", action: () => void copyAssetPath(assetId) },
        "divider",
        { label: "复制名称", action: () => copyName(p.name) },
        { label: "复制 base64", action: () => copyBase64(assetId) },
      ],
    },
  );
  return entries;
}
