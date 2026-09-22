// 预览视口组件型分支 spec 注册表（2026-09-21 R5，扩展性口径见 types.ts 头注）：
// kind → 渲染组件；宿主 PreviewPanel 按 preview.kind 查表分发，未注册 = 无组件型分支
// （host 基建分支 img/video/sequence/anim 不经过此表）。
// 新组件型格式（3D / 第三方插件）= 加 spec 文件 + registerPreviewPaneSpec 一行注册。
// R6 首例验证（2026-09-22）：docx = docx.ts + 下方一行，本文件其余与宿主零改动。
import type { PreviewPaneSpec } from "./types";
import { fontPaneSpec } from "./font";
import { textPaneSpec } from "./text";
import { audioWavePaneSpec } from "./audioWave";
import { otherPaneSpec } from "./other";
import { docxPaneSpec } from "./docx";

const registry = new Map<string, PreviewPaneSpec>();

/** 注册某预览 kind 的组件型分支（重复注册后者覆盖，插件扩展点） */
export function registerPreviewPaneSpec(spec: PreviewPaneSpec): void {
  registry.set(spec.kind, spec);
}

/** 取 kind 对应组件型分支 spec（未注册 = null，宿主不渲染组件型分支） */
export function previewPaneSpecFor(kind: string): PreviewPaneSpec | null {
  return registry.get(kind) ?? null;
}

// 内建五族：字体 / 文本 / 音频波形大图 / docx 文档 / other 不支持预览立姿
registerPreviewPaneSpec(fontPaneSpec);
registerPreviewPaneSpec(textPaneSpec);
registerPreviewPaneSpec(audioWavePaneSpec);
registerPreviewPaneSpec(otherPaneSpec);
registerPreviewPaneSpec(docxPaneSpec);

export { PREVIEW_STAGE_KEY } from "./types";
export type { PreviewPaneSpec } from "./types";
