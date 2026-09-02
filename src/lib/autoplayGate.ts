// 自动播放数量闸（2.3 导入预览性能优化）：开关驱动的卡片自动播放（动图 / 视频 / 序列帧）
// 须申请名额，上限内先到先得、满员排队 FIFO 唤醒；悬停播放直通不走闸（用户意图，同时仅个位数）。
// 背景：小卡片高密度时虚拟滚动窗口内 200+ 卡片同时自动播放，webview 逐帧解码饱和致全局卡顿
// （hover 目录树滞后，2026-08-12 用户实测）。
// 分池（2026-08-12 用户定：6列×7行=42 也要能看）：动图/序列帧单实例便宜（原生 <img> / 帧图轮播）上限 50；
// 视频每个实例一个 <video> 解码器，上限 16。
// 可见性联动（同日用户实机：纯 FIFO 无可见性感知，名额被缓冲区不可见卡占住、可见卡排队等死）——
// 调用方（AssetCard）经 IntersectionObserver 只在可见时申请、滚出视口即释放，名额始终给可见卡。
export type AutoplayPool = "anim" | "video";

interface Pool {
  max: number;
  holders: Set<string>;
  waiters: Map<string, () => void>;
}

const pools: Record<AutoplayPool, Pool> = {
  anim: { max: 50, holders: new Set(), waiters: new Map() },
  video: { max: 16, holders: new Set(), waiters: new Map() },
};

/** 申请名额：立即持有返回 true；满员则排队返回 false，名额让出时 onGranted 同步调用（调用即已持有） */
export function acquireAutoplay(key: string, pool: AutoplayPool, onGranted: () => void): boolean {
  const p = pools[pool];
  if (p.holders.has(key)) return true;
  if (p.holders.size < p.max) {
    p.holders.add(key);
    return true;
  }
  p.waiters.set(key, onGranted);
  return false;
}

/** 释放名额（持有与排队两种状态都安全）；持有释放后 FIFO 唤醒队首 */
export function releaseAutoplay(key: string, pool: AutoplayPool): void {
  const p = pools[pool];
  if (!p.holders.delete(key)) {
    p.waiters.delete(key);
    return;
  }
  const first = p.waiters.entries().next();
  if (first.done) return;
  const [nextKey, wake] = first.value;
  p.waiters.delete(nextKey);
  p.holders.add(nextKey);
  wake();
}
