// 资源库 / 储存空间操作层（2.1 重写：储存空间层接入，38_向导稿）：
// 空间与库的新建 / 打开唯一入口、启动自动重开（reopenLastSession 三态）、切库 toast、
// 向导共享态（wizardState：初次打开 startStep 0 / 菜单"新建资源库" startStep 2）、
// Windows 名称即时校验（CreateLibraryForm 退役提取至此，与后端 core/library.rs 同规则）。
import { reactive } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useContentStore } from "../stores/content";
import { useTabsStore } from "../stores/tabs";
import { useToastStore } from "../stores/toast";
import { useUndoStore } from "../stores/undo";

// 无主资产警告（2026-09-02 资产污染修复配套）：开库自检扫出孤儿 info（数据层无记录、
// 磁盘残留、库内不可见）→ 后端 activate emit 推清单，OrphanInfosAlert 弹窗列出。
// 简单版只展示；完整管理入口（打开资源管理器查看 / 删除按钮）用户后续补稿。
export interface OrphanNotice {
  id: string;
  name: string;
  path: string; // info 目录完整路径
}

export const orphanInfosState = reactive<{ list: OrphanNotice[] | null }>({ list: null });

let orphanInfosListening = false;
export async function initOrphanInfosListener(): Promise<void> {
  if (orphanInfosListening) return;
  orphanInfosListening = true;
  await listen<OrphanNotice[]>("library-orphan-infos", (e) => {
    orphanInfosState.list = e.payload.length > 0 ? e.payload : null;
  });
}

export interface LibraryEntry {
  name: string;
  path: string;
  lastOpenedAt: string;
}

export interface LibraryInfo {
  name: string;
  path: string;
  assetCount: number;
  folderCount: number;
}

export interface SpaceInfo {
  name: string;
  path: string;
  autoImport: boolean; // 自动导入（空间级设置，inbox 空间级共用，2026-08-12 用户定）
}

// 向导共享态：open = 显示；closable = 有库在用（菜单触发）点遮罩可关；startStep = 起始步骤
//（0 = 初次打开完整流程 / 2 = 菜单"新建资源库"直进第二步）
export const wizardState = reactive({ open: false, closable: false, startStep: 0 });

