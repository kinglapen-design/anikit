<script setup lang="ts">
// 预览界面多功能悬浮栏·播放条 业务壳（2026-09-09 预览悬浮栏接入轮）：FloatBar play 模式 + 帧条
// （FrameStrip，序列帧/动图）+ 倍率 / 音量 / 时间显示悬停弹层 + 笔记弹层（NotesPop）。
// 替代旧通栏三件套（PreviewPlayBar / PreviewFrameStrip / PreviewNotes——13_播放条稿直角通栏口径本轮作废）。
// 定位 = 底部居中距底 12（中间面板-多功能悬浮栏稿，同搜索悬浮栏 .sf-anchor 口径），pointer-events 穿透空隙；
// 帧条浮于播放条上方、两者同宽、间距 12（播放条-帧条稿 prose——宽度链归本壳：帧条 5 格自然宽 777，
// 播放条 :width 跟随同宽；容器窄时双双 max-width 100% 钳制，壳内裁剪）。
// show 组按上下文删减（稿 prose「根据上下文，选择保留哪些按钮」）：video 全量；audio 去 transport/step/
// note/cover/fullscreen（旧音频条删减口径）；sequence/anim 去 transport/step/volume/loop（上/下帧归帧条
// 两端钮、动图无音量、帧播放无循环钮——旧帧条删减口径）。
// 帧条分页：5 格一页（稿 5 格陈列），页 = 当前帧所在页（floor(frame/5) 自动跟随，播放/步进不脱节）；
// 两端尖括号钮 = 上/下一帧（stepFrame ∓1，沿用旧帧条语义）；空槽补 null 保 5 格宽度恒定。
// 弹层：倍率 / 音量 / 时间显示 = 悬停开 + 120ms 延迟关桥（LinkListPopover 同款口径），倍率钮点击也可开合；
// 选中即关（旧弹窗口径）；音量钮点击 = 切静音（旧主行为不变）；时间显示切换复用 RatePop（项样式定稿件，
// 「帧数/时间码」两档 ↔ ui.timeDisplay，与查看方式菜单同值联动）。
// 笔记弹层 = 确认制（稿 取消/确认 按钮）：confirm = 保存当前帧笔记（空文本 = 删除，store 语义）不关弹层，
// cancel = 关闭；切帧/切列表项草稿直接回填（未确认即丢弃，显式按钮语义）；打开即聚焦输入由 NotesPop 自带。
// 转码态 = FloatBarPlay disabled（全控件禁用 + 进度区「转码中…」，旧口径：结构保持不跳变）。
// 帧条当前帧高亮 / 笔记圆点：稿未画（09 待办）→ 不做，不自创。
import { computed, ref, watch } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import FloatBar from "./bars/FloatBar.vue";
import FrameStrip from "./bars/FrameStrip.vue";
import WaveProgress from "./bars/WaveProgress.vue";
import RatePop from "./bars/RatePop.vue";
import VolumePop from "./bars/VolumePop.vue";
import NotesPop from "./bars/NotesPop.vue";
import { formatDuration } from "../lib/formatDuration";
import { frameOfTime, totalFrames } from "../lib/formatFrame";
import { useUiStore } from "../stores/ui";
import { usePreviewStore } from "../stores/preview";
import { usePreviewMediaStore } from "../stores/previewMedia";
import { usePreviewNotesStore } from "../stores/previewNotes";

type PlayGroup = "transport" | "step" | "volume" | "rate" | "loop" | "note" | "cover" | "fullscreen";

const ui = useUiStore();
const preview = usePreviewStore();
const media = usePreviewMediaStore();
const notes = usePreviewNotesStore();

const isMedia = computed(() => preview.kind === "video" || preview.kind === "audio");
const isFrames = computed(() => preview.kind === "sequence" || preview.kind === "anim");

// show 组按上下文删减（稿 prose）
const showGroups = computed<PlayGroup[]>(() => {
  if (preview.kind === "video") return ["transport", "step", "volume", "rate", "loop", "note", "cover", "fullscreen"];
  if (preview.kind === "audio") return ["volume", "rate", "loop"];
  return ["rate", "note", "cover", "fullscreen"]; // sequence / anim
});

