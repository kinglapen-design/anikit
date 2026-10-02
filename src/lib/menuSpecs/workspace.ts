// 工作空间右键菜单 spec（PRD 02 步骤 1 只读底座 + 步骤 2 基础整理 + 2026-09-17 手动刷新兜底）：
// 2026-10-01 库菜单移植轮——分组 / 分割线 / 快捷键标注全面对齐库菜单稿（纯文字口径）：
// 文件 = 打开预览组（新标签页 / 默认应用 / 其他应用 / 发送至 Animate / 资源管理器 / 系统菜单）│
// 剪贴板组（复制 / 剪切 / 粘贴条件出现 / 重命名单选 / 创建副本）│ 复制数据 ▸ │ 删除 │ 刷新；
// 文件夹（卡片与树行共用主体）= 新标签页 / 新建文件夹（同级）/ 新建子文件夹 / 添加链接文件夹… /
// 在外部位置创建链接…（方向 a′ 产品化 2026-10-02：本文件夹在外部落点摆入口 junction，只管创建
// 不管存续）│ 重新定位…·删除链接（仅链接门，junction 方向 b 2026-10-02；断链即弹修复窗同术语）·
// 固定·取消固定 │
// 剪贴板组 │ 复制数据 ▸ │ 资源管理器 / 系统菜单 │ 删除 │ 刷新（树行无重命名——ws 树无行内命名态）；
// 空白 = 新建文件夹 / 添加链接文件夹… / 粘贴（内部剪贴板非空 || 系统剪贴板含文件——2026-10-01 PRD 02-13 扩，
// spec 构建 async：build 前 refreshSystemClip）│ 在资源管理器中显示 / 系统菜单（目录背景版——经典版
// 新建 / 排序方式 / 查看 / 刷新 / 属性 在其中）│ 刷新。
// 侧栏「文件夹」区块标题（2026-10-03 用户定，库区块头同口径悬停动作钮）：+ = 根目录新建文件夹
//（createAndRename("") 同径）；⋯ = 添加链接文件夹…（挂根）/ 管理工作空间面板。
// 链接门（junction 方向 b）：「添加链接文件夹…」= 系统目录选择器 → ws_add_link（挂载名 = 目标目录名，
// 重名后端 _n；校验拒绝中文错 toast）；「重新定位… / 删除链接」仅当条目是链接门（ws.linkState 查父目录
// 缓存，未加载按非链接不列——条件出现口径速查 §25）；删除链接 = 只删门进系统回收站，目标内容不动；
// 选中断链门即弹修复窗（selection.brokenLinkPrompt，宿主 App.vue——主钮 重新定位… / 次钮 删除链接 /
// 取消 = 先不管，2026-10-02 用户定）。
// 「系统菜单」= Windows 原生右键菜单（IContextMenu：压缩解压 / TortoiseGit 等 shell 扩展全量）。
// 一律按路径走 ws 命令（assetId = ws: 合成假 id 严禁发 assetId 类命令）；
// 写操作失败 toast 报错，成功由 ws store 局部重扫（watcher 回声后端消重）。
// 重命名 = 行内命名态入口：卡片经 selection.renamingKey（AssetCard 接管提交），
// 文件夹经 selection.namingFolderCardId（FolderStrip 接管提交），与双击名称同径；
// 新建 = 库「先建后命名」口径——默认名创建后当前视口正是父目录时进卡片行内命名态。
// 快捷键标注与库同口径：F2 / Delete / Ctrl+C·X·V / Shift+Enter / Ctrl+D / Ctrl+E 已绑
//（useGlobalKeys ws 分支）；Enter / Ctrl+Enter / Ctrl+Shift+N / Ctrl+Shift+Alt+N 仅展示（库同）。
import { nextTick } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import type { PlacementView } from "../../stores/content";
import type { CtxEntry } from "../contextMenus";
import { previewSession } from "../../stores/preview";
import { useContentStore } from "../../stores/content";
import { useSelectionStore } from "../../stores/selection";
import { useTabsStore } from "../../stores/tabs";
import { useWsStore } from "../../stores/ws";
import {
  linkOutWsFolder,
  openPathWithDefaultApp,
  openPathWithDialog,
  revealPath,
  sendPathToAnimate,
  wsFileBase64,
  wsShellMenu,
  wsShellMenuBackground,
} from "../wsOps";
import { copyText, importWsToLibraryFlow } from "../menuOps";
import { useToastStore } from "../../stores/toast";
import { useUiStore } from "../../stores/ui";

