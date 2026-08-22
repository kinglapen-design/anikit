// 悬停播放层骨架 composable（5.3，自 HoverVideoLayer / HoverAudioLayer / HoverSequenceLayer
// 三份逐字相同骨架收编；视频层 2026-08-22 起为 VideoCanvasLayer（帧流），骨架口径不变）：300ms 进入延迟（防快速划过 / 快速滚动触发加载与转码）、
// hoverPreview 活跃实例注册（主窗口隐藏到托盘时暂停全部活跃悬停播放）、disposed 护栏
// （异步加载返回时组件已卸载则不碰 media 元素）、卸载统一清理。
// 差异逻辑（media 元素挂载 / scrub 范围循环 / 波形 seek / 序列帧轮播）留各组件。
import { onBeforeUnmount, onMounted } from "vue";
import { registerHoverPlayer } from "../stores/hoverPreview";

// 进入延迟（毫秒）：防快速划过 / 快速滚动触发加载
const ENTER_DELAY = 300;

export function useHoverPlayer(opts: {
  load: () => Promise<void>; // 延迟到达后执行的源加载（内部用 isDisposed 做卸载护栏）
  pause: () => void; // 注册进 hoverPreview 的暂停回调（主窗口隐藏时调）
  cleanup: () => void; // 卸载清理：停播 / 卸载源 / 停轮播等
}): { isDisposed: () => boolean } {
  let timer: ReturnType<typeof setTimeout> | undefined;
  let disposed = false;
  let unregister: (() => void) | undefined;

  onMounted(() => {
    unregister = registerHoverPlayer(opts.pause);
    timer = setTimeout(() => void opts.load(), ENTER_DELAY);
  });
  onBeforeUnmount(() => {
    disposed = true;
    clearTimeout(timer);
    unregister?.();
    opts.cleanup();
  });

  return { isDisposed: () => disposed };
}
