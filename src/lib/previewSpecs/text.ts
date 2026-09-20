// 文本预览 spec（5.6）：PreviewText 组件自管（txt 防抖自动保存 / md 只读渲染），宿主直挂零 props。
import PreviewText from "../../components/PreviewText.vue";
import type { PreviewPaneSpec } from "./types";

export const textPaneSpec: PreviewPaneSpec = { kind: "text", component: PreviewText };
