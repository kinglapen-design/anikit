// 动图卡悬停层 spec（5.1/5.3：GIF / WEBP 动图）——AnimImgLayer 原生 <img> 播放
//（悬停 300ms / 自动播放 100ms 进入延迟收编 useHoverPlayer，anim 名额池）。
import AnimImgLayer from "../../components/AnimImgLayer.vue";
import type { HoverLayerSpec } from "./types";

export const animHoverSpec: HoverLayerSpec = {
  layer: AnimImgLayer,
  autoplay: true,
};
