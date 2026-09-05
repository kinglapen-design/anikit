// 菜单栏「菜单」按钮 · 应用主菜单 spec（2026-09-05 接业务轮；稿 = 用户当场供稿截图，同日会话）：
// 根级 = 仓库 ▸ / 文件 ▸ / 查找 ▸ / 整理 ▸ │ 偏好设置 / 帮助 ▸ │ 关于 AniKit │ 退出（Alt+F4）。
// 三级级联（ContextMenu flyout 栈）：
// - 仓库→仓库 = 储存空间管理（多空间业务未立项，阶段九——新建/删除占位，列表仅当前空间）；
// - 仓库→资源库 = 库管理（新建/删除当前/合并 + 库列表切换）。库列表 = 纯对勾行（MenuItem checkable +
//   library 动态切换，2026-09-05 用户定稿口径）——旧 LibraryMenuItem 抓手排序 + 删除× 不进此菜单，
//   删除走同级「删除当前资源库」项；对勾随 currentLibrary 实时（渲染层 library 特判）；
// - 文件→新建 = 新建文档七项带品牌图标（newDocGroupEntries，05_文档图标）；
// - 查找→筛选器 / 以图找图 = 业务另轮（筛选 9 子稿 / 以图搜图未立项），全占位；
// - 整理 = 选中集操作（素材卡 / 档案卡域）。有既有业务的接线（同 assetCard / useGlobalKeys 口径），
//   无业务 disabled 占位（速查 §6）；回收站视口只读——编辑类项全 disabled（useGlobalKeys 同口径）。
import { invoke } from "@tauri-apps/api/core";
import { useContentStore, DEFAULT_BLOCK_ID, type PlacementView } from "../../stores/content";
import { usePickerStore } from "../../stores/picker";
import { useSelectionStore } from "../../stores/selection";
import { useTabsStore } from "../../stores/tabs";
import { useSearchStore } from "../../stores/search";
import { useUiStore } from "../../stores/ui";
import type { CtxEntry, CtxItem } from "../contextMenus";
import { currentLibrary, currentSpace, requestDeleteLibrary, requestLibrarySetup } from "../libraryOps";
import { newDocGroupEntries, scanEntries } from "./addMenu";
import {
  convertToArchiveFlow,
  createAndName,
  createArchiveAndName,
  currentLinkTarget,
  enterLinkPicker,
  moveAssetsFlow,
  pickCustomThumb,
  pickExportAssets,
  pickImportPaths,
} from "../menuOps";
import { openBlock } from "../treeExpansion";
import {
  applyClipboardThumb,
  applyThumbLink,
  copyThumbLink,
  setPlacementOrigin,
} from "../placementOps";
import { isThumbCapableExt } from "../formatBadge";
import { copyArchiveTags, copyAssetTags, pasteTags } from "../customDataOps";
import type { PickerTarget } from "../relationOps";

// —— 选中集解析（整理菜单 / 文件菜单的选中依赖项共用）——

// 单选素材卡（文件夹域 / 回收站视口不响应——编辑类只读口径，同 useGlobalKeys）
function singleAsset(): PlacementView | null {
  const selection = useSelectionStore();
  const content = useContentStore();
  if (selection.selectedFolders.size > 0 || selection.count !== 1 || content.isRecycleView) return null;
  return content.visiblePlacements.find((v) => `${v.assetId}|${v.folderId}` === selection.first) ?? null;
}

// 单选档案卡（FolderNode 判定走 content.archives map）
function singleArchive(): { id: string; name: string } | null {
  const selection = useSelectionStore();
  const content = useContentStore();
  if (selection.count > 0 || selection.selectedFolders.size !== 1 || content.isRecycleView) return null;
  const id = selection.orderedSelectedFolderIds()[0];
  if (!content.archives[id]) return null;
  return { id, name: content.folderName(id) ?? "档案" };
}

// 选择模式发起目标（添加关系 / 链接音频）：单选素材或单选档案
function pickerTarget(): PickerTarget | null {
  const a = singleAsset();
  if (a) return { kind: "asset", id: a.assetId, name: a.name, ext: a.ext, folderId: a.folderId, origin: a.origin };
  const ar = singleArchive();
  if (ar) return { kind: "archive", id: ar.id, name: ar.name };
  return null;
}

