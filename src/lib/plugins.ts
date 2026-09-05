// 插件注册表（阶段七 工具栏展开窗口-插件 稿，2026-09-06 插件面板轮）：内置插件数据源。
// 插件列表面板（plugins/PluginListPanel）按本清单 + ui.pluginOrder 渲染；详情子窗口经
// open_plugin_window(plugin_id) 打开（后端白名单，目前仅 "animate"）。
// 「批量生成缩略图」业务未立项 = disabled 占位（名称/图标降四级、不响应点击）。
// 图标（命名对照十一批入库）：animate-plugin = Animate 品牌彩色原值；
// batch-thumbnails = 玻璃质感位图图标（用户直投 批量生产缩略图-icon，base64 内嵌原值）。
import { invoke } from "@tauri-apps/api/core";
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
  { id: "batch-thumbnails", name: "批量生成缩略图", icon: "batch-thumbnails", category: "桥接", disabled: true },
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
