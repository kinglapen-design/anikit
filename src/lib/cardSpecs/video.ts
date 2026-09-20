// 视频卡 spec（稿 = 文件卡片-视频）：内容 = 缩略图（同 general）；附加件 = 悬停胶囊
//（帧数 + 播放/暂停 + 音量/静音，muteIcon 档）+ 底部 4px 播放条；AssetCard 悬停层 =
// HoverVideoLayer（自动播放走 video 名额池，悬停 scrub / 自动播放封面帧起 10s 封顶见层内）。
import HoverVideoLayer from "../../components/HoverVideoLayer.vue";
import ThumbContent from "../../components/cards/contents/ThumbContent.vue";
import MediaCapsule from "../../components/cards/contents/MediaCapsule.vue";
import VideoBar from "../../components/cards/contents/VideoBar.vue";
import type { CardContentSpec, HoverLayerSpec } from "./types";

export const videoContentSpec: CardContentSpec = {
  content: ThumbContent,
  contentProps: (p) => ({ thumbUrl: p.thumbUrl, thumbFit: p.thumbFit, square: p.square }),
  overlays: [
    {
      comp: MediaCapsule,
      props: (p) => ({ capsuleText: p.capsuleText, playing: p.playing, muted: p.muted, muteIcon: true }),
    },
    { comp: VideoBar, props: (p) => ({ progress: p.progress }) },
  ],
};

export const videoHoverSpec: HoverLayerSpec = {
  layer: HoverVideoLayer,
  autoplay: true,
};
