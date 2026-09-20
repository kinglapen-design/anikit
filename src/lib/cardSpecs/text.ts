// 文本卡 spec（稿 = 文件卡片-txt-md）：内容 = 标题 + 正文滚动预览（随卡宽 cqw 等比缩放）；
// scrollThumb = 内容溢出滚动胶囊（拇指宿主渲染，接线 TextContent 根滚动容器）。
import TextContent from "../../components/cards/contents/TextContent.vue";
import type { CardContentSpec } from "./types";

export const textContentSpec: CardContentSpec = {
  content: TextContent,
  contentProps: (p) => ({ textTitle: p.textTitle, textBody: p.textBody }),
  scrollThumb: true,
};
