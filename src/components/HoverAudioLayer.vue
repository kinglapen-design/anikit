<script setup lang="ts">
// 音频卡片悬停播放层（5.3，2026-07-27 用户定案，参考图 2）：无悬停按钮，悬停即有声播放。
// 2026-09-22 R6 audio 变体接线收窄：卡片内容区已换 FileCard kind="audio" 竖线波形
//（WaveContent，peaks = get_audio_peaks 数据通道；进度 = 品牌色扫过 clip-path 与根数解耦），
// 本层收窄为「播放 + seek 交互层」——透明整层 pointerdown/move/up seek（点击/拖动波形区）、
// media 元素自管（悬停 300ms 进延迟、直读优先 + 转码兜底）；状态三通道 emit 上抛 AssetCard
// → FileCard：playing（扫过恒显 + 胶囊图标）/ progress 0..1（扫过裁切，**rAF 逐帧**——
// timeupdate 仅 4Hz 会一段段跳，2026-09-22 实机修订）/ duration（胶囊时长兜底）；
// 胶囊播放钮点击经 FileCard playToggle → defineExpose togglePlay 回转。
// 旧件退役：mask 波形图扫过 div（扫过归 WaveContent played 层）、自带 ha-capsule（胶囊归
// FileCard overlays MediaCapsule——消除双胶囊）、1px 播放头竖线（更早废除——扫过层缘即播放头）。
import { ref } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { getMediaSource } from "../lib/mediaOps";
import { useHoverPlayer } from "../lib/useHoverPlayer";
import type { PlacementView } from "../stores/content";

const props = defineProps<{ asset: PlacementView }>();
const emit = defineEmits<{
  playing: [value: boolean]; // 播放 ↔ 暂停/结束
  progress: [value: number]; // 0..1 播放进度（WaveContent 扫过裁切）
  duration: [value: number]; // 秒（media 元数据；胶囊时长 asset.duration 兜底）
}>();

const layerRef = ref<HTMLElement>();
const mediaRef = ref<HTMLVideoElement>(); // 音频复用 video 元素（同 previewMedia 手法）
const duration = ref(0);

let seeking = false;

// 进度汇报（2026-09-22 用户实机报「走带一段一段跳」）：timeupdate 仅约 4Hz——播放中改 rAF
// 逐帧读 currentTime（60fps 丝滑扫过，稿 prose「丝滑的扫过」）；暂停 / 结束 / 卸载即停循环
let rafId = 0;
function stopProgressLoop(): void {
  if (rafId) {
    cancelAnimationFrame(rafId);
    rafId = 0;
  }
}
function startProgressLoop(): void {
  stopProgressLoop();
  const tick = (): void => {
    const v = mediaRef.value;
    if (v && duration.value > 0) {
      emit("progress", Math.min(1, Math.max(0, v.currentTime / duration.value)));
    }
    rafId = requestAnimationFrame(tick);
  };
  rafId = requestAnimationFrame(tick);
}

// 悬停播放骨架（300ms 进入延迟 / 活跃实例注册 / disposed 护栏 / 卸载清理）收编 useHoverPlayer
const { isDisposed } = useHoverPlayer({
  load: loadSource,
  pause: () => mediaRef.value?.pause(),
  resume: () => void mediaRef.value?.play().catch(() => {}),
  cleanup: () => {
    stopProgressLoop();
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
    usedTranscode = r.transcoded;
    v.src = convertFileSrc(r.path);
    void v.play().catch((e) => console.warn("悬停播放失败：", e));
  } catch (e) {
    console.warn("悬停播放源解析失败：", e);
  }
}

// 直读优先的播放错误回退（2026-09-13，同 HoverVideoLayer）：wma 等直解不了 →
// error 后 forceTranscode 重调转码兜底（cache/media 持久产物，每资产只转一次）
let usedTranscode = false;
async function onSourceError(): Promise<void> {
  const v = mediaRef.value;
  if (!v || !v.currentSrc || usedTranscode || isDisposed()) return;
  usedTranscode = true;
  try {
    const r = await getMediaSource(props.asset.assetId, true);
    if (isDisposed()) return;
    v.src = convertFileSrc(r.path);
    void v.play().catch(() => {});
  } catch (e) {
    console.warn("悬停播放转码兜底失败：", e);
  }
}

function togglePlay(): void {
  const v = mediaRef.value;
  if (!v || !v.src) return;
  if (v.paused) void v.play().catch(() => {});
  else v.pause();
}
defineExpose({ togglePlay }); // 胶囊播放钮：FileCard playToggle → AssetCard 回转本层

function seek(e: PointerEvent): void {
  const v = mediaRef.value;
  if (!v || duration.value <= 0) return;
  const rect = layerRef.value!.getBoundingClientRect();
  const ratio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
  v.currentTime = ratio * duration.value;
  emit("progress", ratio); // 拖动直发（timeupdate 频率低不跟手）
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
      @play="emit('playing', true); startProgressLoop()"
      @pause="emit('playing', false); stopProgressLoop()"
      @ended="emit('playing', false); stopProgressLoop()"
      @loadedmetadata="duration = mediaRef?.duration || 0; emit('duration', duration)"
      @error="onSourceError"
    />
  </div>
</template>

<style scoped>
/* 透明 seek 交互层：整卡波形区可点/可拖；视觉（竖线波形 / 扫过 / 胶囊）全归 FileCard audio 变体 */
.hover-audio {
  position: absolute;
  inset: 0;
  cursor: pointer;
}
.hidden-media {
  display: none;
}
</style>