// —— 播放状态 / 进度 ——
const playing = computed(() => (isMedia.value ? media.playing : preview.playing));
// 总帧数（序列帧 = 清单长度；动图 = 帧流总帧数）
const frameCount = computed(() =>
  preview.kind === "anim" ? preview.animTotal : (preview.seq?.frames.length ?? 0),
);
const progressModel = computed({
  get: () => {
    if (isMedia.value) return media.duration > 0 ? media.currentTime / media.duration : 0;
    return frameCount.value > 1 ? preview.frame / (frameCount.value - 1) : 0;
  },
  set: (p: number) => {
    if (isMedia.value) {
      if (media.duration > 0) media.seek(p * media.duration);
    } else if (frameCount.value > 0) {
      preview.setFrame(Math.round(p * (frameCount.value - 1)));
    }
  },
});
// 动图帧流回退态（animTotal=0）不出播放条（旧帧播放条口径）——visible=false = 宽度+透明度退场（稿 prose）
const barVisible = computed(() => !isFrames.value || frameCount.value > 0);
// 转码态（仅音视频）
const transcoding = computed(() => isMedia.value && media.transcodingSlow);

// —— 时间码（video = 帧数模式优先，fps 缺失回退 mm:ss；audio 恒 mm:ss；sequence/anim 恒帧号）——
const videoFrames = computed(() => ui.timeDisplay === "frames" && media.fps != null && media.fps > 0);
const tcCurr = computed(() => {
  if (preview.kind === "video") {
    if (videoFrames.value && media.fps) {
      return String(frameOfTime(media.currentTime, media.fps, totalFrames(media.duration, media.fps)));
    }
    return formatDuration(media.currentTime);
  }
  if (preview.kind === "audio") return formatDuration(media.currentTime);
  return String(preview.frame + 1);
});
const tcTotal = computed(() => {
  if (preview.kind === "video") {
    if (videoFrames.value && media.fps) return String(totalFrames(media.duration, media.fps));
    return formatDuration(media.duration);
  }
  if (preview.kind === "audio") return formatDuration(media.duration);
  return String(frameCount.value);
});

// —— 倍率 ——
const curRate = computed(() => (isMedia.value ? media.rate : preview.rate));
const rateText = computed(() => `${curRate.value}x`);
const rateModel = computed({
  get: () => `${curRate.value}x`,
  set: (v: string) => {
    const r = Number.parseFloat(v);
    if (Number.isFinite(r)) {
      if (isMedia.value) media.setRate(r);
      else preview.setRate(r);
    }
    pop.value = ""; // 选中即关（旧倍率弹窗口径）
  },
});

// —— 音量（点击 = 切静音 旧主行为；悬停弹层滑块：静音时显 0、实际音量保留，setVolume>0 自动解静音）——
const volumeModel = computed({
  get: () => (media.muted ? 0 : media.volume),
  set: (v: number) => media.setVolume(v),
});

// —— 播放条事件 ——
function onTogglePlay(): void {
  if (isMedia.value) media.togglePlay();
  else preview.togglePlay();
}
// 逐帧步进（仅 video 有 step 组；步进即暂停，fps 缺失回退 1/24s——旧 PlayBarVideo 口径）
function stepOneFrame(delta: number): void {
  media.pause();
  media.seekBy(delta / (media.fps != null && media.fps > 0 ? media.fps : 24));
}
function onSetCover(): void {
  if (preview.kind === "video") media.setVideoThumb();
  else preview.setAsThumb();
}

// —— 悬停弹层（倍率 / 音量 / 时间显示）：悬停开 + 120ms 延迟关桥；弹层自身悬停取消关 ——
type PopKind = "" | "rate" | "volume" | "tc";
const pop = ref<PopKind>("");
let closeTimer: number | undefined;
function hoverPop(kind: PopKind, on: boolean): void {
  window.clearTimeout(closeTimer);
  if (on) pop.value = kind;
  else closeTimer = window.setTimeout(() => (pop.value = ""), 120);
}
function keepPop(): void {
  window.clearTimeout(closeTimer);
}
function delayClose(): void {
  closeTimer = window.setTimeout(() => (pop.value = ""), 120);
}
// 时间显示切换（复用 RatePop 项样式；「帧数/时间码」↔ ui.timeDisplay，与查看方式菜单同值联动）
const TC_MODES = ["帧数", "时间码"];
const tcModel = computed({
  get: () => (ui.timeDisplay === "frames" ? "帧数" : "时间码"),
  set: (v: string) => {
    ui.timeDisplay = v === "帧数" ? "frames" : "timecode";
    pop.value = ""; // 选中即关
  },
});

