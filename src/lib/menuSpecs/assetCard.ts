// 03 稿 · 素材卡片右键菜单 spec（自 menuSpecs.ts 拆出，规范第 3 条行数控制）：
// 菜单项按新稿纯文字口径（2026-09-01）。
// 2026-09-09 入口收窄（PRD 02-15）：虚拟视图（全部 / 最近使用 / 缩略图）分流
// virtualAssetCardSpec——位置级（移动 / 剪切 / 副本 / 从文件夹中移除）与落点类（粘贴 /
// 导入）不出现（不是位置，黑箱移动语义不成立）；留打开外发 / 链接管理导航 / 复制 / 重命名 /
// 缩略图组 / 导出 / 丢到回收站（删除 = 资产判断无需位置）；未立项占位项不列。
// 2026-09-05 走查「上下文菜单」文件节修订：
// - 缩略图二级重排分组：「打开 Animate 生成缩略图」独组 │ 转换为缩略图链接 / 将「xx」设为缩略图 一组
//   （占位文案「将「文件」设为缩略图」，有缩略图链接时正常显示复制的名称）│ 自定义两项 │
//   复制缩略图 + 刷新缩略图 一组（复制移到刷新上）；
// - 「将此文件设为原始文件」无链接状态（活位置 ≤1）不出现（旧"恒列置灰"废除）；
// - 「转换为缩略图链接并复制」格式不能作缩略图（非静态图/gif）不出现（2026-09-05 用户定，防误导）；
//   主菜单「整理」口径不同——恒列置灰（入口汇总需完整展示，同日三轮用户定，appMenu.ts）；
// - 「打开文件链接所在位置」二级 = linkList 面板（与卡片徽章 hover 弹窗 LinkListPopover 同构）；
// - 导出二级删「导出提示词为 .MD 文档」；复制数据删「复制提示词」、加「复制标签/复制描述」，
//   复制后「粘贴标签/粘贴描述」出现在一级「复制文件」同组下方（同日二轮修订：不放复制数据二级，
//   与粘贴文件项习惯一致；合并追加，多选批量）；
// - 「保存提示词」占位项改「提示词」一级菜单骨架（分析/指定/新建——业务未立项全占位，三态切换随业务轮）。
import { invoke } from "@tauri-apps/api/core";
import { useContentStore, type PlacementView } from "../../stores/content";
import { usePickerStore } from "../../stores/picker";
import { useSelectionStore } from "../../stores/selection";
import { useTabsStore } from "../../stores/tabs";
import { usePreviewStore } from "../../stores/preview";
import type { CtxEntry } from "../contextMenus";
import { isSequence as isSequenceAsset, isThumbCapableExt } from "../formatBadge";
import {
  copyAssetDescriptions,
  copyAssetTags,
  pasteDescriptions,
  pasteTags,
} from "../customDataOps";
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

// 粘贴标签/描述的目标资产集（多选批量）：当前选中集按 assetId 去重
function selectedAssetIds(): string[] {
  return [...new Set(useSelectionStore().selectedRefs().map((r) => r.assetId))];
}

