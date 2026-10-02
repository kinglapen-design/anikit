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
  hasSubDir?: boolean; // 目录条目：是否含可见子目录（2026-09-26 树展开箭头真实化，列目录时后端顺带扫描）；文件恒 false；缺省（旧缓存）时树回退旧乐观判定
  isLink?: boolean; // 目录联接（junction 方向 b，2026-10-02）：目录 reparse point 即 true（断链也 true——门没了目标仍是门）；文件恒 false；缺省（旧缓存）按非链接
  linkOk?: boolean; // 联接目标当前可达（跟随式探测成功；断链 = false）；非联接恒 false
  size: number; // 文件字节数；目录恒 0
  mtimeMs: number; // 文件修改时间（Unix 毫秒）；目录恒 0
  ctimeMs: number; // 文件创建时间（Unix 毫秒，2026-09-26 详情面板基础数据补）；目录 / 平台不支持恒 0
  w: number | null; // 布局比例来源宽：图片 = 源像素（魔数探测）；非图片缓存新鲜 = 缩略图宽（与源同比例）；无 = null
  h: number | null; // 同上（高）
  ext: string; // 小写无点扩展名；目录 / 无扩展名 = ""
  thumbPath: string | null; // 缩略图缓存绝对路径：内容缩略图新鲜 / 格式图标已缓存时给，否则 null
  thumbKind: "asset" | "formatIcon" | null; // 缩略图来源（同库 ThumbKind 口径；未就绪 = null）
}

