// 菜单栏「插件」按钮 · 插件菜单 spec（2026-09-06 同日二轮改定：菜单栏钮 = 普通上下文菜单——
// 用户实机走查定「菜单栏不是弹出插件面板，只要这个上下文菜单」；400×450 插件列表浮层保留在
// 工具栏 plugin 图标钮（ContentTitleBar / PreviewTitleBar——稿目录本就叫「工具栏展开窗口」）。
// 三项两组（2026-11-03 撤 8.2 撤销占位清理）：Animate（打开详情子窗口）│
// API 接口文档 / 开发者选项（未立项占位）。8.2 APP 内批量导出缩略图 2026-10-24 用户定撤销，
// 本项随 1.0 收尾第 7 项 T7 摘除。
// 宽度 = 内容自适应（§3 菜单容器口径；本菜单仅三短项，不走 §26 主菜单 250 minW 特例）。
// 选择模式（picker.active）下「插件」钮禁开（调用方闸，同「菜单/编辑」钮口径）。
import type { CtxEntry } from "../contextMenus";
import { openPluginWindow } from "../plugins";

export function pluginMenuSpec(): CtxEntry[] {
  return [
    { label: "Animate", action: () => void openPluginWindow("animate") },
    "divider",
    { label: "API 接口文档", disabled: true },
    { label: "开发者选项", disabled: true },
  ];
}
