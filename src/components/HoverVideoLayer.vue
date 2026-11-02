<script setup lang="ts">
// 视频卡片播放层（5.3，2026-07-27 用户定案，参考图 1），双模式：
// - 悬停模式（auto=false）：完整播放整段视频（循环），跟随鼠标横向移动快进快退
//   （video seek：pointermove 记目标——seeking 空闲直发、在飞合流于 seeked 补发（2026-09-08
//   实机报：长片源连发 seek，解码器取消-重启饿死，画面卡住甚至无反应；2026-08-22"浏览器
//   自身合并跟不上的 seek、无需门控"结论对长片源不成立，作废）；UI 状态同步写目标值跟手，
//   scrub 期间 timeupdate 不用实际位置覆盖（4Hz 覆盖 = 进度条回弹 / 闪烁，2026-09-08 二轮）；
//   预生成帧图方案已于 2026-07-28 用户决策废弃——CPU 长期高负载 / 磁盘占用大 / 载入慢）；
//   右上胶囊（7.1 内容容器轮换代，稿 = 组件/卡片/文件卡片-视频 悬停胶囊）= 当前帧号 + 播放/暂停 +
//   音量/静音（card-hover-* 图标 14，常态 二级 hover 一级），底部 4px 播放条（轨道 亮色/半透明-2、
//   填充 亮色/一级，贴底 4、左右 12、圆头 99）；旧右下胶囊 + 3px 品牌紫进度条口径作废；
// - 自动播放模式（auto=true，查看方式菜单"自动播放视频"开启且未悬停）：无控件、恒静音、不 scrub，
//   范围 = 封面帧（thumbFrame `t:{秒}`，未设置 = 0）起 10 秒封顶时长循环——
//   "设置当前画面为预览图"后自动播放范围自动跟随该帧（无需新设置 UI）。
// 悬停接管 / 移出在同一组件内切换（media 元素不重建）；media 元素组件自管（与 previewMedia 隔离）。
import { computed, ref, watch } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import AppIcon from "./AppIcon.vue";
import { formatDuration } from "../lib/formatDuration";
import { frameOfTime, totalFrames } from "../lib/formatFrame";
import { getMediaSource, getMediaSourceByPath } from "../lib/mediaOps";
import { AUTOPLAY_ENTER_DELAY, HOVER_ENTER_DELAY, useHoverPlayer } from "../lib/useHoverPlayer";
import { useUiStore } from "../stores/ui";
import type { PlacementView } from "../stores/content";
import "./cards/cards.css";

const props = withDefaults(defineProps<{ asset: PlacementView; auto?: boolean; startAt?: number | null }>(), { auto: false, startAt: null });
const ui = useUiStore();

// 自动播放预览范围上限（秒，2026-07-27 用户定：自动播放最大只支持 10 秒）
const RANGE_SECS = 10;

const layerRef = ref<HTMLElement>();
const videoRef = ref<HTMLVideoElement>();
const ready = ref(false); // canplay：视频淡入（之前保持底层静态缩略图，转码等待无闪烁）
const currentTime = ref(0);
const duration = ref(0);
const muted = ref(true); // 默认静音；自动播放模式恒静音（多视频同播），悬停可点音量钮开
const playing = ref(false); // 播放/暂停胶囊钮态（7.1 内容容器轮新增——稿悬停胶囊含播放/暂停）