// workspace-thumbs-ready 负载条目（services/workspace.rs ThumbReady）
export interface WsThumbReady {
  absPath: string; // 源文件绝对路径
  thumbPath: string; // 缩略图缓存绝对路径
  w: number; // 缩略图实际像素宽（2026-09-26：随回填写条目 w/h，卡片恢复真实比例）；formatIcon 落回项 = 图标边长（忽略）
  h: number; // 同上（高）
  thumbKind: "asset" | "formatIcon"; // 2026-09-26 失败记忆补：asset = 内容缩略图；formatIcon = 解码失败条目落回的系统格式图标
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

// 请求缩略图：立即返回，任务进后端进程级 worker 池（固定并发 + 跨请求去重 + 失败记忆，
// 2026-09-26 性能优化）；priority=true 整批插队队首（视口内缺失项），否则排尾部。
// 完成批次 emit workspace-thumbs-ready（负载含本趟生成成功的内容缩略图 + 失败条目的格式图标落回）
export async function requestWorkspaceThumbs(absPaths: string[], priority = false): Promise<void> {
  await invoke("request_workspace_thumbs", { absPaths, priority });
}

// 在文件管理器中打开并选中（条目右键；约束在当前空间 workspace 内，越界 / 不存在抛中文错）
export async function revealPath(absPath: string): Promise<void> {
  await invoke("reveal_path", { absPath });
}

// —— 打开类（2026-10-01 库右键菜单移植，按真实路径 + 后端 is_under 越界校验同径）——

// 用系统默认程序打开（库「在默认应用打开」按路径版）
export async function openPathWithDefaultApp(absPath: string): Promise<void> {
  await invoke("open_path_with_default_app", { absPath });
}

// 在其他应用打开（系统「打开方式」对话框；openas 动词缺失时后端 rundll32 兜底）
export async function openPathWithDialog(absPath: string): Promise<void> {
  await invoke("open_path_with_dialog", { absPath });
}

// 复制 base64（复制数据子菜单移植；格式白名单 png/jpg/jpeg/bmp/webp/gif + psd/psb/svg 解码合成图、
// 10MB 上限同库侧口径，不满足抛中文错）
export async function wsFileBase64(absPath: string): Promise<string> {
  return await invoke<string>("ws_file_base64", { absPath });
}

// 系统右键菜单（Windows 原生 IContextMenu：压缩解压 / TortoiseGit 等 shell 扩展全量；多选整组弹出，
// 后端在光标处模态弹出，命令返回 = 菜单已关且选中命令已执行；取消 = 正常返回）
export async function wsShellMenu(absPaths: string[]): Promise<void> {
  await invoke("ws_shell_menu", { absPaths });
}

// 系统右键菜单 - 目录背景版（空白处右键；经典版 新建 / 排序方式 / 查看 / 刷新 / 属性 在其中）
export async function wsShellMenuBackground(absPath: string): Promise<void> {
  await invoke("ws_shell_menu_background", { absPath });
}

// 打开系统回收站（工作空间侧栏回收站行，2026-10-01；ws 删除 = 系统回收站，PRD 06）
export async function openSystemRecycleBin(): Promise<void> {
  await invoke("open_system_recycle_bin");
}

// 发送至 Animate（8.3 按路径入口，仅 fla；与库素材共用后端 send_fla 流程，返回素材名）
export async function sendPathToAnimate(absPath: string): Promise<string> {
  return await invoke<string>("send_path_to_animate", { absPath });
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

// 移动（拖拽 / 剪切粘贴）：relPaths 移入 destRel；重名自动 "名_n" 规避（n 从 2 递增，2026-09-23 序号统一定案）
export async function moveWsEntries(relPaths: string[], destRel: string): Promise<void> {
  await invoke("ws_move", { relPaths, destRel });
}

// 移动可行性过滤（引擎落点判定与 store.move 同口径，2026-10-02 文件夹拖拽移动接入时立）：
// 剔除三类不可移项——① 同父（目标 = 源所在目录 = 空移，防 unique_target 造「名_2」副本）；
// ② 目标 = 源自身；③ 目标在源子树内（②③ = 后端 ws_move「不能移动到自身 / 自己的子目录」
// 护栏的前端前置——全剔完 = 落点不高亮、松手无操作，而非报错 toast）
export function wsMovableRels(rels: string[], destRel: string): string[] {
  return rels.filter((r) => {
    if (r === destRel || destRel.startsWith(`${r}/`)) return false;
    const parent = r.includes("/") ? r.slice(0, r.lastIndexOf("/")) : "";
    return parent !== destRel;
  });
}

// 粘贴：复制（cut=false）或移动（cut=true）进 destRel；剪切成功后前端清剪贴板
export async function pasteWsEntries(relPaths: string[], destRel: string, cut: boolean): Promise<void> {
  await invoke("ws_paste", { relPaths, destRel, cut });
}

// 创建副本（库「创建副本」Ctrl+D 移植）：relPaths 逐个复制进自身所在目录，重名自动 _n 规避
export async function duplicateWsEntries(relPaths: string[]): Promise<void> {
  await invoke("ws_duplicate", { relPaths });
}

// —— 2026-10-01 ws ↔ Explorer 双向拖拽 + 剪贴板互通（PRD 02-13）——

// 系统剪贴板文件清单（ws_clipboard_files；菜单「粘贴」可见性查询用）
export interface WsClipboardFiles {
  paths: string[]; // CF_HDROP 文件绝对路径（无文件 = 空）
  cut: boolean; // Preferred DropEffect=MOVE（外部剪切 = 移动语义）
}

// ws_paste_system 结果
export interface WsPasteSystemResult {
  pasted: string[]; // 实际落入的目标 rel 集（源已不存在项跳过；全失效 = 空，调用方 toast）
  moved: boolean; // 外部剪切 = 移动入（同盘 rename / 异盘复制后删源）
  hadFiles: boolean; // 系统剪贴板含文件列表（false = 无文件属常态，调用方静默）
}

// 写系统剪贴板（复制 / 剪切同步：CF_HDROP + 单张静态图 ≤20MB 附位图，库同口径；
// 剪切附 Preferred DropEffect=MOVE——资源管理器粘贴即移动）。relPaths 后端逐条校验（失败跳过）
export async function wsWriteClipboard(relPaths: string[], cut: boolean): Promise<void> {
  await invoke("ws_write_clipboard", { relPaths, cut });
}

// 读系统剪贴板文件清单（右键开 ws 菜单前 refreshSystemClip 调本接口）
export async function wsClipboardFiles(): Promise<WsClipboardFiles> {
  return await invoke<WsClipboardFiles>("ws_clipboard_files");
}

// 系统剪贴板粘贴进 destRel（粘贴仲裁外部分支：复制来源恒复制 / 剪切来源 = 移动）
export async function wsPasteSystem(destRel: string): Promise<WsPasteSystemResult> {
  return await invoke<WsPasteSystemResult>("ws_paste_system", { destRel });
}

// Explorer 拖入（恒 copy，非破坏口径）：外部绝对路径复制进 destRel；返回 created rel 集
export async function wsImportPaths(absPaths: string[], destRel: string): Promise<string[]> {
  return await invoke<string[]>("ws_import_paths", { absPaths, destRel });
}

// 系统剪贴板序号（粘贴仲裁基准：序号变 = 外部覆盖；placementOps 库侧 2026-09-19 同源调用）
export async function getClipboardSeq(): Promise<number> {
  return await invoke<number>("get_clipboard_seq");
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

// —— junction 挂载（方向 b：外部文件夹挂进 WS 子目录，2026-10-02，commands/workspace.rs）——

// 挂载外部文件夹：targetPath（外部目录绝对路径）以 junction 挂进 destRel 下，挂载名 = 目标目录名
//（重名自动 _n 规避）；返回挂载点 rel。拒绝（target 非目录 / 在工作空间内 / 是祖先形成循环）抛中文错
export async function addWsLink(destRel: string, targetPath: string): Promise<string> {
  return await invoke<string>("ws_add_link", { destRel, targetPath });
}

// 解除挂载：链接本体进系统回收站（只删「门」不动外部目标）；断链同样可移除；非链接条目抛中文错
export async function removeWsLink(relPath: string): Promise<void> {
  await invoke("ws_remove_link", { relPath });
}

// 改挂目标：链接重指向 newTarget（校验同挂载——目录 / 不在工作空间内 / 非祖先）；断链链接走本径「送修」
export async function retargetWsLink(relPath: string, newTarget: string): Promise<void> {
  await invoke("ws_retarget_link", { relPath, newTarget });
}

// 从 WS 里「链出去」（方向 a′ 产品化，2026-10-02）：rel 文件夹在外部 destDir 下摆入口 junction
//（入口名 = 文件夹名，重名后端 _n），返回入口绝对路径。**只管创建不管存续**——root 外不登记
// 不监听。校验拒绝（源非文件夹 / 断链门 / 落点缺失 / 落点在储存空间内）抛中文错
export async function linkOutWsFolder(relPath: string, destDir: string): Promise<string> {
  return await invoke<string>("ws_link_out", { relPath, destDir });
}

// —— 事件监听（App.vue onMounted 统一调，模块级幂等）——
// fs-changed（2026-10-01 精准化，性能优化①）：负载 = 变更涉及的目录 rel 数组（去重排序，
// 父目录 + 祖父目录），写入连发去抖 300ms 合并——窗口内多拍目录集取并集，按集精准重扫
// （仅已加载目录；未加载目录下次展开 / 进视口时 loadDir 自然取新）；负载 null = 后端路径
// 解析失败的形态异常，整窗回退全量重扫（rescanLoaded）。
// thumbs-ready 总是 emit（可能空数组），补丁回填 ws store 对应条目的 thumbPath
let started = false;
let fsTimer: ReturnType<typeof setTimeout> | undefined;
let fsAll = false; // 去抖窗口内混入 null 拍 → 整窗回退全量
const fsDirs = new Set<string>(); // 去抖窗口内变更目录并集

export function initWsListeners(): void {
  if (started) return;
  started = true;
  const ws = useWsStore();
  void ws.ensureRoot();
  void listen<string[] | null>("workspace-fs-changed", (e) => {
    if (e.payload === null) {
      fsAll = true;
      fsDirs.clear();
    } else if (!fsAll) {
      for (const d of e.payload) fsDirs.add(d);
    }
    clearTimeout(fsTimer);
    fsTimer = setTimeout(() => {
      const all = fsAll;
      const dirs = [...fsDirs];
      fsAll = false;
      fsDirs.clear();
      if (all) void ws.rescanLoaded();
      else if (dirs.length > 0) void ws.rescanDirs(dirs);
    }, 300);
  });
  void listen<WsThumbReady[]>("workspace-thumbs-ready", (e) => ws.patchThumbs(e.payload));
}