// 选中资产 id 去重集（导出 / 粘贴标签批量口径，同 assetCard）
function selectedAssetIds(): string[] {
  return [...new Set(useSelectionStore().selectedRefs().map((r) => r.assetId))];
}

// 选中档案 id 集（粘贴标签批量）
function selectedArchiveIds(): string[] {
  const content = useContentStore();
  return useSelectionStore().orderedSelectedFolderIds().filter((id) => !!content.archives[id]);
}

// 编辑描述（31 稿 DescEditDialog，ui.descEdit 草稿式）：文件 = get_asset_detail 拉现值；
// 档案 = content.archives 直读（copyArchiveDescriptions 同口径，无命令往返）
function openDescEdit(): void {
  const ui = useUiStore();
  const a = singleAsset();
  if (a) {
    void invoke<{ descriptions: { title: string; content: string }[] }>("get_asset_detail", { assetId: a.assetId })
      .then((d) => {
        ui.descEdit = { kind: "file", id: a.assetId, name: a.name, initial: (d.descriptions ?? []).map((x) => ({ ...x })) };
      })
      .catch((e) => console.warn("[appMenu] 编辑描述拉取失败：", e));
    return;
  }
  const ar = singleArchive();
  if (ar) {
    ui.descEdit = {
      kind: "archive",
      id: ar.id,
      name: ar.name,
      initial: (useContentStore().archives[ar.id]?.descriptions ?? []).map((x) => ({ ...x })),
    };
  }
}

// 链接管理 / 添加链接 / 链接位置三项（文件·整理两菜单同形，抽出共用）
function linkEntries(): CtxEntry[] {
  const a = singleAsset();
  const t = currentLinkTarget();
  return [
    {
      label: "文件/档案链接管理",
      shortcut: "Ctrl+J",
      disabled: !a,
      action: () => {
        if (a) useSelectionStore().linkManageDraft = { assetId: a.assetId, name: a.name };
      },
    },
    {
      label: "添加文件/档案链接",
      shortcut: "Ctrl+L",
      disabled: t === null,
      action: () => {
        if (t) enterLinkPicker(t);
      },
    },
    // 二级 = linkList 面板（LinkListRows，与卡片徽章 hover 弹窗同构；单选素材才有位置列表）
    { label: "打开文件/档案链接所在位置", disabled: !a, panel: a ? { kind: "linkList", assetId: a.assetId } : undefined },
  ];
}

// —— 仓库 ▸（储存空间 + 资源库两级管理）——
function repoEntries(libraryNames: string[]): CtxEntry[] {
  const space = currentSpace.name;
  // 多空间业务未立项（阶段九）：新建/删除占位，列表仅当前空间（对勾恒显、点击无操作 keepOpen）
  const spaceRows: CtxItem[] = space
    ? [{ label: space, checkable: true, checked: true, keepOpen: true }]
    : [{ label: "（当前无仓库）", disabled: true }];
  const libRows: CtxEntry[] =
    libraryNames.length > 0
      ? libraryNames.map((name) => ({
          label: name,
          library: true, // 点击行为在 ContextMenu 动态判定（切换 + 对勾实时），keepOpen = 切库可切回
          keepOpen: true,
          checkable: true,
        }))
      : [{ label: "（当前空间无资源库）", disabled: true }];
  return [
    {
      label: "仓库",
      submenu: [{ label: "新建仓库", disabled: true }, { label: "删除当前仓库", disabled: true }, "divider", ...spaceRows],
    },
    {
      label: "资源库",
      submenu: [
        { label: "新建资源库", action: () => requestLibrarySetup() },
        {
          label: "删除当前资源库",
          disabled: !currentLibrary.name,
          action: () => {
            // 删除目标 = 当前库（稿「删除当前资源库」）；无当前库不可点
            if (currentLibrary.name) requestDeleteLibrary(currentLibrary.name);
          },
        },
        { label: "合并资源库", disabled: true },
        "divider",
        ...libRows,
      ],
    },
    "divider",
    { label: "管理工作空间", disabled: true },
    { label: "清除缓存并重新加载", disabled: true },
  ];
}