// 起点钳制：源已变短于目标起点时回退 0
function clampStart(s: number): number {
  const d = duration.value || props.asset.duration || 0;
  return d > 0 && s >= d ? 0 : s;
}
// 语义命中起点（2026-11-02 用户实机定：「开了自动播放就从命中的往后放」）——
// startAt 仅语义搜索命中态由 AssetCard 传入（定位窄段 startMs）；null = 无命中走原口径
const hitStart = computed(() => (props.startAt != null && props.startAt > 0 ? clampStart(props.startAt) : null));
// 范围起点：自动播放 = 命中点优先（无命中回退封面帧秒数）；悬停 = 0（完整播放）
function rangeStart(): number {
  if (!props.auto) return 0;
  if (hitStart.value != null) return hitStart.value;
  const tf = props.asset.thumbFrame;
  let s = tf?.startsWith("t:") ? Number(tf.slice(2)) || 0 : 0;
  return clampStart(s);
}
// 初始落点：悬停模式 = 命中点优先（范围仍全片——「没开自动播放就停在命中的那一帧」，
// scrub / 循环不受影响）；自动播放 = 范围起点
function initialSeek(): number {
  if (!props.auto && hitStart.value != null) return hitStart.value;
  return rangeStart();
}
// 范围终点：自动播放 = 起点 + 10 秒封顶时长；悬停 = 片尾
function rangeEnd(): number {
  const d = duration.value || props.asset.duration || 0;
  if (!props.auto) return d > 0 ? d : Number.MAX_SAFE_INTEGER;
  return d > 0 ? Math.min(rangeStart() + RANGE_SECS, d) : rangeStart() + RANGE_SECS;
}
const pct = computed(() => {
  const span = rangeEnd() - rangeStart();
  return span > 0 ? Math.min(100, Math.max(0, ((currentTime.value - rangeStart()) / span) * 100)) : 0;
});
// 胶囊位置标识（5.3 帧化改造，2026-07-28 用户定：帧数优先）：查看方式"时间显示 = 帧数"且
// 素材 fps 可用 → 当前帧号；fps 缺失或时间码模式回退当前时间（mm:ss）
// fps 来源：库 = 素材元数据；ws 无元数据 = 播放源响应实时探测值（2026-10-23 悬停放行接入）
const fpsLive = ref<number | null>(null);
const posText = computed(() => {
  const fps = props.asset.fps ?? fpsLive.value;
  if (ui.timeDisplay === "frames" && fps != null && fps > 0) {
    const total = totalFrames(duration.value || props.asset.duration || 0, fps);
    return String(frameOfTime(currentTime.value, fps, total));
  }
  return formatDuration(currentTime.value);
});

// 悬停播放骨架（进入延迟 / 活跃实例注册 / disposed 护栏 / 卸载清理）收编 useHoverPlayer；
// 本组件只管 media 元素、scrub 与范围循环。
// 进入延迟按触发路径分档（2026-08-23）：自动播放（视口可见性驱动）100ms 提速滚回恢复；
// 悬停（鼠标意图）300ms 防划过。挂载瞬间 props.auto 已定型（悬停接管只切 prop 不重建组件）。
const { isDisposed } = useHoverPlayer({
  load: loadSource,
  pause: () => videoRef.value?.pause(),
  resume: () => {
    const v = videoRef.value;
    if (v && ready.value) void v.play().catch(() => {});
  },
  cleanup: () => {
    const v = videoRef.value;
    if (v) {
      v.pause();
      v.removeAttribute("src");
      v.load();
    }
  },
  enterDelay: props.auto ? AUTOPLAY_ENTER_DELAY : HOVER_ENTER_DELAY,
});

async function loadSource(): Promise<void> {
  try {
    // ws 合成项走按路径版（2026-10-23 悬停放行）；null = 非音视频 / 越界，保持底层静态缩略图
    const r = props.asset.wsPath
      ? await getMediaSourceByPath(props.asset.wsPath)
      : await getMediaSource(props.asset.assetId);
    if (!r || isDisposed()) return;
    const v = videoRef.value;
    if (!v) return;
    usedTranscode = r.transcoded;
    directRetried = false;
    if (r.fps != null) fpsLive.value = r.fps; // ws 无 fps 元数据：响应实时探测值补位（胶囊帧号照常）
    v.src = convertFileSrc(r.path);
    v.currentTime = initialSeek();
    void v.play().catch((e) => console.warn("卡片播放失败：", e));
  } catch (e) {
    console.warn("卡片播放源解析失败：", e);
  }
}