const fail = (e: unknown) => useToastStore().notify({ message: String(e), icon: "fail" });

// 刷新当前目录（手动兜底）：全量重扫 + 补请求缺失缩略图（fs-changed 热刷新同径，见 stores/ws）
function refreshDir(rel: string): void {
  void useWsStore().refreshDir(rel);
}

// 删除两段确认（与库删除同款 confirmPrompt 警告黄钮；系统回收站可找回，文案明说）
function confirmDelete(rels: string[], name: string): void {
  useSelectionStore().confirmPrompt = {
    title: "删除",
    body: `「${name}」将被移到系统回收站，可从回收站找回。`,
    confirmLabel: "删除",
    kind: "warning",
    onConfirm: () =>
      void useWsStore()
        .removeEntries(rels)
        .catch((e: unknown) => useToastStore().notify({ message: String(e), icon: "fail" })),
  };
}

// —— junction 挂载（方向 b：外部文件夹挂进 WS 子目录，2026-10-02）——

// 链接目标目录选择器（添加链接文件夹 / 重新指向 / 链出去三用——链出去选的是「入口摆放位置」，
// 语义相反给 title 防混淆）：取消 = null 则止
async function pickLinkTarget(title?: string): Promise<string | null> {
  const picked = await open({ directory: true, multiple: false, ...(title ? { title } : {}) });
  return typeof picked === "string" ? picked : null;
}

// 添加链接文件夹：选中外部目录以 junction 挂进 destRel（挂载名 = 目标目录名，重名后端 _n）；
// 校验拒绝（非目录 / 工作空间内 / 祖先循环）后端中文错 toast
//（export = 「管理工作空间」面板「添加链接文件夹」钮复用同径，destRel "" = 挂根，2026-10-02）
export async function addLinkFlow(destRel: string): Promise<void> {
  const dir = await pickLinkTarget();
  if (dir === null) return;
  await useWsStore().addLink(destRel, dir).catch(fail);
}

// 从 WS 里链出去（方向 a′ 产品化，2026-10-02）：本文件夹在外部落点摆入口 junction——内容
// 留空间卷（阶段九硬链接永不跨卷），外部门误删十秒重建；只管创建不管存续（不登记不监听），
// 成功 toast 入口绝对路径；校验拒绝（源非文件夹 / 落点在储存空间内）后端中文错 toast
async function linkOutFlow(rel: string): Promise<void> {
  const dir = await pickLinkTarget("选择链接入口的摆放位置");
  if (dir === null) return;
  const link = await linkOutWsFolder(rel, dir).catch((e: unknown) => {
    fail(e);
    return null;
  });
  if (link !== null) useToastStore().notify({ message: `已创建外部链接入口：${link}`, icon: "success" });
}

// 重新定位（断链送修同径；断链修复弹窗主钮共用——export 供 App.vue 断链弹窗调）：链接重指向新选目录
export async function retargetLinkFlow(rel: string): Promise<void> {
  const dir = await pickLinkTarget();
  if (dir === null) return;
  await useWsStore().retargetLink(rel, dir).catch(fail);
}

// 删除链接两段确认（与删除同款 confirmPrompt 警告黄钮；断链修复弹窗的「删除链接」= 无再确认直删——
// 弹窗本身就是提示）：只删链接本身（junction 门进系统回收站可找回），目标文件夹内容不受影响——文案明说
function confirmRemoveLink(rel: string, name: string): void {
  useSelectionStore().confirmPrompt = {
    title: "删除链接",
    body: `只删除链接「${name}」本身（进系统回收站，可找回），目标文件夹内容不受影响。`,
    confirmLabel: "删除链接",
    kind: "warning",
    onConfirm: () =>
      void useWsStore()
        .removeLink(rel)
        .catch((e: unknown) => useToastStore().notify({ message: String(e), icon: "fail" })),
  };
}

