// docx 预览 spec（R6 新功能验证首例，PRD 04-04 条 6）：PreviewDocx 自管数据加载与空态，宿主直挂零 props。
import PreviewDocx from "../../components/PreviewDocx.vue";
import type { PreviewPaneSpec } from "./types";

export const docxPaneSpec: PreviewPaneSpec = { kind: "docx", component: PreviewDocx };