// Windows 文件夹命名即时校验（后端 create 同样把关，此处做输入侧提示）
const ILLEGAL_CHARS = /[\\/:*?"<>|]/;
const RESERVED_NAMES = /^(con|prn|aux|nul|com[1-9]|lpt[1-9])$/i;
export function validateAssetName(name: string): string {
  if (!name) return "名称不能留空";
  if (ILLEGAL_CHARS.test(name)) return '名称不能包含 \\ / : * ? " < > | 特殊符号';
  if (/[. ]$/.test(name)) return "名称不能以空格或点结尾";
  if (RESERVED_NAMES.test(name)) return "该名称为系统保留名，请换一个";
  return "";
}

// 库列表（08 菜单动态项）：当前空间内库名列表 + 当前库名（勾选判定）
export async function loadLibraries(): Promise<{ names: string[]; currentName: string | null }> {
  const [names, current] = await Promise.all([
    invoke<string[]>("list_space_libraries").catch(() => [] as string[]),
    invoke<LibraryInfo | null>("get_current_library").catch(() => null),
  ]);
  return { names, currentName: current?.name ?? null };
}

// 当前库 / 空间状态（侧栏库名、自动导入开关判定等）：开库/开空间/启动兜底更新，唯一数据源
export const currentLibrary = reactive<{ name: string | null }>({ name: null });
export const currentSpace = reactive<{ name: string | null; autoImport: boolean }>({
  name: null,
  autoImport: false,
});

// 开库统一收尾：旧库 viewKey 全失效——标签页布局重置（关空自动补"全部"），再全量刷新；错误抛出由调用方处理
async function afterOpen(info: LibraryInfo): Promise<LibraryInfo> {
  const tabs = useTabsStore();
  const content = useContentStore();
  const undo = useUndoStore();
  currentLibrary.name = info.name;
  tabs.closeAllTabs();
  content.clearArrivals(); // 3.5 新增区：切库即归位
  await content.refresh();
  void undo.refreshState();
  return info;
}

export async function createSpace(parentPath: string, name: string, autoImport: boolean): Promise<SpaceInfo> {
  const info = await invoke<SpaceInfo>("create_space", { parentPath, name, autoImport });
  currentSpace.name = info.name;
  currentSpace.autoImport = info.autoImport;
  return info;
}

export async function openSpace(path: string): Promise<SpaceInfo> {
  const info = await invoke<SpaceInfo>("open_space", { path });
  currentSpace.name = info.name;
  currentSpace.autoImport = info.autoImport;
  return info;
}

export async function listSpaceLibraries(): Promise<string[]> {
  return await invoke<string[]>("list_space_libraries").catch(() => [] as string[]);
}

// 创建资源库（16 稿弹窗 / 向导 standalone）：建库即打开，收尾同 openLibrary；
// 自动导入是空间级设置，建库不带该参数（2026-08-12 空间级 inbox 定案）
export async function createLibrary(name: string): Promise<LibraryInfo> {
  return await afterOpen(await invoke<LibraryInfo>("create_library", { name }));
}

// 打开资源库：当前空间内按名打开
export async function openLibrary(name: string): Promise<LibraryInfo> {
  return await afterOpen(await invoke<LibraryInfo>("open_library", { name }));
}

// 启动自动重开（2026-08-08 用户定不每次弹窗；2.1 空间化：最近库所属空间 → open_space → 空间内按名开库）。
// 三态：ok = 已开库（仅 content.refresh，不动标签页、不 toast）；
// need-library = 空间已开但无库（向导直进第二步）；need-wizard = 无最近记录 / 重开失败（向导入口帧）。
export async function reopenLastSession(): Promise<"ok" | "need-library" | "need-wizard"> {
  const recent = await invoke<LibraryEntry[]>("list_recent_libraries").catch(() => [] as LibraryEntry[]);
  if (recent.length === 0) return "need-wizard";
  try {
    const last = recent[0];
    // 库目录 = 空间根/<库名>，空间根 = 库路径去尾段
    const parts = last.path.split(/[\\/]/).filter(Boolean);
    parts.pop();
    if (parts.length === 0) return "need-wizard";
    await openSpace(parts.join("\\"));
    const libs = await listSpaceLibraries();
    if (libs.length === 0) return "need-library";
    await invoke<LibraryInfo>("open_library", { name: libs.includes(last.name) ? last.name : libs[0] }).then((info) => {
      currentLibrary.name = info.name;
    });
    await useContentStore().refresh();
    return "ok";
  } catch (e) {
    console.warn("最近空间/库自动重开失败，转向导：", e);
    return "need-wizard";
  }
}

// 切换资源库（08 菜单）：当前空间内按名开库，成功 / 失败 toast 反馈
export async function switchLibrary(name: string): Promise<void> {
  const toast = useToastStore();
  try {
    const info = await openLibrary(name);
    toast.notify({ message: `已切换到资源库「${info.name}」`, icon: "success", duration: 3000 });
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  }
}

// 新建资源库（菜单触发，16_创建资源库稿）：独立轻量弹窗（初次打开 / 无库场景走 38 向导）
export const createLibraryModalState = reactive({ open: false });

export function requestLibrarySetup(): void {
  createLibraryModalState.open = true;
}

// 库列表拖动排序持久化（08 稿，space.json libraryOrder 单写点）；失败 toast，顺序以后端为准（下次开菜单重拉）
export async function reorderLibraries(order: string[]): Promise<void> {
  try {
    await invoke<string[]>("reorder_libraries", { order });
  } catch (e) {
    useToastStore().notify({ message: String(e), icon: "fail" });
  }
}

// 删除资源库（08 稿 item 删除×）：确认弹窗状态 + 执行。
// 孤儿清理（2026-08-12 用户定案：统一池保留 + 引用计数清理）——弹窗前统计"仅该库引用"的素材数，
// 复选框勾选（默认勾）则删库时一并删除这些素材实体（info 文件夹），共享素材保留。
// 删当前打开的库 → 切剩余第一个 / 无剩余回第二步向导（PRD 01_储存空间 3.2）
export const deleteLibraryState = reactive<{ name: string | null; orphans: number; includeOrphans: boolean }>({
  name: null,
  orphans: 0,
  includeOrphans: true,
});

export function requestDeleteLibrary(name: string): void {
  deleteLibraryState.name = name;
  deleteLibraryState.orphans = 0;
  deleteLibraryState.includeOrphans = true;
  // 统计失败不阻塞弹窗（复选框隐藏 = 不删实体）
  void invoke<number>("count_library_orphans", { name })
    .then((n) => {
      if (deleteLibraryState.name === name) deleteLibraryState.orphans = n;
    })
    .catch(() => {});
}

export async function confirmDeleteLibrary(): Promise<void> {
  const name = deleteLibraryState.name;
  if (!name) return;
  const toast = useToastStore();
  try {
    const deleteOrphans = deleteLibraryState.orphans > 0 && deleteLibraryState.includeOrphans;
    const rest = await invoke<string[]>("delete_library", { name, deleteOrphans });
    deleteLibraryState.name = null;
    toast.notify({ message: `资源库「${name}」已删除`, icon: "success", duration: 3000 });
    if (currentLibrary.name === name) {
      currentLibrary.name = null;
      if (rest.length > 0) await switchLibrary(rest[0]);
      else {
        wizardState.open = true;
        wizardState.closable = false;
        wizardState.startStep = 2;
      }
    }
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  }
}

// 开关自动导入（快速访问"自动导入"确认弹窗，2026-08-12 用户定）：
// 空间级设置（space.json 单写点）+ 启停收件箱（盘符挂载 / 卸载、监听启停）；失败 toast 恒音，返回是否成功
export async function setAutoImport(enabled: boolean): Promise<boolean> {
  const toast = useToastStore();
  try {
    const info = await invoke<SpaceInfo>("set_auto_import", { enabled });
    currentSpace.autoImport = info.autoImport;
    return true;
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
    return false;
  }
}
