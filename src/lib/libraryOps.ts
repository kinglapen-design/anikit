// 资源库操作层（1.3 二轮：08 切换资源库菜单 + 14 汉堡菜单，2026-08-11）：
// 库列表加载（最近列表 + 当前库判定）、开库三入口统一（openLibrary / createLibrary / reopenLastLibrary）、
// 切换资源库（toast 反馈）、"新建资源库"模态共享态（LibrarySetupModal 原是 App.vue 启动强制弹窗，菜单触发需可关闭）。
// LibraryInfo 唯一声明处（2026-08-11 重构：LibrarySetupModal 本地重复声明废除，规范第 1 条）。
import { reactive } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useContentStore } from "../stores/content";
import { useTabsStore } from "../stores/tabs";
import { useToastStore } from "../stores/toast";
import { useUndoStore } from "../stores/undo";

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

// 新建资源库模态共享态：open = 菜单触发显示；closable = 有库时可关（点遮罩），启动强制场景不变
export const librarySetupState = reactive({ open: false, closable: false });

// 库列表（08 菜单动态项）：最近列表 + 当前库路径（勾选判定）
export async function loadLibraries(): Promise<{ entries: LibraryEntry[]; currentPath: string | null }> {
  const [entries, current] = await Promise.all([
    invoke<LibraryEntry[]>("list_recent_libraries").catch(() => [] as LibraryEntry[]),
    invoke<LibraryInfo | null>("get_current_library").catch(() => null),
  ]);
  return { entries, currentPath: current?.path ?? null };
}

// 开库统一收尾：旧库 viewKey 全失效——标签页布局重置（关空自动补"全部"），再全量刷新；错误抛出由调用方处理
async function afterOpen(info: LibraryInfo): Promise<LibraryInfo> {
  const tabs = useTabsStore();
  const content = useContentStore();
  const undo = useUndoStore();
  tabs.closeAllTabs();
  await content.refresh();
  void undo.refreshState();
  return info;
}

// 打开资源库（LibrarySetupModal"打开"流程）：无 toast（模态自身有错误态 / 成功即关）
export async function openLibrary(path: string): Promise<LibraryInfo> {
  return await afterOpen(await invoke<LibraryInfo>("open_library", { path }));
}

// 创建资源库（LibrarySetupModal"创建"流程）：建库即打开，收尾同 openLibrary
export async function createLibrary(path: string): Promise<LibraryInfo> {
  return await afterOpen(await invoke<LibraryInfo>("create_library", { path }));
}

// 启动自动重开最近的库（2026-08-08 用户定：不再每次弹"创建/打开资源库"选路径）：
// 仅 content.refresh（启动时本无标签页，不 closeAllTabs、不 toast）；
// 最近列表为空（首次使用）或重开失败（路径失效等）返回 false——调用方弹模态
export async function reopenLastLibrary(): Promise<boolean> {
  const recent = await invoke<LibraryEntry[]>("list_recent_libraries").catch(() => [] as LibraryEntry[]);
  if (recent.length === 0) return false;
  try {
    await invoke<LibraryInfo>("open_library", { path: recent[0].path });
    await useContentStore().refresh();
    return true;
  } catch (e) {
    console.warn("最近库自动重开失败，转弹窗：", e);
    return false;
  }
}

// 切换资源库（08 菜单）：开库收尾同 openLibrary，成功 / 失败 toast 反馈
export async function switchLibrary(path: string): Promise<void> {
  const toast = useToastStore();
  try {
    const info = await openLibrary(path);
    toast.notify({ message: `已切换到资源库「${info.name}」`, icon: "success", duration: 3000 });
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  }
}

// 新建资源库（菜单触发）：打开创建/打开资源库模态（有库时点遮罩可关）
export function requestLibrarySetup(): void {
  librarySetupState.open = true;
  librarySetupState.closable = true;
}
