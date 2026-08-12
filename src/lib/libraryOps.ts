// 资源库 / 储存空间操作层（2.1 重写：储存空间层接入，38_向导稿）：
// 空间与库的新建 / 打开唯一入口、启动自动重开（reopenLastSession 三态）、切库 toast、
// 向导共享态（wizardState：初次打开 startStep 0 / 菜单"新建资源库" startStep 2）、
// Windows 名称即时校验（CreateLibraryForm 退役提取至此，与后端 core/library.rs 同规则）。
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

export interface SpaceInfo {
  name: string;
  path: string;
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

// 当前库名（侧栏库名显示等）：afterOpen / 启动兜底更新，唯一数据源
export const currentLibrary = reactive<{ name: string | null }>({ name: null });

// 开库统一收尾：旧库 viewKey 全失效——标签页布局重置（关空自动补"全部"），再全量刷新；错误抛出由调用方处理
async function afterOpen(info: LibraryInfo): Promise<LibraryInfo> {
  const tabs = useTabsStore();
  const content = useContentStore();
  const undo = useUndoStore();
  currentLibrary.name = info.name;
  tabs.closeAllTabs();
  await content.refresh();
  void undo.refreshState();
  return info;
}

export async function createSpace(parentPath: string, name: string): Promise<SpaceInfo> {
  return await invoke<SpaceInfo>("create_space", { parentPath, name });
}

export async function openSpace(path: string): Promise<SpaceInfo> {
  return await invoke<SpaceInfo>("open_space", { path });
}

export async function listSpaceLibraries(): Promise<string[]> {
  return await invoke<string[]>("list_space_libraries").catch(() => [] as string[]);
}

// 创建资源库（向导第二步 / 菜单"新建资源库"）：建库即打开，收尾同 openLibrary
export async function createLibrary(name: string, autoImport: boolean): Promise<LibraryInfo> {
  return await afterOpen(await invoke<LibraryInfo>("create_library", { name, autoImport }));
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
    await invoke<SpaceInfo>("open_space", { path: parts.join("\\") });
    const libs = await listSpaceLibraries();
    if (libs.length === 0) return "need-library";
    await invoke<LibraryInfo>("open_library", { name: libs.includes(last.name) ? last.name : libs[0] });
    currentLibrary.name = libs.includes(last.name) ? last.name : libs[0];
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

// 新建资源库（菜单触发）：向导直进第二步（有库时点遮罩 / X 可关）
export function requestLibrarySetup(): void {
  wizardState.open = true;
  wizardState.closable = true;
  wizardState.startStep = 2;
}
