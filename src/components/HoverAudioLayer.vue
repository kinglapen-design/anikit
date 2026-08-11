<script setup lang="ts">
// 音频卡片悬停播放层（5.3，2026-07-27 用户定案，参考图 2）：无悬停按钮，悬停即有声播放。
// 卡片缩略图即波形图（5.1），播放时叠加：已播放部分品牌紫（div 以同一波形图作 CSS mask +
// clip-path 裁到进度，照 PlayBarAudio 方案）+ 1px 品牌紫播放头竖线；
// 点击/拖动波形 seek；右下胶囊 = 当前时间 + 播放/暂停钮。media 元素组件自管。
import { computed, ref } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import AppIcon from "./AppIcon.vue";
import { formatDuration } from "../lib/formatDuration";
import { getMediaSource } from "../lib/mediaOps";
import { useHoverPlayer } from "../lib/useHoverPlayer";
import { useUiStore } from "../stores/ui";
import type { PlacementView } from "../stores/content";

const props = defineProps<{ asset: PlacementView }>();
const ui = useUiStore();

const layerRef = ref<HTMLElement>();
const mediaRef = ref<HTMLVideoElement>(); // 音频复用 video 元素（同 previewMedia 手法）
const playing = ref(false);
const currentTime = ref(0);
const duration = ref(0);

const waveUrl = computed(() => (props.asset.thumbPath ? convertFileSrc(props.asset.thumbPath) : null));
const waveMask = computed(() => (waveUrl.value ? { "--wave-url": `url("${waveUrl.value}")` } : {}));
const pct = computed(() =>
  duration.value > 0 ? Math.min(100, Math.max(0, (currentTime.value / duration.value) * 100)) : 0,
);

let seeking = false;

// 悬停播放骨架（300ms 进入延迟 / 活跃实例注册 / disposed 护栏 / 卸载清理）收编 useHoverPlayer
const { isDisposed } = useHoverPlayer({
  load: loadSource,
  pause: () => mediaRef.value?.pause(),
  cleanup: () => {
    const v = mediaRef.value;
    if (v) {
      v.pause();
      v.removeAttribute("src");
      v.load();
    }
  },
});

async function loadSource(): Promise<void> {
  try {
    const r = await getMediaSource(props.asset.assetId);
    if (isDisposed()) return;
    const v = mediaRef.value;
    if (!v) return;
    v.src = convertFileSrc(r.path);
    void v.play().catch((e) => console.warn("悬停播放失败：", e));
  } catch (e) {
    console.warn("悬停播放源解析失败：", e);
  }
}

function togglePlay(): void {
  const v = mediaRef.value;
  if (!v || !v.src) return;
  if (v.paused) void v.play().catch(() => {});
  else v.pause();
}
function seek(e: PointerEvent): void {
  const v = mediaRef.value;
  if (!v || duration.value <= 0) return;
  const rect = layerRef.value!.getBoundingClientRect();
  const ratio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
  v.currentTime = ratio * duration.value;
  currentTime.value = v.currentTime;
}
function onSeekDown(e: PointerEvent): void {
  seeking = true;
  layerRef.value!.setPointerCapture(e.pointerId);
  seek(e);
  e.preventDefault();
}
function onSeekMove(e: PointerEvent): void {
  if (seeking) seek(e);
}
function onSeekUp(e: PointerEvent): void {
  if (!seeking) return;
  seeking = false;
  layerRef.value!.releasePointerCapture(e.pointerId);
}
</script>

<template>
  <div
    ref="layerRef"
    class="hover-audio"
    style="touch-action: none"
    @pointerdown="onSeekDown"
    @pointermove="onSeekMove"
    @pointerup="onSeekUp"
  >
    <video
      ref="mediaRef"
      class="hidden-media"
      preload="auto"
      @play="playing = true"
      @pause="playing = false"
      @ended="playing = false"
      @timeupdate="currentTime = mediaRef?.currentTime ?? 0"
      @loadedmetadata="duration = mediaRef?.duration || 0"
    />
    <!-- 已播放高亮：同一波形图作 mask（alpha 通道），品牌紫实底，clip-path 裁到进度；
         mask-size 与卡片 img 的 object-fit 同口径（grid contain / 其他 cover） -->
    <div
      v-if="waveUrl"
      class="played"
      :class="{ contain: ui.layoutMode === 'grid' }"
      :style="[waveMask, { clipPath: `inset(0 ${100 - pct}% 0 0)` }]"
    />
    <span v-if="duration > 0" class="head" :style="{ left: `${pct}%` }" />
    <!-- 右下胶囊：时间 + 播放/暂停；不触发 seek -->
    <div class="capsule" @pointerdown.stop @pointermove.stop @pointerup.stop>
      <span class="time">{{ formatDuration(currentTime) }}</span>
      <button class="hud-btn" v-tip="playing ? '暂停' : '播放'" @click.stop="togglePlay">
        <AppIcon :name="playing ? 'pause' : 'play'" :size="12" />
      </button>
    </div>
  </div>
</template>

<style scoped>
.hover-audio {
  position: absolute;
  inset: 0;
  cursor: pointer;
}
.hidden-media {
  display: none;
}
.played {
  position: absolute;
  inset: 0;
  background: var(--brand-1);
  -webkit-mask-image: var(--wave-url);
  mask-image: var(--wave-url);
  -webkit-mask-size: cover;
  mask-size: cover;
  -webkit-mask-position: center;
  mask-position: center;
  -webkit-mask-repeat: no-repeat;
  mask-repeat: no-repeat;
  pointer-events: none;
}
.played.contain {
  -webkit-mask-size: contain;
  mask-size: contain;
}
/* 播放头：1px 品牌紫竖线（同 5.2 波形进度条口径） */
.head {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 1px;
  background: var(--brand-1);
  pointer-events: none;
}
/* 右下胶囊（时间 + 播放/暂停钮）：共享样式在 base.css（.capsule / .time / .hud-btn） */
</style>