// 新建文件夹共享入口（库「先建后命名」口径移植）：默认名创建（ws store 避让重名），
// 当前视口正是父目录时进新卡片的行内命名态（FolderStrip watcher 消费；其余入口如树行
// 新建子文件夹不在当前视口条带内，不进命名态——与库树命名态在卡片缺位时的降级同理）
//（export = 侧栏「文件夹」区块标题 + 钮复用同径，rel "" = 根目录新建，2026-10-03）
export async function createAndRename(rel: string): Promise<void> {
  const ws = useWsStore();
  try {
    const name = await ws.createFolder(rel);
    if (useContentStore().sidebarSelected === `ws:${rel}`) {
      const newRel = rel ? `${rel}/${name}` : name;
      await nextTick();
      useSelectionStore().namingFolderCardId = newRel;
    }
  } catch (e) {
    fail(e);
  }
}

// 批量分流（同库 3.4 口径）：触发项在选择域内且多选 → 动作作用于整个 ws 选中集
//（文件与文件夹可混合——ws 写操作按路径受理）；返回 rel 清单
function batchRels(triggerKey: string, triggerRel: string): string[] {
  const selection = useSelectionStore();
  return selection.count > 1 && selection.isSelected(triggerKey)
    ? selection.selectedRefs().filter((r) => r.assetId.startsWith("ws:")).map((r) => r.assetId.slice(3))
    : [triggerRel];
}

// 系统菜单（Windows 原生右键菜单）：整组选中集一次弹出；root 未就绪（absPath null）静默不动
function openShellMenu(rels: string[]): void {
  const ws = useWsStore();
  const absPaths = rels.map((r) => ws.absPath(r)).filter((x): x is string => x !== null);
  if (absPaths.length > 0) void wsShellMenu(absPaths).catch(fail);
}

// 在资源管理器中显示（root 未就绪静默不动）
function revealRel(rel: string): void {
  const abs = useWsStore().absPath(rel);
  if (abs) void revealPath(abs).catch(fail);
}

// 发送至 Animate（8.3 按路径移植）：同库口径——点击即最小化主窗口（用户视线在 AN 侧，
// AN 弹窗不能被 APP 挡住），结果 toast 回窗后可见
export function sendWsToAnimate(absPath: string): void {
  const toast = useToastStore();
  void getCurrentWindow().minimize();
  void sendPathToAnimate(absPath)
    .then((name) => toast.notify({ message: `已发送至 Animate：${name}`, icon: "success" }))
    .catch((e: unknown) => toast.notify({ message: String(e), icon: "fail" }));
}

// 复制 base64（复制数据子菜单）：后端白名单 / 10MB 上限校验，失败中文 toast
function copyWsBase64(rel: string): void {
  const abs = useWsStore().absPath(rel);
  if (!abs) return;
  void wsFileBase64(abs)
    .then((b64) => copyText(b64, "已复制 base64"))
    .catch(fail);
}

// 复制数据 ▸ 子菜单（库菜单移植：ws 无 anikit:// 链接 / 标签描述，留路径 / 名称 / base64）：
// forFile=false（文件夹）不出 base64 项
function copyDataSubmenu(rel: string, name: string, forFile: boolean): CtxEntry[] {
  return [
    { label: "复制文件路径", action: () => void copyText(useWsStore().absPath(rel) ?? "", "已复制文件路径") },
    "divider",
    { label: "复制名称", action: () => void copyText(name, "已复制名称") },
    ...(forFile ? [{ label: "复制 base64", action: () => copyWsBase64(rel) } as CtxEntry] : []),
  ];
}

