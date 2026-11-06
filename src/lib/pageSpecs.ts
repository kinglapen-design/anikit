// 中间面板自定义页面注册表（7.3 T2，2026-11-03）：viewKey 前缀 → 页面组件。
// 新自定义页面（启动页 widget / 3D / 电子书等）= 组件 + 此处一行；ContentArea 顶层查表分发，
// 未命中 = AssetGrid 默认素材网格页。预览态（会话 assetKey 非空）先于本表，不经此分发；
// ws 视口 = 默认网格页（ws 目录数据链由 ContentArea 既有 watch 承担，不属自定义页面）。
import type { Component } from "vue";
import SequencePackView from "../components/SequencePackView.vue";

export interface PageSpec {
  id: string;
  prefix: string; // viewKey 前缀（"seq:" 等；自定义页面建议 "page:{名}"）
  component: Component;
  props?: (viewKey: string) => Record<string, unknown>;
}

const pageSpecs: PageSpec[] = [
  // 序列帧包内视图（5.2）：viewKey = seq:{assetId}——素材区整区换包内帧卡片列表
  { id: "sequence-pack", prefix: "seq:", component: SequencePackView, props: (viewKey) => ({ assetId: viewKey.slice(4) }) },
];

export function pageSpecFor(viewKey: string): PageSpec | null {
  return pageSpecs.find((s) => viewKey.startsWith(s.prefix)) ?? null;
}
