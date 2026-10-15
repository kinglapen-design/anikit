// 07 稿 · 新增菜单 spec（自 menuSpecs.ts 拆出，规范第 3 条行数控制）：菜单项按新稿纯文字口径（2026-09-01）。
// 新建文档组（newDocGroupEntries）与 14 汉堡"文件"二级共用，故同文件导出。
// 2026-09-05 走查「上下文菜单」修订：
// - 新建文档七项带图标（icon 字段，05_文档图标；同菜单其余项由渲染层扫描自动占位对齐）；
// - 快速访问空白处收窄（gridDispatch 分流调用）：未分类 = quickFullSpec（去新建文件夹/新建档案——
//   快速访问无文件夹概念；留导入 / 新建文档 / 粘贴，2026-09-09 收窄定：未分类 = 候车区总入口）；
//   全部 / 最近使用 / 自动导入 / 缩略图 = scanOnlySpec（只给扫描两项——09-05 走查：新建文档在这里
//   污染严重，也不允许导入；09-09 收窄：全部不是位置，随虚拟视图同档）。
import { useSelectionStore } from "../../stores/selection";
import { useContentStore, DEFAULT_BLOCK_ID } from "../../stores/content";
import type { CtxEntry } from "../contextMenus";
import { createAndName, createArchiveAndName, currentLinkTarget, enterLinkPicker, pickImportPaths } from "../menuOps";
import { openBlock } from "../treeExpansion";
import { createDocument, pasteClipboard, pasteDuplicate, type NewDocKind } from "../placementOps";
import { openScan } from "../dupScanOps";
import { openLinkScan } from "../linkScanOps";

// —— 素材卡片空白处（废除，2026-08-11：空白右键统一走 07 新增菜单；
// 布局方式/排序/显示简介/分组依据归 28 查看方式菜单 ViewOptionsMenu）——

// —— 新建文档组（07 稿 / 14 汉堡"文件"二级共用）——
// 2026-08-15 用户定案：放弃 shell 检测模式（detect_doc_apps 废除），固定七项恒显：
// Animate / Photoshop / Illustrator / MarkDown / 文本 / Word / Excel；
// AE 不建档（工程素材为链接路径、需打包工程，属工作空间而非资源库）。
// 2026-09-05 走查「新增」节：七项带品牌图标（05_文档图标；右键菜单-带图标稿——图标位 18 盒 svg 14，
// 同菜单无图标项占位对齐由渲染层 scanPads 处理）。
export function newDocGroupEntries(): CtxEntry[] {
  const doc = (label: string, kind: NewDocKind, icon: string): CtxEntry => ({
    label,
    icon,
    action: () => void createDocument(kind),
  });
  return [
    doc("Animate 文档", "fla", "animate"),
    doc("Photoshop 文档", "psd", "photoshop"),
    doc("Illustrator 文档", "ai", "illustrator"),
    "divider", // 工程文档 / 文本·办公文档分组线（2026-08-15 用户定）
    doc("MarkDown 文档", "md", "markdown"),
    doc("文本文档", "txt", "txt"),
    doc("Word 文档", "docx", "word"),
    doc("Excel 文档", "xlsx", "excel"),
  ];
}

// —— 扫描两项（新增菜单 / 快速访问收窄菜单 / 回收站空白菜单共用，2026-09-05 抽出）——
// 扫描重复文件范围上下文（2.7 二轮点亮）：当前目录 = 文件夹视图才有；所选文件 = 选中资产去重集；
// 扫描文件/档案链接（2026-10-16 立项点亮，PRD 03_查找文件-04）：全库单范围无二级
export function scanEntries(): CtxEntry[] {
  const selection = useSelectionStore();
  const content = useContentStore();
  const scanFolderId = content.currentFolderId;
  const scanAssetIds = [...new Set(selection.selectedRefs().map((r) => r.assetId))];
  return [
    {
      label: "扫描重复文件",
      submenu: [
        // 三范围（锚点过滤：组内任一资产命中整组返回，PRD 03_查找文件-03）；
        // 快速访问视图无当前文件夹 / 无选中素材时对应项禁用
        {
          label: "当前文件夹",
          disabled: scanFolderId === null,
          action: scanFolderId === null ? undefined : () => void openScan({ kind: "folder", folderId: scanFolderId }),
        },
        { label: "全部", action: () => void openScan({ kind: "all" }) },
        {
          label: "所选文件",
          disabled: scanAssetIds.length === 0,
          action: scanAssetIds.length === 0 ? undefined : () => void openScan({ kind: "assets", assetIds: scanAssetIds }),
        },
      ],
    },
    { label: "扫描文件/档案链接", action: () => void openLinkScan() },
  ];
}

