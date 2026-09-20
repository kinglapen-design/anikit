// 字体预览 spec（5.6）：PreviewFont 组件自管（FontFace 加载 / 安装圆钮 / 分段页），宿主直挂零 props。
import PreviewFont from "../../components/PreviewFont.vue";
import type { PreviewPaneSpec } from "./types";

export const fontPaneSpec: PreviewPaneSpec = { kind: "font", component: PreviewFont };
