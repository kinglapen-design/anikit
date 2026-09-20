// 序列帧卡悬停层 spec（2026-07-27 用户定：序列帧归动图范围，打包就是为了看动画效果）——
// HoverSequenceLayer 按 fps 轮播 info/frames/ 帧原图（悬停 / 自动播放动图开关同行为）。
import HoverSequenceLayer from "../../components/HoverSequenceLayer.vue";
import type { HoverLayerSpec } from "./types";

export const sequenceHoverSpec: HoverLayerSpec = {
  layer: HoverSequenceLayer,
  autoplay: true,
};
