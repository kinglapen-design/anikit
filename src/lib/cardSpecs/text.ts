// 文本卡 spec（稿 = 文件卡片-txt-md）：内容 = 标题 + 正文滚动预览（字号固定稿值，
// 2026-09-22 用户定不随卡缩放）；scrollThumb = 内容溢出滚动胶囊（拇指宿主渲染可拖，
// 接线 TextContent 根滚动容器）；topInset = 格式角标让位（角标下缘 + 4：12 + 20 + 4 = 36）。
import TextContent from "../../components/cards/contents/TextContent.vue";
import type { CardContentSpec } from "./types";

export const textContentSpec: CardContentSpec = {
  content: TextContent,
  contentProps: (p) => ({ textTitle: p.textTitle, textBody: p.textBody, topInset: p.badge ? 36 : 12 }),
  scrollThumb: true,
};
