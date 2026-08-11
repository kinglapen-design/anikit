// 资源库操作层（1.3 二轮：08 切换资源库菜单 + 14 汉堡菜单，2026-08-11）：
// 库列表加载（最近列表 + 当前库判定）、切换资源库（open_library → 标签页布局重置 → 全量刷新）、
// "新建资源库"模态共享态（LibrarySetupModal 原是 App.vue 启动强制弹窗，菜单触发需可关闭）。
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

interface LibraryInfo {
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

// 切换资源库：open_library 后旧库 viewKey 全失效——标签页布局重置（关空自动补"全部"），再全量刷新
export async function switchLibrary(path: string): Promise<void> {
  const content = useContentStore();
  const tabs = useTabsStore();
  const toast = useToastStore();
  const undo = useUndoStore();
  try {
    const info = await invoke<LibraryInfo>("open_library", { path });
    tabs.closeAllTabs();
    await content.refresh();
    void undo.refreshState();
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
