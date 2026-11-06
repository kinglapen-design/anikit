// 插件注册表（阶段七 工具栏展开窗口-插件 稿，2026-09-06 插件面板轮）：内置插件数据源。
// 插件列表面板（plugins/PluginListPanel）按本清单 + ui.pluginOrder 渲染；详情子窗口经
// open_plugin_window(plugin_id) 打开（后端白名单 = animate / mcp）。
// 「AI 外接通道（MCP）」（2026-09-20，待办细节清单 09-20 立项；UI 显示名 = MCP 连接器——同日用户定
// 去括号）：详情窗 = 服务总开关（启停 17701）+ 允许写入开关（写入闸，从偏好设置弹窗挪入）+ 使用教程/
// 功能列表/路径；状态与开关走 commands/mcp.rs 三命令（配置 settings.mcp 子树，不经 ui store 管道——
// 子窗口双写竞态）。
// 图标（命名对照十一批入库）：animate-plugin = Animate 品牌彩色原值；
// mcp = MCP 官方插头对接 logo（十三批用户直投，黑线条转 currentColor）。
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { useToastStore } from "../stores/toast";

export interface PluginMeta {
  id: string;
  name: string;
  icon: string; // lib/icons 文件名干
  category: string; // PLUGIN_CATEGORIES 之一
  disabled?: boolean; // 业务未立项占位：不响应点击 / 更多钮禁用
}

// 分类五档（稿「插件分类-按钮」：桥接默认选中）
export const PLUGIN_CATEGORIES = ["桥接", "格式", "属性", "效率", "其他"] as const;

export const PLUGINS: readonly PluginMeta[] = [
  { id: "animate", name: "Animate 插件", icon: "animate-plugin", category: "桥接" },
  { id: "mcp", name: "MCP 连接器", icon: "mcp", category: "桥接" }, // UI 显示名去括号（2026-09-20 用户定「名字不要加括号」；文档术语 AI 外接通道（MCP）不变）
  { id: "paste-link", name: "编辑器贴图直链", icon: "insert-link", category: "桥接" }, // 2026-10-23；图标 insert-link 占位（待用户补源随批换）
];

// CEP 安装状态（commands/cep.rs；install_dir = 2026-09-06 后端新增字段，详情窗「路径」组显示）
export interface CepInstallStatus {
  sourceAvailable: boolean;
  installed: boolean;
  installedVersion: string | null;
  bundledVersion: string;
  debugModeReady: boolean;
  installDir: string;
}

export async function getCepInstallStatus(): Promise<CepInstallStatus | null> {
  try {
    return await invoke<CepInstallStatus>("get_cep_install_status");
  } catch {
    return null;
  }
}

// 打开插件详情子窗口（后端单例聚焦；结果 toast 在本封装内）
export async function openPluginWindow(pluginId: string): Promise<void> {
  const toast = useToastStore();
  try {
    await invoke("open_plugin_window", { pluginId });
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  }
}

// CEP 插件安装 / 卸载（install_cep_plugin / uninstall_cep_plugin 均返回最新状态；toast 在本封装内）
let cepBusy = false;
export async function installCepPlugin(): Promise<CepInstallStatus | null> {
  if (cepBusy) return null;
  const toast = useToastStore();
  cepBusy = true;
  try {
    const st = await invoke<CepInstallStatus>("install_cep_plugin");
    toast.notify({ message: `CEP 插件已安装（v${st.bundledVersion}），重启 Animate 后在 窗口 > 扩展 中打开`, icon: "success" });
    return st;
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
    return null;
  } finally {
    cepBusy = false;
  }
}

export async function uninstallCepPlugin(): Promise<CepInstallStatus | null> {
  if (cepBusy) return null;
  const toast = useToastStore();
  cepBusy = true;
  try {
    const st = await invoke<CepInstallStatus>("uninstall_cep_plugin");
    toast.notify({ message: "CEP 插件已卸载", icon: "success" });
    return st;
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
    return null;
  } finally {
    cepBusy = false;
  }
}

// —— AI 外接通道（MCP，commands/mcp.rs）——
// 状态与两 set 命令回包同构：调用方动作后直接以回包校正本地态；toast 在本封装内

