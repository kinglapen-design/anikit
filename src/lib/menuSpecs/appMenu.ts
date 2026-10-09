// 菜单栏「菜单」按钮 · 应用主菜单 spec（2026-09-05 接业务轮；稿 = 用户当场供稿截图，同日会话）：
// 根级 = 仓库 ▸ / 文件 ▸ / 查找 ▸ / 整理 ▸ │ 偏好设置 / 帮助 ▸ │ 关于 AniKit │ 退出（Alt+F4）。
// 三级级联（ContextMenu flyout 栈）：
// - 仓库→仓库 = 储存空间管理（多空间业务未立项，阶段九——新建/删除占位，列表仅当前空间）；
// - 仓库→资源库 = 库管理（新建/删除当前/合并 + 库列表切换），spec 与侧栏资源库行切换钮共享
//   libraryMenuEntries（menuSpecs/libraryMenu，2026-09-07 提取）。库列表 = 纯对勾行（MenuItem checkable，
//   2026-09-05 用户定稿口径）——08 稿 LibraryMenuItem 抓手排序 + 删除× 全库废除（2026-09-07 用户：
//   资源库不至于多到需要排序），删除走同级「删除当前资源库」项；对勾随 currentLibrary 实时（函数化求值）；
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
import { currentSpace } from "../libraryOps";
import { libraryMenuEntries } from "./libraryMenu";
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
import { copyArchiveTags, copyAssetTags, clearTags, pasteTags } from "../customDataOps";
import { assetPromptEntries, archivePromptEntries } from "./promptMenu";
import type { PickerTarget } from "../relationOps";

// —— 选中集解析（整理菜单 / 文件菜单的选中依赖项共用）——