// —— 文件 ▸（新建 / 导入 / 链接 / 导出）——
function fileEntries(): CtxEntry[] {
  const content = useContentStore();
  const folderId = content.currentFolderId;
  // 转换为档案：文件夹视口且非档案节点才可（档案再转换无语义）
  const convertible = folderId !== null && !content.archives[folderId];
  const hasAssets = selectedAssetIds().length > 0 && !content.isRecycleView;
  return [
    // 新建三级 = 新建文档七项带图标（2026-09-05 用户定：用 2026-09-05 入库的 05_文档图标七件）
    { label: "新建", submenu: newDocGroupEntries() },
    "divider",
    {
      label: "新建文件夹",
      shortcut: "Ctrl+Shift+N",
      action: () => {
        openBlock(DEFAULT_BLOCK_ID);
        void createAndName(null, null, DEFAULT_BLOCK_ID);
      },
    },
    {
      label: "新建子文件夹",
      shortcut: "Ctrl+Shift+Alt+N",
      disabled: folderId === null,
      action: () => {
        if (folderId) void createAndName(folderId, null);
      },
    },
    {
      label: "新建档案",
      shortcut: "Shift+N",
      action: () => {
        openBlock(DEFAULT_BLOCK_ID);
        void createArchiveAndName(null, null, DEFAULT_BLOCK_ID);
      },
    },
    {
      label: "转换为档案",
      shortcut: "Alt+N",
      disabled: !convertible,
      action: () => {
        if (convertible && folderId) convertToArchiveFlow(folderId);
      },
    },
    "divider",
    { label: "导入文件", action: () => void pickImportPaths("files") },
    { label: "导入文件夹", action: () => void pickImportPaths("folder") },
    { label: "导入链接", disabled: true },
    { label: "导入素材包", disabled: true },
    "divider",
    ...linkEntries(),
    "divider",
    { label: "添加至其他资源库", disabled: true },
    {
      label: "导出到计算机",
      disabled: !hasAssets,
      action: () => void pickExportAssets(selectedAssetIds()),
    },
    { label: "导出为素材包", disabled: true },
    { label: "导出为 CSV", disabled: true },
  ];
}

// —— 查找 ▸（搜索 / 筛选 / 扫描）——
function findEntries(): CtxEntry[] {
  // 筛选器 / 以图找图 = 业务另轮（筛选 9 子稿 + 后端筛选条件；以图搜图未立项）——全占位
  const scan = scanEntries(); // [扫描重复文件 ▸（三范围）, 扫描文件/档案链接 disabled]
  return [
    {
      label: "搜索",
      shortcut: "Ctrl+F",
      action: () => useSearchStore().requestFocus(useTabsStore().activeTabId),
    },
    { label: "全局搜索", disabled: true },
    "divider",
    {
      label: "筛选器",
      submenu: [
        { label: "重置所有筛选", disabled: true },
        "divider",
        { label: "颜色", disabled: true },
        { label: "时长", disabled: true },
        { label: "标签", disabled: true },
        { label: "扩展名", disabled: true },
        { label: "文件夹", disabled: true },
        { label: "链接状态", disabled: true },
        { label: "文件大小", disabled: true },
        { label: "日期", disabled: true },
      ],
    },
    {
      label: "以图找图",
      submenu: [
        { label: "资源库", disabled: true },
        "divider",
        { label: "Google", disabled: true },
        { label: "Bing", disabled: true },
        { label: "百度", disabled: true },
        { label: "搜狗", disabled: true },
      ],
    },
    scan[0], // 扫描重复文件 ▸（当前目录 / 全部 / 所选文件，范围禁用上下文在 scanEntries 内）
    scan[1], // 扫描文件/档案链接（占位）
  ];
}

