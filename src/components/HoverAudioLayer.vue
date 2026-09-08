<script setup lang="ts">
// 音频卡片悬停播放层（5.3，2026-07-27 用户定案，参考图 2）：无悬停按钮，悬停即有声播放。
// 卡片缩略图即波形图（5.1），播放时叠加：已播放部分品牌紫（div 以同一波形图作 CSS mask +
// clip-path 裁到进度，照播放条波形方案——现 = bars/WaveProgress，2026-09-09 接入轮）；点击/拖动波形 seek；
// 右上胶囊（7.1 内容容器轮换代，稿 = 组件/卡片/文件卡片-音频 悬停胶囊）= 时长 + 播放/暂停
//（kc-float 铬 + card-hover-* 图标 14）；新稿扫过层无 1px 播放头竖线（旧口径废除——
// 扫过层缘即播放头）；peaks 竖线波形如稿待后端数据通道（09 待办，本轮沿用 mask 波形图方案）。
// media 元素组件自管。
import { computed, ref } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import AppIcon from "./AppIcon.vue";
import { formatDuration } from "../lib/formatDuration";
import { getMediaSource } from "../lib/mediaOps";
import { useHoverPlayer } from "../lib/useHoverPlayer";
import { useUiStore } from "../stores/ui";
import type { PlacementView } from "../stores/content";
import "./cards/cards.css";

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
  resume: () => void mediaRef.value?.play().catch(() => {}),
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
  // 不可 preventDefault（2026-08-18 修复"音频卡片缩略图点击不选中"）：
  // pointerdown preventDefault 会抑制兼容鼠标事件，click 不派发 → 卡片永远选不中
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
    <!-- 右上胶囊（7.1 换代贴稿）：时长 + 播放/暂停；不触发 seek；z 4 在渐变罩之上 -->
    <div class="kc-float ha-capsule" @pointerdown.stop @pointermove.stop @pointerup.stop>
      <span class="ha-pos">{{ formatDuration(asset.duration ?? duration) }}</span>
      <button class="ha-btn" v-tip="playing ? '暂停' : '播放'" @click.stop="togglePlay">
        <AppIcon :name="playing ? 'card-hover-pause' : 'card-hover-play'" :size="14" />
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
/* 右上胶囊（7.1 换代贴稿 = 文件卡片-音频 悬停胶囊）：kc-float 铬；z 4 在渐变罩（z 1）之上；
   旧右下胶囊（base.css .capsule）与 1px 播放头竖线口径废除 */
.ha-capsule {
  position: absolute;
  right: var(--n-space-12);
  top: var(--n-space-12);
  z-index: 4;
  gap: var(--n-space-6);
}
/* 时长：数字档 12/16 二级 */
.ha-pos {
  font-family: var(--n-font-mono);
  font-size: var(--n-fs-num);
  line-height: var(--n-lh-num);
  font-weight: var(--n-fw-medium);
  color: var(--n-l2);
}
/* 胶囊图标钮：常态 二级，自身 hover 提亮 一级（稿「hover-播放/音量按钮」节），150ms */
.ha-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  color: var(--n-l2);
  cursor: pointer;
  transition: color var(--n-dur-150) var(--n-ease-in-out);
}
.ha-btn:hover {
  color: var(--n-l1);
}
</style>
