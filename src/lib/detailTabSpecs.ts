// 详情面板 tab / 卡片组件注册表（7.3 T3 本体，2026-11-03）：
// detailSections.ts = 数据层（tab 与卡片 id、标题、排序显隐配置）；本文件 = 组件映射层——
// 新自定义卡片 = detailSections.ts 加 id + 标题 + 此处一行组件映射（previewSpecs 同范式）；
// 新详情 tab = DETAIL_TAB_SPECS 加一条 + 内容组件（DetailSections 分段与内容分发同表驱动）。
import type { Component } from "vue";
import TabBasic from "../components/sections/TabBasic.vue";
import TabLinks from "../components/sections/TabLinks.vue";
import TabCustom from "../components/sections/TabCustom.vue";
import PromptSection from "../components/sections/PromptSection.vue";
import NotesSection from "../components/sections/NotesSection.vue";

/** 详情三 tab（稿序 = 基础信息 / 链接关系 / 自定义组件）：分段控制器 icon + 内容组件同源 */
export const DETAIL_TAB_SPECS = [
  { key: "basic", icon: "basic-info", component: TabBasic },
  { key: "links", icon: "link-relations", component: TabLinks },
  { key: "custom", icon: "custom-component", component: TabCustom },
] as const;

export type DetailTabSpec = (typeof DETAIL_TAB_SPECS)[number];

export function detailTabSpecFor(key: string): DetailTabSpec {
  return DETAIL_TAB_SPECS.find((s) => s.key === key) ?? DETAIL_TAB_SPECS[0];
}

/** 自定义组件 tab 卡片 id → 组件（id 注册表与排序显隐在 detailSections.ts DETAIL_CUSTOM_BLOCKS） */
export const DETAIL_CUSTOM_CARD_COMPONENTS: Record<string, Component> = {
  prompt: PromptSection,
  notes: NotesSection,
};

export function customCardComponentFor(id: string): Component | null {
  return DETAIL_CUSTOM_CARD_COMPONENTS[id] ?? null;
}