// 直读优先的播放错误回退（2026-09-13）：源编码 webview 直解不了（无 OS 解码器的 HEVC、
// mov/wma 容器等）→ error 后带 forceTranscode 重调，后端转码兜底；转码期 ready 恒 false，
// 卡片保持底层静态缩略图（无闪烁）
let usedTranscode = false;
// 瞬时错误直读重试一次（2026-09-24，同 previewMedia 口径）：seek / 硬解管线偶发报错
// 不等于格式不可播——非 SRC_NOT_SUPPORTED 先同源重载续播，仍 error 才落转码
let directRetried = false;
async function onSourceError(): Promise<void> {
  const v = videoRef.value;
  if (!v || !v.currentSrc || usedTranscode || isDisposed()) return;
  if (v.error?.code !== MediaError.MEDIA_ERR_SRC_NOT_SUPPORTED && !directRetried) {
    directRetried = true;
    const resume = currentTime.value;
    const onMeta = (): void => {
      v.removeEventListener("loadedmetadata", onMeta);
      if (isDisposed()) return;
      if (resume > 0 && resume < (v.duration || 0)) v.currentTime = resume;
      void v.play().catch(() => {});
    };
    v.addEventListener("loadedmetadata", onMeta);
    v.load();
    return;
  }
  usedTranscode = true;
  try {
    const r = props.asset.wsPath
      ? await getMediaSourceByPath(props.asset.wsPath, true)
      : await getMediaSource(props.asset.assetId, true);
    if (!r || isDisposed()) return;
    v.src = convertFileSrc(r.path);
    v.currentTime = initialSeek();
    void v.play().catch(() => {});
  } catch (e) {
    console.warn("卡片播放转码兜底失败：", e);
  }
}

// 模式切换（悬停接管 / 移出回自动播放）：元素不重建——切回自动播放强制静音，
// 位置超出范围由 timeupdate 自然跳回起点
watch(
  () => props.auto,
  (auto) => {
    const v = videoRef.value;
    if (auto && v) {
      v.muted = true;
      muted.value = true;
    }
  },
);

function onTimeUpdate(): void {
  const v = videoRef.value;
  if (!v) return;
  // 范围循环：到终点跳回起点（悬停模式 = 片尾跳回开头整段循环；不足 10 秒的视频自然整段循环）
  if (v.currentTime >= rangeEnd()) v.currentTime = rangeStart();
  // scrub 期间 UI 由 onScrub 直写跟手——seek 落点滞后于鼠标目标，timeupdate（4Hz）用实际
  // 位置覆盖 = 进度条回弹 / 闪烁（2026-09-08 实机复验报）；seek 落定后恢复同步
  if (v.seeking || scrubTarget !== null) return;
  currentTime.value = v.currentTime;
}
function onLoadedMetadata(): void {
  duration.value = videoRef.value?.duration || 0;
}
function onEnded(): void {
  const v = videoRef.value;
  if (!v) return;
  v.currentTime = rangeStart();
  void v.play().catch(() => {});
}
// 跟随鼠标快进快退（仅悬停模式）：横向位置映射到 [start, end]。
// UI 状态（进度条 / 帧号）同步写目标值立即跟手；video seek 串行化合流——seeking 在飞只记
// 最新目标（scrubTarget），seeked 补发（2026-09-08 实机报：长片源连发 seek 取消-重启饿死，
// 画面卡住甚至无反应）；元素未 ready 时不 seek，保持底层缩略图
let scrubTarget: number | null = null;

function onScrub(e: PointerEvent): void {
  if (props.auto) return;
  const v = videoRef.value;
  if (!v || !ready.value) return;
  const rect = layerRef.value!.getBoundingClientRect();
  const ratio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
  const t = rangeStart() + ratio * (rangeEnd() - rangeStart());
  currentTime.value = t;
  if (v.seeking) {
    scrubTarget = t;
    return;
  }
  scrubTarget = null; // 直发时清账——防陈旧目标在后续 seeked 误补发回跳
  v.currentTime = t;
}
function onSeeked(): void {
  const v = videoRef.value;
  if (!v || scrubTarget === null) return;
  const t = scrubTarget;
  scrubTarget = null;
  if (Math.abs(t - v.currentTime) > 0.04) v.currentTime = t; // 落点已在一帧内不重复 seek
}
function toggleMute(): void {
  const v = videoRef.value;
  if (!v) return;
  v.muted = !v.muted;
  muted.value = v.muted;
}
// 播放/暂停（7.1 新增，稿悬停胶囊）：暂停后进度条/帧号停当帧，再点续播
function togglePlay(): void {
  const v = videoRef.value;
  if (!v || !v.src) return;
  if (v.paused) void v.play().catch(() => {});
  else v.pause();
}
</script>