// —— 帧条（序列帧 / 动图）：5 格一页，页随当前帧；两端钮 = 上/下一帧（旧帧条步进语义）——
const STRIP_PAGE = 5;
const STRIP_W = 777; // 5 格自然宽稿值链（5×129 + gap 4×8 + 钮 2×34 + 钮 padding 2×8 + 壳 padding 16）
const stripShow = computed(() => !!preview.seq || (preview.kind === "anim" && preview.animTotal > 0));
const stripPage = computed(() => Math.floor(preview.frame / STRIP_PAGE));
const stripFrames = computed<(string | null)[]>(() => {
  const out: (string | null)[] = [];
  for (let i = 0; i < STRIP_PAGE; i++) {
    const idx = stripPage.value * STRIP_PAGE + i;
    if (idx >= frameCount.value) {
      out.push(null); // 尾页空槽：保 5 格宽度恒定（播放条同宽不跳）
      continue;
    }
    if (preview.kind === "anim") {
      const p = preview.animStrips[`f:${idx}`]; // 帧条小图渐进填充，未就绪 = 空槽
      out.push(p ? convertFileSrc(p) : null);
    } else {
      const f = preview.seq?.frames[idx];
      out.push(f?.stripPath ? convertFileSrc(f.stripPath) : null);
    }
  }
  return out;
});
function stripSelect(i: number): void {
  const idx = stripPage.value * STRIP_PAGE + i;
  if (idx < frameCount.value) preview.setFrame(idx);
}

// —— 音频波形（音乐预览稿 prose：波形只在播放条上）：高清重采样在 WaveProgress 内，本壳只给回退图 ——
const waveFallback = computed(() => {
  const p = preview.current?.thumbPath;
  return p ? convertFileSrc(p) : null;
});

// —— 笔记弹层（确认制；帧号列格式化 = 旧 itemLabel 口径：t: → 帧 N / mm:ss，f: → 帧 N，帧名 → 帧 N）——
const noteDraft = ref("");
function itemLabel(key: string): string {
  if (key.startsWith("t:")) {
    const secs = Number(key.slice(2));
    if (videoFrames.value && media.fps) {
      const total = totalFrames(media.duration, media.fps);
      return `帧 ${frameOfTime(secs, media.fps, total)}`;
    }
    return formatDuration(secs);
  }
  if (key.startsWith("f:")) {
    const i = Number(key.slice(2));
    return Number.isFinite(i) ? `帧 ${i + 1}` : key;
  }
  const i = preview.seq?.frames.findIndex((f) => f.name === key) ?? -1;
  return i >= 0 ? `帧 ${i + 1}` : key;
}
const noteItems = computed(() => notes.notes.map((n) => ({ frame: itemLabel(n.key), text: n.text })));
const noteSel = computed(() => notes.notes.findIndex((n) => n.key === notes.currentKey));
// 草稿回填：切帧 / 切列表项 / 笔记异步加载完成都直接回填（确认制 = 未确认即丢弃，无 dirty 防误删需求）
watch(
  () => [notes.currentKey, notes.currentText] as const,
  () => {
    noteDraft.value = notes.currentText;
  },
  { immediate: true },
);
function noteConfirm(): void {
  const k = notes.currentKey;
  if (k && noteDraft.value !== notes.currentText) void notes.save(k, noteDraft.value); // 空文本 = 删除（store 语义）
}
function noteSelect(i: number): void {
  const n = notes.notes[i];
  if (n) notes.jump(n.key);
}
function noteRemove(i: number): void {
  const n = notes.notes[i];
  if (n) void notes.remove(n.key);
}
</script>

