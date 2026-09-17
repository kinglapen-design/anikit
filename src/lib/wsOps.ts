// 工作空间命令封装（PRD 02 步骤 1 只读底座，规范第 1 条）：workspace_root / list_workspace_dir /
// request_workspace_thumbs 统一出口 + 磁盘实况 / 缩略图事件监听（initWsListeners，App 存活期注册一次）。
// 类型以后端序列化结构为唯一来源（src-tauri/src/commands/workspace.rs、services/workspace.rs；
// serde camelCase，Option ↔ TS null）。本层不吞错：失败原样抛出，调用方按降级口径处理。
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useWsStore } from "../stores/ws";

// workspace_root 返回（commands/workspace.rs WorkspaceRootInfo）；null = 未开空间
export interface WsRootInfo {
  rootPath: string; // workspace/ 目录绝对路径（前端 convertFileSrc 直读其下文件）
  exists: boolean; // 目录是否存在（空间创建初期 / 目录被删时 false，前端据此显空态）
}

// list_workspace_dir 单条（services/workspace.rs WsEntry）：磁盘实况语义，无标签 / 评分等附加数据
export interface WsEntry {
  name: string; // 条目名（文件 / 文件夹名）
  relPath: string; // 相对 workspace 根的路径（/ 分隔）
  isDir: boolean;
  size: number; // 文件字节数；目录恒 0
  mtimeMs: number; // 文件修改时间（Unix 毫秒）；目录恒 0
  w: number | null; // 图片像素宽（内容魔数探测）；非图片 / 目录 = null
  h: number | null; // 图片像素高，同上
  ext: string; // 小写无点扩展名；目录 / 无扩展名 = ""
  thumbPath: string | null; // 缩略图缓存绝对路径：内容缩略图新鲜 / 格式图标已缓存时给，否则 null
  thumbKind: "asset" | "formatIcon" | null; // 缩略图来源（同库 ThumbKind 口径；未就绪 = null）
}

// workspace-thumbs-ready 负载条目（services/workspace.rs ThumbReady）
export interface WsThumbReady {
  absPath: string; // 源文件绝对路径
  thumbPath: string; // 缩略图缓存绝对路径
}

// 当前空间的工作空间根；未开空间 null
export async function getWorkspaceRoot(): Promise<WsRootInfo | null> {
  return await invoke<WsRootInfo | null>("workspace_root");
}

// 列举工作空间目录：relPath 空串 = 根；目录在前、同名不区分大小写排序；
// 未开空间 / workspace 目录缺失 / 越界 → []（后端降级不报错）
export async function listWorkspaceDir(relPath: string): Promise<WsEntry[]> {
  return await invoke<WsEntry[]>("list_workspace_dir", { relPath });
}

// 请求缩略图：立即返回，后台逐一生成缺失 / 过期的缓存（解码失败条目跳过），
// 完成 emit workspace-thumbs-ready（负载 = 本趟实际生成成功的清单，可能为空数组）
export async function requestWorkspaceThumbs(absPaths: string[]): Promise<void> {
  await invoke("request_workspace_thumbs", { absPaths });
}

// 在文件管理器中打开并选中（条目右键；约束在当前空间 workspace 内，越界 / 不存在抛中文错）
export async function revealPath(absPath: string): Promise<void> {
  await invoke("reveal_path", { absPath });
}

// —— 步骤 2 基础整理写操作（命令返回即更新 UI；watcher 回声后端消重，见 services/workspace）——

// 新建文件夹：relDir 下创建 name；重名 / 非法名抛中文错（commands/workspace.rs ws_create_folder）
export async function createWsFolder(relDir: string, name: string): Promise<void> {
  await invoke("ws_create_folder", { relDir, name });
}

// 重命名：relPath 最后一段改为 newName（文件与文件夹同径）；目标重名抛中文错
export async function renameWsEntry(relPath: string, newName: string): Promise<void> {
  await invoke("ws_rename", { relPath, newName });
}

// 删除到系统回收站（多路径一次调用；PRD 06 不进库内回收站）
export async function deleteWsEntries(relPaths: string[]): Promise<void> {
  await invoke("ws_delete", { relPaths });
}

// 移动（拖拽 / 剪切粘贴）：relPaths 移入 destRel；重名自动 " (n)" 规避
export async function moveWsEntries(relPaths: string[], destRel: string): Promise<void> {
  await invoke("ws_move", { relPaths, destRel });
}

// 粘贴：复制（cut=false）或移动（cut=true）进 destRel；剪切成功后前端清剪贴板
export async function pasteWsEntries(relPaths: string[], destRel: string, cut: boolean): Promise<void> {
  await invoke("ws_paste", { relPaths, destRel, cut });
}

// —— 步骤 3 常用固定（视图状态 = .anikit/workspace.json 的 pinned 清单，PRD 02 技术口径）——
// 后端读取时静默剔除失效条目（删除 / 改名 / 非目录）并回写；未开空间一律返回 []

// 当前固定清单（rel 路径，固定顺序）
export async function listWsPinned(): Promise<string[]> {
  return await invoke<string[]>("ws_list_pinned");
}

// 固定目录（rel 须为空间内已存在目录；重复固定幂等）；返回最新清单
export async function pinWsEntry(rel: string): Promise<string[]> {
  return await invoke<string[]>("ws_pin", { relPath: rel });
}

// 取消固定；返回最新清单（失效条目读取时顺带清理）
export async function unpinWsEntry(rel: string): Promise<string[]> {
  return await invoke<string[]>("ws_unpin", { relPath: rel });
}

// —— 事件监听（App.vue onMounted 统一调，模块级幂等）——
// fs-changed 写入连发去抖 300ms 合并为一拍重扫（缩略图缓存写回已后端过滤，不触发）；
// thumbs-ready 总是 emit（可能空数组），补丁回填 ws store 对应条目的 thumbPath
let started = false;
let fsTimer: ReturnType<typeof setTimeout> | undefined;

export function initWsListeners(): void {
  if (started) return;
  started = true;
  const ws = useWsStore();
  void ws.ensureRoot();
  void listen("workspace-fs-changed", () => {
    clearTimeout(fsTimer);
    fsTimer = setTimeout(() => void ws.rescanLoaded(), 300);
  });
  void listen<WsThumbReady[]>("workspace-thumbs-ready", (e) => ws.patchThumbs(e.payload));
}