// 视口空白处菜单（gridDispatch ws 分支调用）。
// 2026-10-01 起 spec 构建 async：先 refreshSystemClip——「粘贴」出现条件 = 内部剪贴板非空 ||
// 系统剪贴板含文件（PRD 02-13；系统菜单已证明右键流程可 async）
export async function wsBlankSpec(rel: string): Promise<CtxEntry[]> {
  const ws = useWsStore();
  await ws.refreshSystemClip();
  const entries: CtxEntry[] = [
    { label: "新建文件夹", shortcut: "Ctrl+Shift+N", action: () => void createAndRename(rel) },
    // 链接文件夹（junction 方向 b）：外部目录挂进当前目录
    { label: "添加链接文件夹…", action: () => void addLinkFlow(rel) },
  ];
  if (ws.clip || ws.systemClipHasFiles) {
    entries.push({
      label: ws.clip?.cut ? "粘贴（移动）" : "粘贴",
      shortcut: "Ctrl+V",
      action: () => void ws.paste(rel).catch(fail),
    });
  }
  entries.push(
    "divider",
    { label: "在资源管理器中显示", action: () => revealRel(rel) },
    // 系统菜单 - 目录背景版（2026-10-01 补）：经典版 新建 / 排序方式 / 查看 / 刷新 / 属性 在其中
    {
      label: "系统菜单",
      action: () => {
        const abs = ws.absPath(rel);
        if (abs) void wsShellMenuBackground(abs).catch(fail);
      },
    },
    "divider",
    { label: "刷新", action: () => refreshDir(rel) },
  );
  return entries;
}

// 侧栏「文件夹」区块标题「更多」钮菜单（2026-10-03 用户定：区块头悬停动作钮补齐，
// 与库区块头 ⋯ 钮同口径指针位弹出）：添加链接文件夹（destRel "" = 挂根，管理面板
// 添加钮同径）/ 管理工作空间面板（appMenu「管理工作空间」同入口，ui.wsManageOpen）
export function wsFolderSectionSpec(): CtxEntry[] {
  return [
    { label: "添加链接文件夹…", action: () => void addLinkFlow("") },
    { label: "管理工作空间", action: () => void (useUiStore().wsManageOpen = true) },
  ];
}