// 单选素材卡（文件夹域 / 回收站视口不响应——编辑类只读口径，同 useGlobalKeys）；
// 工作空间视口（PRD 02 步骤 1）同样不响应——ws 合成项 assetId = `ws:` 假 id，严禁发 assetId 类命令
function singleAsset(): PlacementView | null {
  const selection = useSelectionStore();
  const content = useContentStore();
  if (selection.selectedFolders.size > 0 || selection.count !== 1 || content.isRecycleView) return null;
  if (content.isWorkspaceView) return null;
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

// 添加标签（32 稿属性标签弹窗 PropsDialog，ui.propsEdit 草稿式）：文件 = get_asset_detail 拉现值；
// 档案 = content.archives 直读（copyArchiveTags 同口径）。单目标面板——多选禁用
function openPropsEdit(): void {
  const ui = useUiStore();
  const a = singleAsset();
  if (a) {
    void invoke<{ tags: string[] }>("get_asset_detail", { assetId: a.assetId })
      .then((d) => {
        ui.propsEdit = { kind: "file", id: a.assetId, name: a.name, initial: [...(d.tags ?? [])] };
      })
      .catch((e) => console.warn("[appMenu] 属性标签拉取失败：", e));
    return;
  }
  const ar = singleArchive();
  if (ar) {
    ui.propsEdit = { kind: "archive", id: ar.id, name: ar.name, initial: [...(useContentStore().archives[ar.id]?.props ?? [])] };
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
  return [
    {
      label: "仓库",
      submenu: [{ label: "新建仓库", disabled: true }, { label: "删除当前仓库", disabled: true }, "divider", ...spaceRows],
    },
    // 资源库 ▸ = 共享 spec（menuSpecs/libraryMenu，2026-09-07 提取）：侧栏资源库行切换钮同一内容
    { label: "资源库", submenu: libraryMenuEntries(libraryNames) },
    "divider",
    // 管理工作空间（2026-10-02 落地点亮）：链接进来 / 链出去双向总览面板，宿主 App.vue
    { label: "管理工作空间", action: () => void (useUiStore().wsManageOpen = true) },
    { label: "清除缓存并重新加载", disabled: true },
  ];
}

// —— 文件 ▸（新建 / 导入 / 链接 / 导出）——
function fileEntries(): CtxEntry[] {
  const content = useContentStore();
  const folderId = content.currentFolderId;
  // 转换为档案：文件夹视口且非档案节点才可（档案再转换无语义）
  const convertible = folderId !== null && !content.archives[folderId];
  // 工作空间视口（PRD 02 步骤 1）：ws 合成项 assetId = `ws:` 假 id 严禁发 assetId 类命令——导出同禁
  const hasAssets = selectedAssetIds().length > 0 && !content.isRecycleView && !content.isWorkspaceView;
  // 禁导入视口（全部/最近使用/自动导入/缩略图/回收站，isNoImportView 统一判定）新建文档组禁用
  //（2026-09-08 用户定：四视口 = 数据自动维护；2026-09-09 入口收窄扩「全部」——全部不是位置，
  // 落点必借道。未分类 = 候车区总入口可新建文档，落点 root 所见即所得）
  const noExtra = content.isNoImportView;
  // 新建文件夹 / 档案：六个快速访问视口一律禁（2026-09-09 用户定：快速访问无文件夹概念，
  // 建到根级视图里也看不见）；新建子文件夹 / 转换为档案本随 folderId 禁用不重复接
  const noFolder = content.isQuickView;
  return [
    // 新建三级 = 新建文档七项带图标（2026-09-05 用户定：用 2026-09-05 入库的 05_文档图标七件）
    { label: "新建", disabled: noExtra, submenu: newDocGroupEntries() },
    "divider",
    {
      label: "新建文件夹",
      shortcut: "Ctrl+Shift+N",
      disabled: noFolder,
      action: () => {
        // 文件夹视口内 = 建到当前文件夹（2026-09-19 BUG 1 修订，同 addMenu 口径）；
        // 其余位置视图保持原口径 = 默认区块根级
        const cur = content.currentFolderId;
        if (cur !== null) void createAndName(cur, null);
        else {
          openBlock(DEFAULT_BLOCK_ID);
          void createAndName(null, null, DEFAULT_BLOCK_ID);
        }
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
      disabled: noFolder,
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
    // 只读视口（全部/最近使用/自动导入/缩略图/回收站，isNoImportView 统一判定）禁用导入——
    // 2026-09-08 用户定，与右键收窄 / 拖拽拒入 / Ctrl+V 拒入同口径；2026-09-09 收窄扩「全部」；
    // 菜单项保留置灰不消失。未分类 = 候车区总入口可导入
    { label: "导入文件", disabled: content.isNoImportView, action: () => void pickImportPaths("files") },
    { label: "导入文件夹", disabled: content.isNoImportView, action: () => void pickImportPaths("folder") },
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
// 筛选器子菜单（2026-10-09 筛选业务五轮点亮）：维度项 = 悬浮栏展开筛选组并打开对应弹窗
// （requestFilterPop → FloatBarHost；与按钮组 10 项同序同名）；「重置所有筛选」= clearAllFilters，
// 无生效筛选时禁用；ws 视口无筛选能力整组禁用（悬浮栏筛选钮同口径不渲染）
const FILTER_MENU_DIMS = ["颜色", "时长", "属性", "扩展名", "文件夹", "比例", "链接", "大小", "添加日期", "修改日期"] as const;
function findEntries(): CtxEntry[] {
  // 以图找图 = 业务另轮（未立项）——占位
  const scan = scanEntries(); // [扫描重复文件 ▸（三范围）, 扫描文件/档案链接 disabled]
  const panelId = useTabsStore().activeTabId;
  const viewKey = useContentStore().sidebarSelected;
  const canFilter = !!panelId && !viewKey.startsWith("ws:");
  const hasFilters = canFilter && useSearchStore().activeFilterDims(panelId).length > 0;
  return [
    {
      label: "搜索",
      shortcut: "Ctrl+F",
      action: () => useSearchStore().requestFocus(useTabsStore().activeTabId),
    },
    // 全局搜索（2026-09-05 用户定）：= 聚焦搜索框 + 帮用户输入 `~`（后端前缀协议全局范围），就这么简单
    {
      label: "全局搜索",
      action: () => useSearchStore().prefillGlobal(useTabsStore().activeTabId, useContentStore().sidebarSelected),
    },
    "divider",
    {
      label: "筛选器",
      submenu: [
        {
          label: "重置所有筛选",
          disabled: !hasFilters,
          action: () => useSearchStore().clearAllFilters(panelId, viewKey),
        },
        "divider",
        ...FILTER_MENU_DIMS.map((dim) => ({
          label: dim as string,
          disabled: !canFilter,
          action: () => useSearchStore().requestFilterPop(panelId, dim),
        })),
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

// 提示词组（2026-09-05 走查骨架 → 提示词轮接业务）：三态二级 = menuSpecs/promptMenu 共享构建器。
// 主菜单口径 = 无选中目标时整组 disabled 恒列（入口汇总需完整展示，同缩略图组 2026-09-05 三轮定）；
// 档案态读 content.archives——读不到档案数据（list_archives 未返回）时 singleArchive 已拦 null，
// 档案选中也落整组 disabled
function promptMenuEntries(): CtxEntry[] {
  const a = singleAsset();
  if (a) return assetPromptEntries(a, useTabsStore().activeTabId);
  const ar = singleArchive();
  if (ar) return archivePromptEntries(ar);
  return [
    { label: "分析提示词", disabled: true },
    { label: "指定文本文件为提示词", disabled: true },
    { label: "新建 .md 提示词文档", disabled: true },
    { label: "新建 .txt 提示词文档", disabled: true },
  ];
}

// —— 整理 ▸（选中集操作；稿全列，无业务占位）——
function organizeEntries(): CtxEntry[] {
  const selection = useSelectionStore();
  const content = useContentStore();
  const a = singleAsset();
  const ar = singleArchive();
  const target = pickerTarget();
  const hasAssets = selectedAssetIds().length > 0 && !content.isRecycleView && !content.isWorkspaceView;
  // 标签可写目标（清空标签）：选中集含文件或档案且非回收站（回收站只读不开编辑类，useGlobalKeys 同口径）；
  // 工作空间视口（PRD 02 步骤 1）同禁——ws 合成假 id 严禁发 assetId 类命令
  const hasTaggable =
    (selectedAssetIds().length > 0 || selectedArchiveIds().length > 0) && !content.isRecycleView && !content.isWorkspaceView;
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
    // 主菜单恒列（2026-09-05 三轮用户定：主菜单 = 入口汇总需完整展示，与右键菜单的条件出现不同）——
    // 无单选或格式不能作缩略图（非静态图/gif）时 disabled 置灰
    { label: "转为为缩略图链接并复制", disabled: !a || !isThumbCapableExt(a.ext), action: () => a && copyThumbLink(a.assetId, a.name) },
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
    // 提示词（提示词轮接业务）：单选文件/档案时按态出三态二级；无选中目标整组 disabled 恒列
    {
      label: "提示词",
      submenu: promptMenuEntries(),
    },
    "divider",
    // 添加标签 = 32 稿属性标签弹窗（单目标面板，多选禁用；2026-09-05 用户定接线）
    { label: "添加标签", disabled: !a && !ar, action: openPropsEdit },
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
      // 工作空间视口（PRD 02 步骤 1）同禁：选中集为 ws 合成假 id，严禁发 assetId 类命令
      disabled: !selection.copiedData.tags?.length || content.isWorkspaceView,
      action: () => void pasteTags({ assets: selectedAssetIds(), archives: selectedArchiveIds() }),
    },
    // 清空标签 = 所选项（文件 + 档案，多选批量）标签全删（2026-09-05 用户定）；回收站只读不开
    {
      label: "清空标签",
      disabled: !hasTaggable,
      action: () => void clearTags({ assets: selectedAssetIds(), archives: selectedArchiveIds() }),
    },
    "divider",
    { label: "编辑描述", disabled: !a && !ar, action: openDescEdit },
    "divider",
    // 移动 = 位置级操作：虚拟视图（全部/最近使用/缩略图）禁——不是位置，移动 = 黑箱
    //（2026-09-09 入口收窄，isVirtualView 统一判定；回收站只读本随 hasAssets 禁）
    { label: "移动", shortcut: "Ctrl+Y", disabled: !hasAssets || content.isVirtualView, action: () => moveAssetsFlow() },
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
    {
      label: "偏好设置",
      // 2026-09-20 退役占位：弹窗唯一内容（MCP 写入开关）已挪插件面板 MCP 详情窗
      //（plugins/McpPluginWindow，settings.mcp 子树），空壳弹窗删除；后续设置项立项再点亮
      disabled: true,
    },
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