<template>
  <!-- 定位 = 底部居中距底 12（稿），浮于预览画面上方、穿透空隙（仅栏体 / 弹层可交互） -->
  <div class="pfb-anchor">
    <div class="pfb-col">
      <FrameStrip
        v-if="stripShow"
        :frames="stripFrames"
        @select="stripSelect"
        @page-prev="preview.stepFrame(-1)"
        @page-next="preview.stepFrame(1)"
      />
      <FloatBar
        mode="play"
        :width="stripShow ? STRIP_W : undefined"
        :visible="barVisible"
        :playing="playing"
        :muted="media.muted"
        :loop-on="media.loop"
        :fullscreen="preview.fullscreen"
        :disabled="transcoding"
        :rate-text="rateText"
        :curr="tcCurr"
        :total="tcTotal"
        :show="showGroups"
        v-model:progress="progressModel"
        @toggle-play="onTogglePlay"
        @seek-back="media.seekBy(-5)"
        @seek-forward="media.seekBy(5)"
        @step-prev="stepOneFrame(-1)"
        @step-next="stepOneFrame(1)"
        @volume-click="media.toggleMute()"
        @volume-hover="hoverPop('volume', $event)"
        @open-rate="pop = pop === 'rate' ? '' : 'rate'"
        @rate-hover="hoverPop('rate', $event)"
        @tc-hover="hoverPop('tc', $event)"
        @toggle-loop="media.toggleLoop()"
        @open-note="notes.toggle()"
        @set-cover="onSetCover"
        @open-fullscreen="preview.setFullscreen(!preview.fullscreen)"
        @progress-drag-start="media.setScrubbing(true)"
        @progress-drag-end="media.setScrubbing(false)"
      >
        <!-- 音频语境：进度区换波形（音乐预览稿 prose「只在播放条上放波形图即可」） -->
        <template v-if="preview.kind === 'audio'" #progress>
          <WaveProgress
            v-model="progressModel"
            :fallback-url="waveFallback"
            @drag-start="media.setScrubbing(true)"
            @drag-end="media.setScrubbing(false)"
          />
        </template>
      </FloatBar>
    </div>

    <!-- 悬停弹层：播放条上方居中（bottom = 栏高 42 + 间距 12）；悬停开合 + 120ms 桥（同时只一档） -->
    <Transition name="pfb-pop">
      <RatePop
        v-if="pop === 'rate'"
        class="pfb-pop-pos"
        v-model="rateModel"
        @pointerenter="keepPop"
        @pointerleave="delayClose"
      />
    </Transition>
    <Transition name="pfb-pop">
      <VolumePop
        v-if="pop === 'volume'"
        class="pfb-pop-pos"
        v-model="volumeModel"
        @pointerenter="keepPop"
        @pointerleave="delayClose"
      />
    </Transition>
    <Transition name="pfb-pop">
      <RatePop
        v-if="pop === 'tc'"
        class="pfb-pop-pos pfb-tc-pop"
        :rates="TC_MODES"
        v-model="tcModel"
        @pointerenter="keepPop"
        @pointerleave="delayClose"
      />
    </Transition>

    <!-- 笔记弹层（533×378 稿值；同弹层位，盖帧条区） -->
    <Transition name="pfb-pop">
      <NotesPop
        v-if="notes.open"
        class="pfb-pop-pos"
        :notes="noteItems"
        :selected="noteSel"
        v-model:draft="noteDraft"
        @select="noteSelect"
        @remove="noteRemove"
        @confirm="noteConfirm"
        @cancel="notes.open = false"
      />
    </Transition>
  </div>
</template>

<style scoped>
/* 定位 = 底部居中距底 12（中间面板-多功能悬浮栏稿）；浮于画面上方（z 1 压 stage），穿透空隙 */
.pfb-anchor {
  position: absolute;
  left: 0;
  right: 0;
  bottom: var(--n-space-12);
  display: flex;
  justify-content: center;
  pointer-events: none;
  z-index: 1;
}
.pfb-anchor > * {
  pointer-events: auto;
}
/* 帧条 + 播放条竖列：间距 12（稿 prose），同宽——帧条宽 100% 随列，播放条 :width 同值跟随；
   列宽 = 帧条自然宽 777，容器窄时 max-width 钳制（壳内裁剪，同挤压口径） */
.pfb-col {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
  max-width: 100%;
}
/* 弹层定位：播放条上方居中（anchor 底 = 播放条底；bottom = 栏高 42 + 间距 12），水平钳制归弹层 max-width */
.pfb-pop-pos {
  position: absolute;
  left: 50%;
  bottom: calc(42px + var(--n-space-12));
  transform: translateX(-50%);
  max-width: 100%;
}
/* 弹层显隐 = 透明度 + 轻微缩放 150ms 缓入缓出（稿无弹层动效 prose；锚点在下，
   套旧 menu-pop 缩放语义 + 悬浮栏族 token 档） */
.pfb-pop-enter-active,
.pfb-pop-leave-active {
  transition:
    opacity var(--n-dur-150) var(--n-ease-in-out),
    transform var(--n-dur-150) var(--n-ease-in-out);
}
.pfb-pop-enter-from,
.pfb-pop-leave-to {
  opacity: 0;
  transform: translateX(-50%) scale(0.98);
}
</style>
