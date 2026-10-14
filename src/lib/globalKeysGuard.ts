// 全局键共享让位判定（2026-10-15 快捷键全量点亮轮拆出——useGlobalKeys 主文件近 500 行硬上限，
// 打开类 / 新建类两组新键各自成文件，共用本守卫）：
// 输入框文本编辑态（input / textarea / contenteditable）/ 预览子窗口 / 激活 panel 预览中 /
// 库向导态 / 菜单·浮层·弹窗打开时 —— 全局键一律不抢（与 onOperationKeys 既有口径一致）。
// isEditableTarget 同迁于此（useGlobalKeys re-export，App / AssetGrid / PreviewPanel 既有引用不破）。
import { activePreviewSession } from "../stores/preview";

export interface GlobalKeysOpts {
  previewParam: string | null;
  needLibrarySetup: () => boolean;
}

// 输入框豁免判定：文本输入态不拦截全局快捷键（含 CM6 contenteditable）
export function isEditableTarget(t: HTMLElement): boolean {
  return t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable;
}

// 返回 true = 本次按键让位（不响应任何全局动作）；选择模式（picker）禁放行由各处理器按
// 编辑类 / 查看类自行判定（编辑类全禁；查看类如 "/" 展开收起放行，onViewKeys 同口径）
export function globalKeysYield(e: KeyboardEvent, opts: GlobalKeysOpts): boolean {
  if (isEditableTarget(e.target as HTMLElement)) return true;
  const preview = activePreviewSession(); // 事件时刻取激活 panel 会话（只查不建）
  if (opts.previewParam || preview?.assetKey || opts.needLibrarySetup()) return true;
  return !!document.querySelector(".app-menu, .zoom-mask, .kit-mask, .bm-mask");
}