// —— 07 稿 · 新增菜单 ——
// 三入口共用一套（2026-08-11 用户定）：侧栏"新增"按钮 / 网格空白处右键 / Ctrl+N（按钮位弹出）。
// 上下文规则：剪贴板无文件 → 三个粘贴项不出现（同 03 稿口径，2026-08-11 用户定）；
// 新建文档组 = 固定七项恒显（2026-08-15 用户定案放弃 shell 检测动态显隐，见 newDocGroupEntries）。
// includeCreate（2026-09-05 走查）：快速访问视口空白处传 false——新建文件夹/新建档案在快速访问里没用。
export function addButtonSpec(opts?: { includeCreate?: boolean }): CtxEntry[] {
  const includeCreate = opts?.includeCreate ?? true;
  const selection = useSelectionStore();
  const hasClipboard = selection.clipboard !== null;
  const entries: CtxEntry[] = [];
  if (hasClipboard) {
    entries.push(
      { label: "作为链接粘贴", shortcut: "Ctrl+V", action: () => void pasteClipboard() },
      { label: "作为副本粘贴", shortcut: "Ctrl+Shift+V", action: () => void pasteDuplicate() },
      "divider",
    );
  }
  if (includeCreate) {
    entries.push(
      // 新建文件夹入口（2026-09-19 BUG 1 修订）：文件夹视口内 = 建到当前文件夹（用户心智 =
      // 文件管理器"在这里新建"，原恒建根级被报 BUG）；其余视口保持原口径 = 默认区块「目录」
      // 根级（2026-08-14 区块子系统），区块折叠时先强制展开让新节点可见
      {
        label: "新建文件夹",
        shortcut: "Ctrl+Shift+N",
        action: () => {
          const cur = useContentStore().currentFolderId;
          if (cur !== null) void createAndName(cur, null);
          else {
            openBlock(DEFAULT_BLOCK_ID);
            void createAndName(null, null, DEFAULT_BLOCK_ID);
          }
        },
      },
      {
        label: "新建档案",
        shortcut: "Shift+N",
        // 3.2 一轮点亮：同新建文件夹口径——默认区块「目录」根级，区块折叠先强制展开
        action: () => {
          openBlock(DEFAULT_BLOCK_ID);
          void createArchiveAndName(null, null, DEFAULT_BLOCK_ID);
        },
      },
      "divider",
    );
  }
  entries.push(
    { label: "导入本地文件", action: () => void pickImportPaths("files") },
    { label: "导入本地文件夹", action: () => void pickImportPaths("folder") },
    { label: "导入素材包", disabled: true },
    // 添加文件/档案链接（6.2 三轮点亮）：进选择模式文件模板，落点 = 当前文件夹视口（无文件夹视口禁用）；
    // 「档案链接管理」项删除（2026-08-24 用户定：链接管理针对单个文件，入口 = 右键菜单 + 详情面板）
    {
      label: "添加文件/档案链接",
      shortcut: "Ctrl+L",
      disabled: currentLinkTarget() === null,
      action: () => {
        const t = currentLinkTarget();
        if (t) enterLinkPicker(t);
      },
    },
    "divider",
    ...scanEntries(),
  );
  entries.push("divider", ...newDocGroupEntries());
  return entries;
}

// —— 快速访问空白处收窄（2026-09-05 走查「快速访问」节；2026-09-09 入口收窄修订）——
// 未分类 = 候车区总入口：新增菜单去掉新建两项（新建文件夹/新建档案——快速访问无文件夹概念），
// 留导入 / 新建文档 / 粘贴 / 添加链接 / 扫描
export function quickFullSpec(): CtxEntry[] {
  return addButtonSpec({ includeCreate: false });
}

// 全部 / 最近使用 / 自动导入（quick:inbox）/ 缩略图：只给扫描两项（新建文档/导入/链接全不出——
// 最近使用是"使用过的文件"，新建文件污染严重；自动导入、缩略图用户同定收窄；
// 2026-09-09 收窄：全部 = 虚拟视图不是位置，落点类同档禁）
export function scanOnlySpec(): CtxEntry[] {
  return scanEntries();
}
