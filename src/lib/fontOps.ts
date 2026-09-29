// 字体命令封装（5.6，规范第 1 条）：get_font_info / install_font / uninstall_font 统一出口。
// 类型以后端序列化结构为唯一来源（src-tauri/src/commands/font.rs；serde camelCase）。
import { invoke } from "@tauri-apps/api/core";

// get_font_info 返回：预览「其它」页元数据 + origin 路径（@font-face 加载源）+ 系统安装状态
export interface FontInfo {
  originPath: string;
  name: string;
  fullName: string;
  family: string;
  style: string;
  version: string;
  glyphCount: number;
  installed: boolean;
  /** cmap 覆盖中文（'永' U+6C38）——卡片示例字分流（2026-09-22 R6 font 接线）：中文四行 / 拉丁三行 */
  hasCjk: boolean;
}

// install_font / uninstall_font 返回（按钮态切换）
export interface FontInstallState {
  installed: boolean;
}

// 字体元数据与安装状态（非字体 / 解析失败抛中文文案）
export async function getFontInfo(assetId: string): Promise<FontInfo> {
  return await invoke<FontInfo>("get_font_info", { assetId });
}

// 按真实路径读字体元数据与安装状态（2026-09-26 工作空间字体卡；越界 / 非字体 / 解析失败抛中文文案）。
// 安装 / 卸载无按路径版（系统级写入只认库资产），ws 卡片不出安装钮
export async function getFontInfoByPath(absPath: string): Promise<FontInfo> {
  return await invoke<FontInfo>("get_font_info_by_path", { absPath });
}

// 安装到系统（用户级：其他软件立即可见）
export async function installFont(assetId: string): Promise<FontInstallState> {
  return await invoke<FontInstallState>("install_font", { assetId });
}

// 从系统卸载（用户级注册表 + 字体目录文件）
export async function uninstallFont(assetId: string): Promise<FontInstallState> {
  return await invoke<FontInstallState>("uninstall_font", { assetId });
}