// ws 文件夹菜单主体（文件夹卡片 / 树行共用）：树行无行内命名态 → rename = false 摘重命名项。
// 与库 folderMenuEntries 对齐移植（2026-10-01）：打开组 / 固定 / 剪贴板组 / 复制数据 / 系统菜单组 /
// 删除 / 刷新——库的库专属段（排序依据 / 展开收起 / 图标颜色 / 添加资源库 / 导出）不挂（磁盘实况无语义）。
// 「打开」冗余项不列（同库 2026-09-23_02 口径：单击卡片 / 树行已导航）。
// async 同 wsBlankSpec：build 前 refreshSystemClip（「粘贴」出现条件扩为 内部非空 || 系统含文件）
async function wsFolderEntries(rel: string, name: string, rename: boolean): Promise<CtxEntry[]> {
  const ws = useWsStore();
  const selection = useSelectionStore();
  const tabs = useTabsStore();
  await ws.refreshSystemClip();
  const parentRel = rel.includes("/") ? rel.slice(0, rel.lastIndexOf("/")) : "";
  const key = `ws:${rel}|ws:${parentRel}`;
  const batch = batchRels(key, rel);
  const multi = batch.length > 1;
  const pinned = ws.pinned.includes(rel);
  // 链接门（junction 方向 b）：条目联接态查父目录缓存——非链接 / 父目录未加载（null）按非链接，
  // 「重新指向… / 移除链接」不列（条件出现 ≠ disabled 占位，速查 §25）
  const link = ws.linkState(rel);
  return [
    { label: "新标签页打开", shortcut: "Ctrl+Enter", action: () => tabs.openInNewTab(`ws:${rel}`) },
    { label: "新建文件夹", shortcut: "Ctrl+Shift+N", action: () => void createAndRename(parentRel) },
    { label: "新建子文件夹", shortcut: "Ctrl+Shift+Alt+N", action: () => void createAndRename(rel) },
    // 链接文件夹（junction 方向 b）：外部目录挂进本文件夹
    { label: "添加链接文件夹…", action: () => void addLinkFlow(rel) },
    // 链出去（方向 a′ 产品化）：本文件夹在外部落点摆入口 junction（只管创建不管存续）；
    // 多选只作用被右键项（§65 链接菜单口径），不整批分流
    { label: "在外部位置创建链接…", action: () => void linkOutFlow(rel) },
    // 导入到库（2026-10-03 ws ↔ 库互通）：文件夹整棵进库（结构保留 / 序列帧由导入弹窗确认）
    { label: "导入到库", action: () => importWsToLibraryFlow(batch) },
    "divider",
    ...(link?.isLink
      ? [
          { label: "重新定位…", action: () => void retargetLinkFlow(rel) } as CtxEntry,
          { label: "删除链接", action: () => confirmRemoveLink(rel, name) } as CtxEntry,
        ]
      : []),
    {
      label: pinned ? "从常用中取消固定" : "固定到常用",
      action: () => void (pinned ? ws.unpin(rel) : ws.pin(rel)).catch(fail),
    },
    "divider",
    { label: multi ? `复制 ${batch.length} 项` : "复制", shortcut: "Ctrl+C", action: () => ws.setClip(batch, false) },
    { label: multi ? `剪切 ${batch.length} 项` : "剪切", shortcut: "Ctrl+X", action: () => ws.setClip(batch, true) },
    // 粘贴进该文件夹（库文件夹菜单「粘贴文件夹」同语义；2026-10-01 出现条件扩为
    // 内部剪贴板非空 || 系统剪贴板含文件，动作不变——store 内序号仲裁）
    ...(ws.clip || ws.systemClipHasFiles
      ? [
          {
            label: ws.clip?.cut ? "粘贴（移动）" : "粘贴",
            shortcut: "Ctrl+V",
            action: () => void ws.paste(rel).catch(fail),
          } as CtxEntry,
        ]
      : []),
    ...(rename && !multi
      ? [{ label: "重命名", shortcut: "F2", action: () => void (selection.namingFolderCardId = rel) } as CtxEntry]
      : []),
    {
      label: multi ? `创建 ${batch.length} 个副本` : "创建副本",
      shortcut: "Ctrl+D",
      action: () => void ws.duplicate(batch).catch(fail),
    },
    "divider",
    { label: "复制数据", submenu: copyDataSubmenu(rel, name, false) },
    "divider",
    { label: "在资源管理器中显示", action: () => revealRel(rel) },
    { label: "系统菜单", action: () => openShellMenu(batch) },
    "divider",
    {
      label: multi ? `删除 ${batch.length} 项` : "删除",
      shortcut: "Delete",
      action: () => confirmDelete(batch, multi ? `已选中的 ${batch.length} 项` : name),
    },
    "divider",
    { label: "刷新", action: () => refreshDir(rel) },
  ];
}

// ws 文件夹卡片菜单（FolderStrip ws 分流调用；重命名 = 置 FolderStrip 行内命名态，提交在其 ws 分支）
export async function wsFolderSpec(rel: string, name: string): Promise<CtxEntry[]> {
  return await wsFolderEntries(rel, name, true);
}

// ws 树行菜单（WorkspaceTree 行包装 div 右键调用；pin 行（常用区块）复用本 spec——纯导航行
// 不入选择域，批量分流天然不命中，退化为对 pin 目录自身的单项管理操作）。
// 树行无重命名项（ws 树无行内命名态——重命名走视口文件夹卡片 / 名称双击）
export async function wsTreeRowSpec(rel: string): Promise<CtxEntry[]> {
  return await wsFolderEntries(rel, rel.slice(rel.lastIndexOf("/") + 1), false);
}

