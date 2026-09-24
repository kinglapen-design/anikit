// 预览会话注入键（per-panel 会话化，2026-09-25）：ContentArea/PreviewWindow provide，预览族组件 inject
import type { InjectionKey } from "vue";
import type { PreviewSession } from "../stores/preview";
export const PREVIEW_SESSION_KEY: InjectionKey<PreviewSession> = Symbol("preview-session");
