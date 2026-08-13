// 07 稿 · 新增菜单 spec（自 menuSpecs.ts 拆出，规范第 3 条行数控制）：宽 320 无搜索框。
// 新建文档组（newDocGroupEntries）与 14 汉堡"文件"二级共用，故同文件导出。
import { useSelectionStore } from "../../stores/selection";
import { useContentStore } from "../../stores/content";
import type { CtxEntry, CtxItem } from "../contextMenus";
import { docApp } from "../docApps";
import { createAndName, pickFolderForLink, pickImportPaths } from "../menuOps";
import { createTextDocument, pasteClipboard, pasteDuplicate } from "../placementOps";
import { openScan } from "../dupScanOps";
// MarkDown 文档图标（2026-08-11 用户供 md.webp，webp 非 svg 走 iconData 内联图）
import mdDocIcon from "../../../assets/icons/01_icon-default/md.webp?url";

// —— 素材卡片空白处（废除，2026-08-11：空白右键统一走 07 新增菜单；
// 布局方式/排序/显示简介/分组依据归 28 查看方式菜单 ViewOptionsMenu）——

// —— 新建文档组（07 稿 / 14 汉堡"文件"二级共用）——
// 系统软件检测动态显隐（lib/docApps，每次打开 UI 监测）：已安装 → 带系统图标（exe 提取，iconData）
// 显示，未安装 → 删除对应新建项；MarkDown 文档恒显示（纯文本无需软件）；
// MarkDown / 文本文档已点亮（createTextDocument），其余新建业务 3.1 接。
export function newDocGroupEntries(): CtxEntry[] {
  const docItem = (id: string, label: string, action?: () => void): CtxItem | null => {
    const app = docApp(id);
    if (!app?.installed) return null;
    return { label, iconData: app.icon ?? undefined, disabled: !action, action };
  };
  const softwareDocs = [
    docItem("animate", "Animate 文档"),
    docItem("photoshop", "Photoshop 文档"),
    docItem("afterEffects", "After Effects 文档"),
    docItem("illustrator", "Illustrator 文档"),
  ].filter((it): it is CtxItem => it !== null);
  const officeDocs = [
    docItem("text", "文本文档", () => void createTextDocument("txt")),
    docItem("word", "Word 文档"),
    docItem("excel", "Excel 文档"),
  ].filter((it): it is CtxItem => it !== null);
  const docGroup: CtxEntry[] = [];
  if (softwareDocs.length > 0) docGroup.push(...softwareDocs, "divider");
  docGroup.push({ label: "MarkDown 文档", iconData: mdDocIcon, action: () => void createTextDocument("md") });
  if (officeDocs.length > 0) docGroup.push(...officeDocs);
  return docGroup;
}

// —— 07 稿 · 新增菜单（宽 320 无搜索框）——
// 三入口共用一套（2026-08-11 用户定）：侧栏"新增"按钮 / 网格空白处右键 / Ctrl+N（按钮位弹出）。
// 上下文规则：剪贴板无文件 → 三个粘贴项不出现（同 03 稿口径，2026-08-11 用户定）；
// 新建文档组 = 系统软件检测动态显隐（lib/docApps，每次打开 UI 监测）：已安装 → 带系统图标
//（exe 提取，iconData）显示，未安装 → 删除对应新建项；MarkDown 文档恒显示（纯文本无需软件）；
// 新建文档业务阶段三 3.1 接，本轮项仍 disabled 占位。
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
      { label: "作为链接粘贴并重命名", icon: "paste-as-link-rename", shortcut: "Ctrl+Shift+V", disabled: true },
      { label: "作为副本粘贴", icon: "paste-as-copy", shortcut: "Ctrl+Alt+V", action: () => void pasteDuplicate() },
      "divider",
    );
  }
  entries.push(
    { label: "新建文件夹", icon: "new-folder", shortcut: "Ctrl+Shift+N", action: () => void createAndName(null, null) },
    { label: "新建档案", icon: "new-archive", shortcut: "Shift+N", disabled: true },
    "divider",
    { label: "导入本地文件", icon: "import-file", action: () => void pickImportPaths("files") },
    { label: "导入本地文件夹", icon: "import-folder", action: () => void pickImportPaths("folder") },
    { label: "导入素材包", icon: "import-pack", disabled: true },
    { label: "档案链接管理", icon: "linked-file-manage", shortcut: "Ctrl+J", disabled: true },
    { label: "添加文件/档案链接", icon: "add-file-link", shortcut: "Ctrl+L", action: () => pickFolderForLink() },
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
