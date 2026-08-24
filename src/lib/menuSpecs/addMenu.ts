// 07 稿 · 新增菜单 spec（自 menuSpecs.ts 拆出，规范第 3 条行数控制）：宽 320 无搜索框。
// 新建文档组（newDocGroupEntries）与 14 汉堡"文件"二级共用，故同文件导出。
import { useSelectionStore } from "../../stores/selection";
import { useContentStore, DEFAULT_BLOCK_ID } from "../../stores/content";
import type { CtxEntry } from "../contextMenus";
import { createAndName, createArchiveAndName, currentLinkTarget, enterLinkPicker, pickImportPaths } from "../menuOps";
import { openBlock } from "../treeExpansion";
import { createDocument, pasteClipboard, pasteDuplicate, type NewDocKind } from "../placementOps";
import { openScan } from "../dupScanOps";
// 文档图标（本地资源：md.webp 2026-08-11 用户供；txt.png 2026-08-15 系统记事本图标一次性提取留存；
// 非 svg 走 iconData 内联图，svg 走 getIcon 按名引用）
import mdDocIcon from "../../../assets/icons/01_icon-default/md.webp?url";
import txtDocIcon from "../../../assets/icons/01_icon-default/txt.png?url";

// —— 素材卡片空白处（废除，2026-08-11：空白右键统一走 07 新增菜单；
// 布局方式/排序/显示简介/分组依据归 28 查看方式菜单 ViewOptionsMenu）——

// —— 新建文档组（07 稿 / 14 汉堡"文件"二级共用）——
// 2026-08-15 用户定案：放弃 shell 检测模式（detect_doc_apps 废除），固定七项恒显 + 本地图标：
// Animate / Photoshop / Illustrator / MarkDown / 文本 / Word / Excel；
// AE 不建档（工程素材为链接路径、需打包工程，属工作空间而非资源库）。
export function newDocGroupEntries(): CtxEntry[] {
  const doc = (label: string, kind: NewDocKind, icon?: string, iconData?: string): CtxEntry => ({
    label,
    icon,
    iconData,
    action: () => void createDocument(kind),
  });
  return [
    doc("Animate 文档", "fla", "animate-on"),
    doc("Photoshop 文档", "psd", "photoshop-on"),
    doc("Illustrator 文档", "ai", "illustrator"),
    "divider", // 工程文档 / 文本·办公文档分组线（2026-08-15 用户定）
    doc("MarkDown 文档", "md", undefined, mdDocIcon),
    doc("文本文档", "txt", undefined, txtDocIcon),
    doc("Word 文档", "docx", "word"),
    doc("Excel 文档", "xlsx", "excel"),
  ];
}

// —— 07 稿 · 新增菜单（宽 320 无搜索框）——
// 三入口共用一套（2026-08-11 用户定）：侧栏"新增"按钮 / 网格空白处右键 / Ctrl+N（按钮位弹出）。
// 上下文规则：剪贴板无文件 → 三个粘贴项不出现（同 03 稿口径，2026-08-11 用户定）；
// 新建文档组 = 固定七项恒显（2026-08-15 用户定案放弃 shell 检测动态显隐，见 newDocGroupEntries）。
export function addButtonSpec(): CtxEntry[] {
  const selection = useSelectionStore();
  const content = useContentStore();
  const hasClipboard = selection.clipboard !== null;
  // 扫描重复文件范围上下文（2.7 二轮点亮）：当前目录 = 文件夹视图才有；所选文件 = 选中资产去重集
  const scanFolderId = content.currentFolderId;
  const scanAssetIds = [...new Set(selection.selectedRefs().map((r) => r.assetId))];
  const entries: CtxEntry[] = [];
  if (hasClipboard) {
    entries.push(
      { label: "作为链接粘贴", icon: "paste-as-link", shortcut: "Ctrl+V", action: () => void pasteClipboard() },
      { label: "作为副本粘贴", icon: "paste-as-copy", shortcut: "Ctrl+Shift+V", action: () => void pasteDuplicate() },
      "divider",
    );
  }
  entries.push(
    // 新建文件夹入口 = 默认区块「目录」根级（2026-08-14 区块子系统：保持原语义——根级最底部）；
    // 区块折叠时先强制展开让新节点可见（与侧栏区块头 + 按钮同口径）
    {
      label: "新建文件夹",
      icon: "new-folder",
      shortcut: "Ctrl+Shift+N",
      action: () => {
        openBlock(DEFAULT_BLOCK_ID);
        void createAndName(null, null, DEFAULT_BLOCK_ID);
      },
    },
    {
      label: "新建档案",
      icon: "new-archive",
      shortcut: "Shift+N",
      // 3.2 一轮点亮：同新建文件夹口径——默认区块「目录」根级，区块折叠先强制展开
      action: () => {
        openBlock(DEFAULT_BLOCK_ID);
        void createArchiveAndName(null, null, DEFAULT_BLOCK_ID);
      },
    },
    "divider",
    { label: "导入本地文件", icon: "import-file", action: () => void pickImportPaths("files") },
    { label: "导入本地文件夹", icon: "import-folder", action: () => void pickImportPaths("folder") },
    { label: "导入素材包", icon: "import-pack", disabled: true },
    // 添加文件/档案链接（6.2 三轮点亮）：进选择模式文件模板，落点 = 当前文件夹视口（无文件夹视口禁用）；
    // 「档案链接管理」项删除（2026-08-24 用户定：链接管理针对单个文件，入口 = 右键菜单 + 详情面板）
    {
      label: "添加文件/档案链接",
      icon: "add-file-link",
      shortcut: "Ctrl+L",
      disabled: currentLinkTarget() === null,
      action: () => {
        const t = currentLinkTarget();
        if (t) enterLinkPicker(t);
      },
    },
    "divider",
    {
      label: "扫描重复文件",
      icon: "scan-duplicates",
      submenuWidth: 96,
      submenu: [
        // 三范围（锚点过滤：组内任一资产命中整组返回，PRD 03_查找文件-03）；
        // 快速访问视图无当前文件夹 / 无选中素材时对应项禁用
        {
          label: "当前目录",
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
    { label: "扫描文件/档案链接", icon: "scan-links", disabled: true },
  );
  entries.push("divider", ...newDocGroupEntries());
  return entries;
}
