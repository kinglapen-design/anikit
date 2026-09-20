// 通用卡内容 spec（稿 = 文件卡片-通用）：缩略图充满（cover 默认；square = shell 缩略图 /
// 未导入卡 = 正方形 + 50% 居中）；无附加件。
import ThumbContent from "../../components/cards/contents/ThumbContent.vue";
import type { CardContentSpec } from "./types";

export const generalContentSpec: CardContentSpec = {
  content: ThumbContent,
  contentProps: (p) => ({ thumbUrl: p.thumbUrl, thumbFit: p.thumbFit, square: p.square }),
};
