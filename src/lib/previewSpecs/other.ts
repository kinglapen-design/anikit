// other 不支持预览立姿 spec（R4 按稿 2026-09-21 落地，稿 = 布局-中间面板-内容容器-此文件不支持预览）：
// PreviewUnsupported 自管格式图标 / 文件名 / 提示 / 打开按钮组（ws 合成项隐藏）；
// openDefaultApp / openWithDialog 业务接线在组件内（UI 不含业务逻辑纪律），宿主零业务。
import PreviewUnsupported from "../../components/PreviewUnsupported.vue";
import type { PreviewPaneSpec } from "./types";

export const otherPaneSpec: PreviewPaneSpec = { kind: "other", component: PreviewUnsupported };