// ws 条目菜单（打开预览 = 进预览，与双击卡片同径；重命名 = 置卡片行内命名态，
// 提交在 AssetCard 的 ws 分支；所在目录 rel = folderId 去 "ws:" 前缀）。
// 与库 assetCardSpec 对齐移植（2026-10-01）：库的库专属组（移动至分组 / 链接管理 / 缩略图 /
// 导出 / AI / 提示词 / 置顶）不挂——磁盘实况无语义或未立项；「新窗口打开」不挂（预览子窗口
// 视口解析不含 ws 合成项，链路未通）。
// panelId = 发起右键的 panel（2026-09-25 per-panel 会话化：「打开预览」开进属主 panel 会话）。
// async 同 wsBlankSpec：build 前 refreshSystemClip（「粘贴」出现条件扩为 内部非空 || 系统含文件）
export async function wsCardSpec(p: PlacementView, panelId: string): Promise<CtxEntry[]> {
  const rel = p.folderId.slice(3);
  const entryRel = p.assetId.slice(3);
  const ws = useWsStore();
  const selection = useSelectionStore();
  const tabs = useTabsStore();
  await ws.refreshSystemClip();
  const key = `${p.assetId}|${p.folderId}`;
  const batch = batchRels(key, entryRel);
  const multi = batch.length > 1;
  return [
    { label: "打开预览", shortcut: "Enter", action: () => void previewSession(panelId).open(key) },
    { label: "新标签页打开", shortcut: "Ctrl+Enter", action: () => tabs.openPreviewInNewTab(panelId, key) },
    // 打开外发组（库 6.3 同组移植）：默认应用 / 打开方式作用于被右键项（同库单发口径，不批量）
    {
      label: "在默认应用打开",
      shortcut: "Shift+Enter",
      action: () => {
        const abs = ws.absPath(entryRel);
        if (abs) void openPathWithDefaultApp(abs).catch(fail);
      },
    },
    {
      label: "在其他应用打开",
      action: () => {
        const abs = ws.absPath(entryRel);
        if (abs) void openPathWithDialog(abs).catch(fail);
      },
    },
    // 发送至 Animate：仅 fla + 单选（库同口径；多选发送记 09 待办）；面板未连接 / 在途由后端报错 toast
    {
      label: "发送至 Animate",
      shortcut: "Ctrl+E",
      disabled: multi || p.ext !== "fla",
      action: () => {
        const abs = ws.absPath(entryRel);
        if (abs) sendWsToAnimate(abs);
      },
    },
    // 导入到库（2026-10-03 ws ↔ 库互通，PRD 02）：ws → 库 = 复制导入（importDraft 既有流），
    // 目标文件夹由库树选择器定；混选（文件 + 文件夹）整批受理（classify_paths 判 kind）
    { label: "导入到库", action: () => importWsToLibraryFlow(batch) },
    { label: "在资源管理器中显示", action: () => revealRel(entryRel) },
    { label: "系统菜单", action: () => openShellMenu(batch) },
    "divider",
    { label: multi ? `复制 ${batch.length} 项` : "复制", shortcut: "Ctrl+C", action: () => ws.setClip(batch, false) },
    { label: multi ? `剪切 ${batch.length} 项` : "剪切", shortcut: "Ctrl+X", action: () => ws.setClip(batch, true) },
    // 粘贴进当前目录（库素材菜单粘贴项同语义；2026-10-01 出现条件扩为
    // 内部剪贴板非空 || 系统剪贴板含文件，动作不变——store 内序号仲裁）
    ...(ws.clip || ws.systemClipHasFiles
      ? [
          {
            label: ws.clip?.cut ? "粘贴（移动）" : "粘贴",
            shortcut: "Ctrl+V",
            action: () => void ws.paste(rel).catch(fail),
          } as CtxEntry,
        ]
      : []),
    ...(multi ? [] : [{ label: "重命名", shortcut: "F2", action: () => void (selection.renamingKey = key) } as CtxEntry]),
    {
      label: multi ? `创建 ${batch.length} 个副本` : "创建副本",
      shortcut: "Ctrl+D",
      action: () => void ws.duplicate(batch).catch(fail),
    },
    "divider",
    { label: "复制数据", submenu: copyDataSubmenu(entryRel, p.name, true) },
    "divider",
    {
      label: multi ? `删除 ${batch.length} 项` : "删除",
      shortcut: "Delete",
      action: () => confirmDelete(batch, multi ? `已选中的 ${batch.length} 个文件` : p.name),
    },
    "divider",
    { label: "刷新", action: () => refreshDir(rel) },
  ];
}