// —— 整理 ▸（选中集操作；稿全列，无业务占位）——
function organizeEntries(): CtxEntry[] {
  const selection = useSelectionStore();
  const content = useContentStore();
  const a = singleAsset();
  const ar = singleArchive();
  const target = pickerTarget();
  const hasAssets = selectedAssetIds().length > 0 && !content.isRecycleView;
  // 将此文件设为原始文件：单选非 origin + 活位置 > 1（assetCard 同口径；主菜单恒列、不满足则 disabled）
  const liveCount = a ? content.placements.filter((q) => q.assetId === a.assetId).length : 0;
  const canSetOrigin = !!a && !a.origin && liveCount > 1;
  return [
    ...linkEntries(),
    "divider",
    { label: "添加关系", disabled: !target, action: () => target && usePickerStore().enterRelation(target) },
    "divider",
    { label: "添加音频", disabled: !target, action: () => target && usePickerStore().enterVoice(target) },
    "divider",
    { label: "添加缩略图", disabled: true },
    // 条件出现（2026-09-05 用户定，与卡片右键同步）：单选且格式不能作缩略图（非静态图/gif）不列——
    // 列了点了必报错，误导人；无选中时保留 disabled 占位（主菜单是固定设计清单，无格式可判）
    ...(a === null || isThumbCapableExt(a.ext)
      ? [{ label: "转为为缩略图链接并复制", disabled: !a, action: () => a && copyThumbLink(a.assetId, a.name) } as CtxEntry]
      : []),
    selection.thumbLink
      ? {
          label: `将「${selection.thumbLink.name}」设置为缩略图`,
          disabled: !a || selection.thumbLink.assetId === a.assetId, // 自绑无意义（后端同护栏）
          action: () => a && void applyThumbLink(a.assetId),
        }
      : { label: "将「文件」设置为缩略图", disabled: true },
    { label: "自定义缩略图 - 从剪贴板", disabled: !a, action: () => a && void applyClipboardThumb(a.assetId) },
    { label: "自定义缩略图 - 从本地磁盘", disabled: !a, action: () => a && void pickCustomThumb(a.assetId) },
    "divider",
    // 更换封面 = 档案封面设置组业务（CoverSettings 取景上下文），独立入口随档案轮接
    { label: "更换封面图 - 从本地磁盘", disabled: true },
    { label: "更换封面图 - 从资源库", disabled: true },
    "divider",
    // 提示词骨架（2026-09-05 走查口径：业务未立项全占位，三态切换随业务轮）
    {
      label: "提示词",
      submenu: [
        { label: "分析提示词", disabled: true },
        { label: "指定文本文件为提示词", disabled: true },
        { label: "新建 .md 提示词文档", disabled: true },
        { label: "新建 .txt 提示词文档", disabled: true },
      ],
    },
    "divider",
    { label: "添加标签", disabled: true },
    {
      label: "复制标签",
      disabled: !a && !ar,
      action: () => {
        if (a) void copyAssetTags(a.assetId);
        else if (ar) copyArchiveTags(ar.id);
      },
    },
    {
      label: "粘贴标签",
      disabled: !selection.copiedData.tags?.length,
      action: () => void pasteTags({ assets: selectedAssetIds(), archives: selectedArchiveIds() }),
    },
    { label: "清空标签", disabled: true },
    "divider",
    { label: "编辑描述", disabled: !a && !ar, action: openDescEdit },
    "divider",
    { label: "移动", shortcut: "Ctrl+Y", disabled: !hasAssets, action: () => moveAssetsFlow() },
    {
      label: "将所选设置为原始文件/文档",
      disabled: !canSetOrigin,
      action: () => a && void setPlacementOrigin(a.assetId, a.folderId, a.name),
    },
  ];
}

// —— 主菜单根级（稿全列）——
export function appMenuSpec(libraryNames: string[]): CtxEntry[] {
  return [
    { label: "仓库", submenu: repoEntries(libraryNames) },
    { label: "文件", submenu: fileEntries() },
    { label: "查找", submenu: findEntries() },
    { label: "整理", submenu: organizeEntries() },
    "divider",
    { label: "偏好设置", disabled: true },
    {
      label: "帮助",
      submenu: [
        { label: "使用教程", disabled: true },
        { label: "意见反馈", disabled: true },
        { label: "查看软件日志", disabled: true },
      ],
    },
    "divider",
    { label: "关于 AniKit", disabled: true },
    "divider",
    { label: "退出", shortcut: "Alt+F4", action: () => void invoke("exit_app") },
  ];
}