<template>
  <div ref="layerRef" class="hover-video" @pointermove="onScrub">
    <video
      ref="videoRef"
      class="v"
      :class="{ ready, contain: ui.layoutMode === 'grid' }"
      :muted="muted"
      preload="auto"
      playsinline
      @canplay="ready = true"
      @play="playing = true"
      @pause="playing = false"
      @seeked="onSeeked"
      @timeupdate="onTimeUpdate"
      @loadedmetadata="onLoadedMetadata"
      @ended="onEnded"
      @error="onSourceError"
    />
    <!-- 控件（仅悬停模式；7.1 换代贴稿）：右上胶囊 = 帧号 + 播放/暂停 + 音量/静音；
         底部 4px 播放条（贴底 4 左右 12，纯显示 pointer-events none 不挡名称）；
         层根不建层叠上下文，胶囊 / 播放条 z 4 / 5 全局层叠在渐变罩（z 1）之上；
         均 pointermove.stop 不触发 scrub -->
    <template v-if="!auto">
      <div class="kc-float hv-capsule" @pointermove.stop>
        <span class="hv-pos">{{ posText }}</span>
        <button class="hv-btn" v-tip="playing ? '暂停' : '播放'" @click.stop="togglePlay">
          <AppIcon :name="playing ? 'card-hover-pause' : 'card-hover-play'" :size="14" />
        </button>
        <button class="hv-btn" v-tip="muted ? '取消静音' : '静音'" @click.stop="toggleMute">
          <AppIcon :name="muted ? 'card-hover-mute' : 'card-hover-volume'" :size="14" />
        </button>
      </div>
      <div class="hv-bar"><div class="hv-fill" :style="{ width: `${pct}%` }" /></div>
    </template>
  </div>
</template>

<style scoped>
.hover-video {
  position: absolute;
  inset: 0;
}
.v {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: cover;
  opacity: 0;
  transition: opacity 0.15s ease;
}
.v.contain {
  object-fit: contain;
}
.v.ready {
  opacity: 1;
}
/* 控件区（7.1 换代贴稿）：胶囊 = 右上 12/12（kc-float 铬，cards.css）；播放条 = 底部 4px 横贯
   （贴底 4、左右 12、圆头 99 稿值字面量），轨道 亮色/半透明-2 + 填充 亮色/一级；
   旧 base.css .capsule 共享样式不再用（HoverAudioLayer 同步换代） */
.hv-capsule {
  position: absolute;
  right: var(--n-space-12);
  top: var(--n-space-12);
  z-index: 4;
  gap: var(--n-space-6);
}
/* 帧号：数字档 12/16 二级 */
.hv-pos {
  font-family: var(--n-font-mono);
  font-size: var(--n-fs-num);
  line-height: var(--n-lh-num);
  font-weight: var(--n-fw-medium);
  color: var(--n-l2);
}
/* 胶囊图标钮：常态 二级，自身 hover 提亮 一级（稿「hover-播放/音量按钮」节），150ms */
.hv-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  color: var(--n-l2);
  cursor: pointer;
  transition: color var(--n-dur-150) var(--n-ease-in-out);
}
.hv-btn:hover {
  color: var(--n-l1);
}
.hv-bar {
  position: absolute;
  left: var(--n-space-12);
  right: var(--n-space-12);
  bottom: var(--n-space-4);
  z-index: 5;
  height: 4px;
  border-radius: 99px; /* 稿值 99 */
  background: var(--n-l-a2);
  pointer-events: none;
}
.hv-fill {
  display: block;
  height: 100%;
  border-radius: 99px; /* 稿值 99 */
  background: var(--n-l1);
}
</style>
