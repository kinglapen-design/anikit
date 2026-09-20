// 卡片扩展点注册表（2026-09-21 R5 扩展点注册表化 · 卡片轮，范式 = src/lib/barSpecs/；
// 扩展性口径见 types.ts 头注）：两张表——contentRegistry（FileCard kind → 内容形态，
// 未注册宿主回退 general）与 hoverRegistry（AssetCard kind → 悬停播放层，未注册 = 无悬停层）。
// spec 为无状态组件引用对象（区别于 barSpecs 的工厂闭包——播放条需逐实例捕获 store）。
// 新增卡片形态（3D 卡 / 插件格式卡）= 加 spec 文件 + 下方一行注册，宿主零改动。
import type { CardContentSpec, HoverLayerSpec } from "./types";
import { generalContentSpec } from "./general";
import { videoContentSpec, videoHoverSpec } from "./video";
import { audioContentSpec, audioHoverSpec } from "./audio";
import { fontContentSpec } from "./font";
import { textContentSpec } from "./text";
import { animHoverSpec } from "./anim";
import { sequenceHoverSpec } from "./sequence";

const contentRegistry = new Map<string, CardContentSpec>();
const hoverRegistry = new Map<string, HoverLayerSpec>();

/** 注册某 FileCard kind 的内容形态（重复注册后者覆盖，插件扩展点） */
export function registerCardContentSpec(kind: string, spec: CardContentSpec): void {
  contentRegistry.set(kind, spec);
}

/** 取 kind 内容 spec（未注册 = null，宿主回退 general） */
export function cardContentSpecFor(kind: string): CardContentSpec | null {
  return contentRegistry.get(kind) ?? null;
}

/** 注册某 AssetCard kind 的悬停播放层（重复注册后者覆盖，插件扩展点） */
export function registerHoverLayerSpec(kind: string, spec: HoverLayerSpec): void {
  hoverRegistry.set(kind, spec);
}

/** 取 kind 悬停层 spec（未注册 / other = null = 无悬停层） */
export function hoverLayerSpecFor(kind: string): HoverLayerSpec | null {
  return hoverRegistry.get(kind) ?? null;
}

// 内建七族：内容五（general / video / audio / font / text）+ 悬停四（video / audio / anim / sequence）
registerCardContentSpec("general", generalContentSpec);
registerCardContentSpec("video", videoContentSpec);
registerCardContentSpec("audio", audioContentSpec);
registerCardContentSpec("font", fontContentSpec);
registerCardContentSpec("text", textContentSpec);
registerHoverLayerSpec("video", videoHoverSpec);
registerHoverLayerSpec("audio", audioHoverSpec);
registerHoverLayerSpec("anim", animHoverSpec);
registerHoverLayerSpec("sequence", sequenceHoverSpec);

export type { CardContentOverlay, CardContentSpec, FileCardProps, HoverLayerSpec } from "./types";