export interface McpStatus {
  running: boolean; // server 实际在跑
  port: number | null; // 实际监听端口（17701 被占回退动态）
  url: string | null; // 接入地址 http://127.0.0.1:{port}/mcp
  serverEnabled: boolean; // 服务总开关配置（settings.mcp）
  writeEnabled: boolean; // 写入闸配置（settings.mcp）
}

export async function getMcpStatus(): Promise<McpStatus | null> {
  try {
    return await invoke<McpStatus>("get_mcp_status");
  } catch {
    return null;
  }
}

// 服务总开关（启停 17701；先启停后持久化，启动失败回错误不回写开态）
export async function setMcpServerEnabled(enabled: boolean): Promise<McpStatus | null> {
  const toast = useToastStore();
  try {
    const st = await invoke<McpStatus>("set_mcp_server_enabled", { enabled });
    toast.notify({
      message: st.running ? `MCP 服务已启动（${st.url}）` : "MCP 服务已停止",
      icon: "success",
    });
    return st;
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
    return null;
  }
}

// 写入闸（只读/写入切换；持久化即生效，写工具逐次直读）
export async function setMcpWriteEnabled(enabled: boolean): Promise<McpStatus | null> {
  const toast = useToastStore();
  try {
    const st = await invoke<McpStatus>("set_mcp_write_enabled", { enabled });
    toast.notify({ message: enabled ? "已允许 MCP 写入" : "MCP 已切回只读", icon: "success" });
    return st;
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
    return null;
  }
}

// —— 插件 WS 通道（commands/plugin_server.rs，2026-09-25，MCP 同款服务总开关）——
// 状态与 set 回包同构：调用方动作后直接以回包校正本地态；toast 在本封装内

export interface PluginServerStatus {
  running: boolean; // server 实际在跑
  port: number | null; // 实际监听端口（127.0.0.1 动态端口，running 时有值）
  serverEnabled: boolean; // 服务总开关配置（settings.pluginServer）
}

export async function getPluginServerStatus(): Promise<PluginServerStatus | null> {
  try {
    return await invoke<PluginServerStatus>("get_plugin_server_status");
  } catch {
    return null;
  }
}

// 服务总开关（先启停后持久化，启动失败回错误不回写开态）
export async function setPluginServerEnabled(enabled: boolean): Promise<PluginServerStatus | null> {
  const toast = useToastStore();
  try {
    const st = await invoke<PluginServerStatus>("set_plugin_server_enabled", { enabled });
    toast.notify({
      message: st.running ? `插件通信服务已启动（127.0.0.1:${st.port}）` : "插件通信服务已停止",
      icon: "success",
    });
    return st;
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
    return null;
  }
}

// —— 编辑器贴图直链（2026-10-23，commands/paste_link.rs；PRD 05-01「编辑器贴图直链」条 + 02-09「by-name 形态」条）——
// 贴图工具 anikit-paste.exe 随包分发；Obsidian 插件文件嵌 exe 字节（include_bytes），安装时写出——
// portable / dev 路径解析问题由此根除。toast 在本封装内

export interface PasteLinkInfo {
  command: string; // Typora 自定义上传命令（exe 绝对路径，含引号）
  toolPath: string; // anikit-paste.exe 实际位置
  inboxDir: string | null; // 收件箱真实目录（未打开储存空间 = null）
}

export async function getPasteLinkInfo(): Promise<PasteLinkInfo | null> {
  try {
    return await invoke<PasteLinkInfo>("get_paste_link_info");
  } catch {
    return null;
  }
}

// 安装 Obsidian 插件到指定 vault（返回插件目录 = {vault}/.obsidian/plugins/anikit-paste）
export async function installObsidianPlugin(vaultPath: string): Promise<string | null> {
  const toast = useToastStore();
  try {
    const dir = await invoke<string>("install_obsidian_plugin", { vaultPath });
    toast.notify({ message: "插件已安装，重启 Obsidian 后在 设置 > 第三方插件 启用「AniKit 贴图直链」", icon: "success" });
    return dir;
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
    return null;
  }
}

// 「选择库安装」流程（面板更多菜单 / 详情窗按钮共用）：目录选择器 → 安装
export async function installObsidianPluginFlow(): Promise<void> {
  const picked = await openDialog({ directory: true, multiple: false, title: "选择 Obsidian 库（vault）根目录" });
  if (typeof picked !== "string" || !picked) return;
  await installObsidianPlugin(picked);
}
