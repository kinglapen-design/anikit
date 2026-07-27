<script setup lang="ts">
// 序列帧卡片播放层（5.3，2026-07-27 用户定：序列帧也是动图范围，打包就是为了看动画效果）：
// 悬停播放与"自动播放动图"开关同行为——按 fps（metadata.fps，缺省 24）轮播 info/frames/ 帧原图
// （fullPath，不碰帧条小图与预览解析队列——队列"提交即清空 pending"语义会被卡片批量调用互相顶）。
// 无控件；300ms 进入延迟防划过 / 快速滚动。
import { onBeforeUnmount, onMounted, ref } from "vue";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { registerHoverPlayer } from "../stores/hoverPreview";
import { useUiStore } from "../stores/ui";
import type { PlacementView } from "../stores/content";

const props = defineProps<{ asset: PlacementView }>();
const ui = useUiStore();

// 进入延迟（毫秒）：防快速划过 / 滚动触发帧清单加载
const ENTER_DELAY = 300;

interface FrameEntry {
  name: string;
  fullPath: string;
  stripPath: string | null;
  delayMs: number | null;
}
interface SequenceFrames {
  fps: number | null;
  thumbFrame: string | null;
  frames: FrameEntry[];
  unpacking: boolean;
}

const frames = ref<string[]>([]);
const idx = ref(0);

let timer: ReturnType<typeof setTimeout> | undefined;
let interval: ReturnType<typeof setInterval> | undefined;
let disposed = false;
let unregister: (() => void) | undefined;

onMounted(() => {
  unregister = registerHoverPlayer(() => stop());
  timer = setTimeout(() => void loadFrames(), ENTER_DELAY);
});

async function loadFrames(): Promise<void> {
  try {
    const r = await invoke<SequenceFrames>("list_sequence_frames", { assetId: props.asset.assetId });
    if (disposed || r.frames.length === 0) return;
    frames.value = r.frames.map((f) => convertFileSrc(f.fullPath));
    // 从封面帧开始播（thumbFrame = 帧名），未设置从头
    const cover = r.thumbFrame ? r.frames.findIndex((f) => f.name === r.thumbFrame) : -1;
    idx.value = cover >= 0 ? cover : 0;
    const fps = r.fps ?? 24;
    interval = setInterval(() => {
      idx.value = (idx.value + 1) % frames.value.length;
    }, 1000 / fps);
  } catch (e) {
    console.warn("序列帧清单加载失败：", e);
  }
}

function stop(): void {
  clearInterval(interval);
  interval = undefined;
}

onBeforeUnmount(() => {
  disposed = true;
  clearTimeout(timer);
  stop();
  unregister?.();
});
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
  background: var(--c1);
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
