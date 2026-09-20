// 音频卡 spec（稿 = 文件卡片-音频）：内容 = 竖线波形 + 品牌色扫过（进度 clip-path，与根数解耦）；
// 附加件 = 悬停胶囊（时长 + 播放/暂停，无音量钮）；AssetCard 悬停层 = HoverAudioLayer
//（仅悬停触发——音频无"自动播放"开关，悬停即有声播放）。
import HoverAudioLayer from "../../components/HoverAudioLayer.vue";
import WaveContent from "../../components/cards/contents/WaveContent.vue";
import MediaCapsule from "../../components/cards/contents/MediaCapsule.vue";
import type { CardContentSpec, HoverLayerSpec } from "./types";

export const audioContentSpec: CardContentSpec = {
  content: WaveContent,
  contentProps: (p) => ({ peaks: p.peaks, progress: p.progress }),
  overlays: [
    { comp: MediaCapsule, props: (p) => ({ capsuleText: p.capsuleText, playing: p.playing }) },
  ],
};

export const audioHoverSpec: HoverLayerSpec = {
  layer: HoverAudioLayer,
  autoplay: false,
};
