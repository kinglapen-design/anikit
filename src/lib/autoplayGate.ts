// 自动播放数量闸（2.3 导入预览性能优化）：开关驱动的卡片自动播放（动图 / 视频 / 序列帧）
// 须申请名额，上限内先到先得、满员排队 FIFO 唤醒；悬停播放直通不走闸（用户意图，同时仅个位数）。
// 背景：小卡片高密度时虚拟滚动窗口内 200+ 卡片同时自动播放，webview 逐帧解码饱和致全局卡顿
// （hover 目录树滞后，2026-08-12 用户实测）。上限内总解码量有界，与卡片密度解耦。
const MAX_AUTOPLAY = 16;

const holders = new Set<string>();
const waiters = new Map<string, () => void>();

/** 申请名额：立即持有返回 true；满员则排队返回 false，名额让出时 onGranted 同步调用（调用即已持有） */
export function acquireAutoplay(key: string, onGranted: () => void): boolean {
  if (holders.has(key)) return true;
  if (holders.size < MAX_AUTOPLAY) {
    holders.add(key);
    return true;
  }
  waiters.set(key, onGranted);
  return false;
}

/** 释放名额（持有与排队两种状态都安全）；持有释放后 FIFO 唤醒队首 */
export function releaseAutoplay(key: string): void {
  if (!holders.delete(key)) {
    waiters.delete(key);
    return;
  }
  const first = waiters.entries().next();
  if (first.done) return;
  const [nextKey, wake] = first.value;
  waiters.delete(nextKey);
  holders.add(nextKey);
  wake();
}