// —— 03 稿 · 素材卡片（右键未选中项调用方已先单选；批量项对全部选中项执行）——
// 上下文规则（稿原文）：无复制/剪切 → 三个粘贴项不出现；非序列帧 → 序列帧两项不出现；
// 链接文件删除组只出「从文件夹中移除文件链接」，非链接出「从文件夹中移除 / 丢到回收站」
//（稿 prose 另提及"去到未分类"，2026-08-11 用户确认无此项，不列）；
// 提示词 = 一级菜单骨架（2026-09-05 走查重做；业务未立项全占位，记 09 待办）
export function assetCardSpec(p: PlacementView): CtxEntry[] {
  // 选择模式（6.2 一轮，PRD 03_数据链接 04）：右键仅留查看类操作，编辑类不出现
  if (usePickerStore().active) return pickerAssetCardSpec(p);
  const content = useContentStore();
  // 虚拟视图收窄（2026-09-09 入口收窄，PRD 02-15）：全部 / 最近使用 / 缩略图不是位置——
  // 位置级与落点类不出现，留资产级 / 所见即所得集（回收站走 recycleCardSpec 专属口径，不到这里）
  if (content.isVirtualView) return virtualAssetCardSpec(p);
  const selection = useSelectionStore();
  const assetId = p.assetId;
  const assetKey = `${p.assetId}|${p.folderId}`;
  const isSequence = isSequenceAsset(p); // 序列帧（动图同 frameCount 字段，靠 animated 区分）
  const hasClipboard = selection.clipboard !== null;
  // 活位置数（同 assetId 全部 Placement）：「将此文件设为原始文件」仅链接状态（>1）出现——
  // 2026-09-05 走查：没有链接状态的文件不出该项（旧"恒列置灰"废除）
  const liveCount = content.placements.filter((q) => q.assetId === assetId).length;
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
  );
  // 将此文件设为原始文件（6.2 四轮走查③，PRD 01 边界规则 8 手动 origin 转移）：
  // 仅链接卡片（origin=false）可点——origin 卡已是原始文件；单选才可；活位置 ≤1 不出现
  if (liveCount > 1) {
    entries.push({
      label: "将此文件设为原始文件",
      disabled: selection.count !== 1 || p.origin,
      action: () => void setPlacementOrigin(assetId, p.folderId, p.name),
    });
  }
  entries.push(
    // 打开文件链接所在位置（2026-09-05 走查：二级与徽章 hover 弹窗一致）=
    // linkList 面板（LinkListRows 双行行：区块/目录两段 + 显示名 + 徽章，点行跳转定位）
    { label: "打开文件链接所在位置", panel: { kind: "linkList", assetId } },
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
  // 粘贴标签/描述（2026-09-05 走查修订：不放复制数据二级——复制后直接跟在「复制文件」同组下方，
  // 与粘贴文件项的使用习惯一致；合并追加语义（按词/按标题去重）见 customDataOps）
  if (selection.copiedData.tags?.length) {
    entries.push({ label: "粘贴标签", action: () => void pasteTags({ assets: selectedAssetIds(), archives: [] }) });
  }
  if (selection.copiedData.descriptions?.length) {
    entries.push({ label: "粘贴描述", action: () => void pasteDescriptions({ assets: selectedAssetIds(), archives: [] }) });
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
        // 2026-09-05 走查分组：Animate 生成独组 │ 转换链接 / 设为缩略图 │ 自定义两项 │ 复制 + 刷新
        { label: "打开 Animate 生成缩略图", disabled: true },
        "divider",
        // 缩略图链接（批次 4 点亮）：转换 = 纯引用复制不动库数据；设置 = set_thumb_asset 绑定（hidden 随绑定打）
        // 条件出现（2026-09-05 用户定）：格式不能作缩略图（asset_menu.rs is_thumb_capable 口径：静态图 + gif）
        // 就不列「转换为缩略图链接并复制」——列了点了必报错，误导人
        ...(isThumbCapableExt(p.ext)
          ? [{ label: "转换为缩略图链接并复制", disabled: selection.count !== 1, action: () => copyThumbLink(assetId, p.name) } as CtxEntry]
          : []),
        selection.thumbLink
          ? {
              label: `将「${selection.thumbLink.name}」设为缩略图`,
              disabled: selection.thumbLink.assetId === assetId, // 自绑无意义（后端同护栏）
              action: () => void applyThumbLink(assetId),
            }
          : { label: "将「文件」设为缩略图", disabled: true },
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
        { label: "复制缩略图", disabled: selection.count !== 1, action: () => void copyThumb(assetId) },
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
        // 「导出提示词为 .MD 文档」占位删除（2026-09-05 走查）
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
    // 提示词（2026-09-05 走查重做，一级「提示词」+ 展开箭头）：业务未立项，本轮纯菜单骨架全占位——
    // 二级按态切换（未指定 = 分析/指定/新建；已指定 = 打开编辑器/复制/断开/删除；
    // 已扫描 = 保存 md/txt/复制/重新分析/清空）随业务轮接
    {
      label: "提示词",
      submenu: [
        { label: "分析提示词", disabled: true },
        { label: "指定文本文件为提示词", disabled: true },
        { label: "新建 .md 提示词文档", disabled: true },
        { label: "新建 .txt 提示词文档", disabled: true },
      ],
    },
    {
      label: "复制数据",
      submenu: [
        { label: "复制链接", action: () => void copyAssetLink(assetId) },
        { label: "复制文件路径", action: () => void copyAssetPath(assetId) },
        "divider",
        { label: "复制名称", action: () => copyName(p.name) },
        { label: "复制 base64", action: () => copyBase64(assetId) },
        "divider",
        // 复制标签/描述（2026-09-05 走查）：写 selection.copiedData 会话槽，复制后一级菜单多出对应粘贴项；
        // 「复制提示词」占位删除（提示词移一级菜单骨架）
        { label: "复制标签", action: () => void copyAssetTags(assetId) },
        { label: "复制描述", action: () => void copyAssetDescriptions(assetId) },
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

// 虚拟视图收窄集（2026-09-09 入口收窄，用户定，PRD 02-15）：全部 / 最近使用 / 缩略图 =
// 不是位置的视图——位置级（移动 / 剪切 / 副本 / 从文件夹中移除）与落点类（粘贴两项 /
// 导入）语义不成立（黑箱移动），一律不出现；留资产级与所见即所得项：打开外发全组 /
// 链接管理与位置导航 / 复制（文件 / 数据）/ 重命名（用户定：卡片上所见即所得——缩略图视口
// 改 hidden 伴生图名配工程名案例）/ 缩略图绑定组 / 导出 / 丢到回收站（删除 = 资产"要不要"
// 的判断，无需在乎位置；多位置条目右键哪条丢哪条）。业务未立项占位项（新标签页 /
// 添加资源库 / AI 工具集 / 提示词 / 置顶 / Animate 生成缩略图）快速访问不列。
function virtualAssetCardSpec(p: PlacementView): CtxEntry[] {
  const selection = useSelectionStore();
  const content = useContentStore();
  const assetId = p.assetId;
  const assetKey = `${p.assetId}|${p.folderId}`;
  // 活位置数（同主菜单口径）：「将此文件设为原始文件」仅链接状态（>1）出现
  const liveCount = content.placements.filter((q) => q.assetId === assetId).length;
  const entries: CtxEntry[] = [
    { label: "打开预览", shortcut: "Enter", action: () => void usePreviewStore().open(assetKey) },
  ];
  if (isSequenceAsset(p)) {
    entries.push(
      {
        label: "打开序列帧包",
        action: () => {
          selection.clear();
          useTabsStore().openView(`seq:${assetId}`);
        },
      },
      { label: "解散序列帧包导入", action: () => dissolveSequenceFlow(p) },
    );
  }
  entries.push(
    { label: "新窗口打开", action: () => void invoke("open_preview_window", { assetKey }) },
    { label: "在默认应用打开", shortcut: "Shift+Enter", action: () => openDefaultApp(assetId) },
    { label: "在其他应用打开", action: () => void openWithDialog(assetId) },
    // 发送至 Animate：仅 fla + 单选（同主菜单口径）
    { label: "发送至 Animate", shortcut: "Ctrl+E", disabled: selection.count !== 1 || p.ext !== "fla", action: () => void sendToAnimate(assetId) },
    { label: "在资源管理器打开", action: () => revealWithWarning(assetId) },
    "divider",
    {
      label: "文件链接管理",
      shortcut: "Ctrl+J",
      disabled: selection.count !== 1,
      action: () => {
        selection.linkManageDraft = { assetId, name: p.name };
      },
    },
  );
  if (liveCount > 1) {
    entries.push({
      label: "将此文件设为原始文件",
      disabled: selection.count !== 1 || p.origin,
      action: () => void setPlacementOrigin(assetId, p.folderId, p.name),
    });
  }
  entries.push(
    { label: "打开文件链接所在位置", panel: { kind: "linkList", assetId } },
    "divider",
    { label: "复制文件", shortcut: "Ctrl+C", action: () => copySelection("copy") },
  );
  // 粘贴标签/描述（资产级元数据，合并追加）——复制后直接跟在「复制文件」同组下方，同主菜单口径
  if (selection.copiedData.tags?.length) {
    entries.push({ label: "粘贴标签", action: () => void pasteTags({ assets: selectedAssetIds(), archives: [] }) });
  }
  if (selection.copiedData.descriptions?.length) {
    entries.push({ label: "粘贴描述", action: () => void pasteDescriptions({ assets: selectedAssetIds(), archives: [] }) });
  }
  entries.push(
    {
      label: selection.count > 1 ? "批量重命名" : "重命名",
      shortcut: "F2",
      action: () => {
        if (selection.count > 1) selection.renameDraft = selection.selectedRefs();
        else selection.renamingKey = assetKey;
      },
    },
    "divider",
    {
      label: "缩略图",
      submenu: [
        // 分组同主菜单（2026-09-05 走查口径）：转换链接 / 设为缩略图 │ 自定义两项 │ 复制 + 刷新；
        // 占位项「打开 Animate 生成缩略图」收窄集不列
        ...(isThumbCapableExt(p.ext)
          ? [{ label: "转换为缩略图链接并复制", disabled: selection.count !== 1, action: () => copyThumbLink(assetId, p.name) } as CtxEntry]
          : []),
        selection.thumbLink
          ? {
              label: `将「${selection.thumbLink.name}」设为缩略图`,
              disabled: selection.thumbLink.assetId === assetId, // 自绑无意义（后端同护栏）
              action: () => void applyThumbLink(assetId),
            }
          : { label: "将「文件」设为缩略图", disabled: true },
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
        { label: "复制缩略图", disabled: selection.count !== 1, action: () => void copyThumb(assetId) },
        { label: "刷新缩略图", action: () => void refreshThumb(assetId) },
        // 还原为素材：仅 hidden 伴生缩略图资产显示（缩略图视图右键）
        ...(p.hidden
          ? [{ label: "还原为素材", action: () => void restoreThumbAsset(assetId) } as CtxEntry]
          : []),
      ],
    },
    // 导出拍平一级（「导出为素材包」占位未立项，收窄集不列空壳二级）
    {
      label: "导出到本地磁盘",
      action: () => void pickExportAssets(selection.selectedRefs().map((r) => r.assetId)),
    },
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
      ],
    },
    "divider",
    {
      label: "丢到回收站",
      shortcut: "Delete",
      action: () => void deletePlacements(selection.selectedRefs()),
    },
  );
  return entries;
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
    // 同主菜单口径（2026-09-05）：二级 = linkList 面板（与徽章 hover 弹窗同构）
    { label: "打开文件链接所在位置", panel: { kind: "linkList", assetId } },
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
