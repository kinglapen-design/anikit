<script setup lang="ts">
// 序列帧卡片播放层（5.3，2026-07-27 用户定：序列帧也是动图范围，打包就是为了看动画效果）：
// 悬停播放与"自动播放动图"开关同行为——按 fps（metadata.fps，缺省 24）轮播 info/frames/ 帧原图
// （fullPath，不碰帧条小图与预览解析队列——队列"提交即清空 pending"语义会被卡片批量调用互相顶）。
// 无控件；进入延迟防划过 / 快速滚动（自动播放触发走 100ms 短档，2026-08-23 滚回恢复提速）。
import { ref } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { AUTOPLAY_ENTER_DELAY, HOVER_ENTER_DELAY, useHoverPlayer } from "../lib/useHoverPlayer";
import { listSequenceFrames } from "../lib/mediaOps";
import { useUiStore } from "../stores/ui";
import type { PlacementView } from "../stores/content";

// auto = 自动播放触发（视口可见性驱动）；缺省 false = 悬停触发；
// startFrame = 语义命中原始帧（2026-11-02 用户实机定，AssetCard 定位窄段透传）——
// 起播索引优先于封面帧；null = 无命中走原口径
const props = withDefaults(defineProps<{ asset: PlacementView; auto?: boolean; startFrame?: number | null }>(), { auto: false, startFrame: null });
const ui = useUiStore();

const frames = ref<string[]>([]);
const idx = ref(0);

let interval: ReturnType<typeof setInterval> | undefined;
let fps = 24; // loadFrames 载入后记录（resume 重启轮播复用）

// 悬停播放骨架（进入延迟 / 活跃实例注册 / disposed 护栏 / 卸载清理）收编 useHoverPlayer
const { isDisposed } = useHoverPlayer({
  load: loadFrames,
  pause: stop,
  resume: () => {
    if (frames.value.length > 0) start();
  },
  cleanup: stop,
  enterDelay: props.auto ? AUTOPLAY_ENTER_DELAY : HOVER_ENTER_DELAY,
});

async function loadFrames(): Promise<void> {
  try {
    const r = await listSequenceFrames(props.asset.assetId);
    if (isDisposed() || r.frames.length === 0) return;
    frames.value = r.frames.map((f) => convertFileSrc(f.fullPath));
    // 起播索引：语义命中帧优先（startFrame 定位窄段透传）；无命中回退封面帧（thumbFrame = 帧名），未设置从头
    const hit = props.startFrame;
    const cover = r.thumbFrame ? r.frames.findIndex((f) => f.name === r.thumbFrame) : -1;
    idx.value = hit != null && hit >= 0 && hit < r.frames.length ? hit : cover >= 0 ? cover : 0;
    fps = r.fps ?? 24;
    start();
  } catch (e) {
    console.warn("序列帧清单加载失败：", e);
  }
}

function start(): void {
  stop();
  interval = setInterval(() => {
    idx.value = (idx.value + 1) % frames.value.length;
  }, 1000 / fps);
}

function stop(): void {
  clearInterval(interval);
  interval = undefined;
}
</script>

<template>
  <div v-if="frames.length > 0" class="hover-sequence">
    <img class="v" :class="{ contain: ui.layoutMode === 'grid' }" :src="frames[idx]" :alt="asset.name" draggable="false" />
  </div>
</template>

<style scoped>
.hover-sequence {
  position: absolute;
  inset: 0;
  /* 与卡片图同 C1 底：帧图透明通道盖不住底层静态封面会双重影像（2026-07-27 用户实测） */
  background: var(--n-g1);
}
.v {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.v.contain {
  object-fit: contain;
}
</style>
