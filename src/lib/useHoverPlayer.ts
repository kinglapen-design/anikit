// 悬停播放层骨架 composable（5.3，自 HoverVideoLayer / HoverAudioLayer / HoverSequenceLayer
// 三份逐字相同骨架收编）：进入延迟（防快速划过 / 快速滚动触发加载与转码）、
// hoverPreview 活跃实例注册（主窗口隐藏到托盘暂停 / 重新显示恢复全部活跃悬停播放）、
// disposed 护栏（异步加载返回时组件已卸载则不碰 media 元素）、卸载统一清理。
// 差异逻辑（media 元素挂载 / scrub 范围循环 / 波形 seek / 序列帧轮播）留各组件。
import { onBeforeUnmount, onMounted } from "vue";
import { registerHoverPlayer } from "../stores/hoverPreview";

// 进入延迟默认值（毫秒）：悬停触发防划过 / 快速滚动触发加载。
// 自动播放触发（IntersectionObserver 可见性驱动，本身即意图信号）由调用方传更短值提速——
// 2026-08-23 用户实机：滚回视口的卡固定等满 300ms 才开始加载，恢复感知半秒+
export const HOVER_ENTER_DELAY = 300;
// 自动播放触发的进入延迟：100ms 滤掉飞速滚过（一屏瞬时穿过的卡不加载），比悬停路径砍 200ms 纯等待
export const AUTOPLAY_ENTER_DELAY = 100;

export function useHoverPlayer(opts: {
  load: () => Promise<void>; // 延迟到达后执行的源加载（内部用 isDisposed 做卸载护栏）
  pause: () => void; // 注册进 hoverPreview 的暂停回调（主窗口隐藏时调）
  resume: () => void; // 恢复回调（窗口重新显示时调；实例活着 = 卡片仍在视口/悬停态未消，直接续播）
  cleanup: () => void; // 卸载清理：停播 / 卸载源 / 停轮播等
  enterDelay?: number; // 进入延迟（毫秒），缺省 HOVER_ENTER_DELAY
}): { isDisposed: () => boolean } {
  let timer: ReturnType<typeof setTimeout> | undefined;
  let disposed = false;
  let unregister: (() => void) | undefined;

  onMounted(() => {
    unregister = registerHoverPlayer(opts.pause, opts.resume);
    timer = setTimeout(() => void opts.load(), opts.enterDelay ?? HOVER_ENTER_DELAY);
  });
  onBeforeUnmount(() => {
    disposed = true;
    clearTimeout(timer);
    unregister?.();
    opts.cleanup();
  });

  return { isDisposed: () => disposed };
}
