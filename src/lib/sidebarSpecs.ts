// 左栏分段注册表（7.3 T4，2026-11-03）：分段 id → 图标 + 内容组件 + 切分段恢复的视口。
// 新分段（待办 / 天气等自定义树或面板）= 组件 + 此处一条；SidebarLeft 分段与内容分发同表驱动。
// component = null = 空态占位段（功能正在开发中）；viewToOpen 回空串 = 不动中间容器。
import type { Component } from "vue";
import LibraryView from "../components/LibraryView.vue";
import WorkspaceTree from "../components/WorkspaceTree.vue";

export interface SidebarSectionSpec {
  key: string;
  icon: string;
  component: Component | null;
  /** 切分段恢复的视口（停留记忆键由本函数封装）；空串 = 不动中间容器（占位段） */
  viewToOpen(ui: { lastLibView: string; lastWsView: string }): string;
}

export const SIDEBAR_SECTION_SPECS: SidebarSectionSpec[] = [
  { key: "library", icon: "library", component: LibraryView, viewToOpen: (ui) => ui.lastLibView || "quick:all" },
  { key: "workspace", icon: "workspace", component: WorkspaceTree, viewToOpen: (ui) => ui.lastWsView || "ws:" },
  { key: "ai", icon: "ai-chat", component: null, viewToOpen: () => "" },
];

export function sidebarSectionSpecFor(key: string): SidebarSectionSpec {
  return SIDEBAR_SECTION_SPECS.find((s) => s.key === key) ?? SIDEBAR_SECTION_SPECS[0];
}
