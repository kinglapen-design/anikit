// 播放条 spec 注册表（2026-09-09 预览悬浮栏二轮，扩展性口径见 types.ts 头注）：
// kind → 工厂；宿主 FloatBarHost 按 preview.kind 查表分发，拿不到 = 无播放条（悬浮栏退场）。
// 新格式（3D / 第三方插件）= 加 spec 文件 + registerPlayBarSpec 一行注册。
import type { PlayBarSpec } from "./types";
import { makeVideoSpec } from "./video";
import { makeAudioSpec } from "./audio";
import { makeFramesSpec } from "./frames";

const registry = new Map<string, () => PlayBarSpec>();

/** 注册某预览 kind 的播放条形态（重复注册后者覆盖，插件扩展点） */
export function registerPlayBarSpec(kind: string, factory: () => PlayBarSpec): void {
  registry.set(kind, factory);
}

/** 取 kind 对应播放条 spec（工厂每次新建闭包对象；未注册 = null） */
export function playBarSpecFor(kind: string): PlayBarSpec | null {
  return registry.get(kind)?.() ?? null;
}

// 内建三族：视频全量 / 音频删减 / 序列帧·动图帧播放（同工厂）
registerPlayBarSpec("video", makeVideoSpec);
registerPlayBarSpec("audio", makeAudioSpec);
registerPlayBarSpec("sequence", makeFramesSpec);
registerPlayBarSpec("anim", makeFramesSpec);

export type { PlayBarSpec, FrameStripSpec, PlayGroup } from "./types";
