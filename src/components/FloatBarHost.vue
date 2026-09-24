<script setup lang="ts">
// 多功能悬浮栏宿主（2026-09-09 预览悬浮栏二轮，用户定）——稿 prose「搜索栏 / 搜索栏展开成筛选 /
// 播放条 / MarkDown 工具栏 = 同一容器，切换时底色不变、宽度平滑伸缩、功能按钮 Y 轴出场入场」的工程落点：
// 搜索态与预览播放态共用一个 FloatBar 实例，预览进出 = mode 切换 morph（不再两个组件各自挂载）。
// 定位 = 底部居中距底 12、**两侧各 12 呼吸间距**（同日用户定）；pointer-events 穿透空隙。
// 播放态形态 = barSpecs 注册表驱动（spec = show 组 / 时间码 / 进度 / 控制面；新格式注册即接入，
// 宿主零改动）；查不到 spec 的预览 kind（image/font/text/other）= FloatBar visible=false 宽度+透明度退场。
// 播放条 fill = 横向填满（anchor 减两侧 12），进度条吃余量伸展；帧条浮于播放条上方同宽、间距 12。
// 弹层（倍率 / 音量 / 时间显示 / 笔记）= 对齐触发件上方居中（旧通栏口径回接，悬停开合 + 120ms 桥；
// 水平钳制在 anchor 内）。笔记弹层 = 确认制（confirm 保存不关 / cancel 关闭；切帧直接回填草稿）。
// 搜索侧（panelId/viewKey 具备才接 store——独立预览窗口不传，search 分支全禁）：
// draft ↔ search store 实时分词；Ctrl+F 聚焦；Esc 清空失焦 + requestHide（「始终显示搜索框」关时收回）。
import { computed, inject, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import FloatBar from "./bars/FloatBar.vue";
import FrameStrip from "./bars/FrameStrip.vue";
import WaveProgress from "./bars/WaveProgress.vue";
import RatePop from "./bars/RatePop.vue";
import VolumePop from "./bars/VolumePop.vue";
import NotesPop from "./bars/NotesPop.vue";
import { playBarSpecFor } from "../lib/barSpecs";
import { formatDuration } from "../lib/formatDuration";
import { frameOfTime, totalFrames } from "../lib/formatFrame";
import { useUiStore } from "../stores/ui";
import { useSearchStore } from "../stores/search";
import { previewMediaSession } from "../stores/previewMedia";
import { previewNotesSession } from "../stores/previewNotes";
import { PREVIEW_SESSION_KEY } from "../lib/previewSessionKey";

const props = withDefaults(
  defineProps<{
    panelId?: string; // 搜索侧 store 键（缺省 = 搜索分支禁用，独立预览窗口）
    viewKey?: string;
    previewActive?: boolean; // 预览态（ContentArea：preview.assetKey 且本 panel 激活；预览窗口恒 true）
  }>(),
  { panelId: undefined, viewKey: undefined, previewActive: false },
);
// 「始终显示搜索框」关时的收回请求（Esc 清空失焦后通知 ContentArea 隐藏；开关恒开时无害）
const emit = defineEmits<{ requestHide: [] }>();

const ui = useUiStore();
const search = useSearchStore();
// 预览族会话（ContentArea / PreviewWindow provide 注入；media / notes 随同属主 panel 注册表取）
const preview = inject(PREVIEW_SESSION_KEY)!;
const media = previewMediaSession(preview.panelId);
const notes = previewNotesSession(preview.panelId);

// ─── 模式与形态分发 ───
// spec 按 kind 查表（barSpecs 注册表，工厂收属主 panel 会话 ctx）；预览态且有 spec = play 模式，否则 search；
// md 文本语境（kind text 且 ext md）= markdown 工具栏恒显（2026-09-25 二轮走查改版：
// 预览态 = 仅视图组（铅笔 / 分屏——进编辑唯一入口，预览不再点击文字进编辑）；编辑态 = 全格式组）
const spec = computed(() => (props.previewActive ? playBarSpecFor(preview.kind, { preview, media }) : null));
const mdActive = computed(
  () =>
    props.previewActive &&
    preview.kind === "text" &&
    (preview.current?.ext ?? "").toLowerCase() === "md",
);
const mode = computed<"search" | "markdown" | "play">(() =>
  mdActive.value ? "markdown" : spec.value ? "play" : "search",
);
// 预览态查不到 spec（image/font/text/other）= 宽度+透明度退场（稿 prose）；md 语境恒显示工具栏；其余恒显示
const barVisible = computed(() => mdActive.value || !(props.previewActive && !spec.value));

// ─── 搜索侧（panelId/viewKey 具备才接 store）──
const hasSearch = computed(() => !!props.panelId && !!props.viewKey);
const draft = ref(props.panelId ? search.draftFor(props.panelId) : "");
const activeFilters = ref<string[]>([]); // 筛选按钮组点亮不接业务（本地切换；筛选弹窗另轮）
const floatBarRef = ref<InstanceType<typeof FloatBar>>();

watch(draft, (v) => {
  if (hasSearch.value) search.setDraft(props.panelId!, props.viewKey!, v);
});
// store → 本地回填（全局搜索预填 `~` 等外部写入通道；等值不回流防环）
watch(
  () => (props.panelId ? search.draftFor(props.panelId) : ""),
  (v) => {
    if (v !== draft.value) draft.value = v;
  },
);
// 视图 / 递归开关变化：搜索保持并按新范围重搜
watch([() => props.viewKey, () => ui.showSubfolders], () => {
  if (hasSearch.value) search.rerun(props.panelId!, props.viewKey!);
});
// Ctrl+F 聚焦请求（requestFocus 记 panelId，仅本栏响应）
watch(
  () => search.focusSeq,
  () => {
    if (props.panelId && search.focusPanelId === props.panelId) floatBarRef.value?.focusSearch();
  },
);
function toggleFilter(key: string): void {
  const i = activeFilters.value.indexOf(key);
  if (i >= 0) activeFilters.value.splice(i, 1);
  else activeFilters.value.push(key);
}
// Esc = 清空并失焦；「始终显示搜索框」关时顺带收回悬浮栏
function onEscape(): void {
  draft.value = "";
  floatBarRef.value?.blurSearch();
  emit("requestHide");
}
onMounted(() => {
  // 热重载 / 布局恢复：残留内容命中可能已失效，补搜一次恢复结果态
  if (hasSearch.value) search.rerun(props.panelId!, props.viewKey!);
});
onBeforeUnmount(() => {
  if (props.panelId) search.removePanel(props.panelId);
});

// ─── 播放侧：spec → FloatBarPlay props ───
const playAttrs = computed(() => {
  const s = spec.value;
  if (!s || mode.value !== "play") return {};
  return {
    playing: s.playing(),
    muted: s.muted?.() ?? false,
    loopOn: s.loopOn?.() ?? false,
    fullscreen: s.fullscreen?.() ?? false,
    disabled: s.disabled(),
    rateText: `${s.rate()}x`,
    curr: s.tcCurr(),
    total: s.tcTotal(),
    show: s.show,
    progress: progressModel.value,
  };
});
const progressModel = computed({
  get: () => (mode.value === "play" ? (spec.value?.progressGet() ?? 0) : 0),
  set: (p: number) => {
    if (mode.value === "play") spec.value?.progressSet(p);
  },
});

// ─── 帧条（spec.frames 提供 = 显示；5 格分页页随当前帧；空槽补 null 保 5 格）───
const STRIP_PAGE = 5;
const stripShow = computed(() => {
  const f = spec.value?.frames;
  return mode.value === "play" && !!f && f.count() > 0;
});
const stripPage = computed(() => {
  const f = spec.value?.frames;
  return f ? Math.floor(f.frame() / STRIP_PAGE) : 0;
});
// 当前帧页内索引（当前帧选中框 = 亮色/一级边框，2026-09-09 四轮用户定稿）
const stripCurrent = computed(() => {
  const f = spec.value?.frames;
  return f ? f.frame() - stripPage.value * STRIP_PAGE : -1;
});
const stripFrames = computed<(string | null)[]>(() => {
  const f = spec.value?.frames;
  const out: (string | null)[] = [];
  if (!f) return out;
  for (let i = 0; i < STRIP_PAGE; i++) {
    const idx = stripPage.value * STRIP_PAGE + i;
    out.push(idx < f.count() ? f.thumb(idx) : null); // 尾页越界 = 空槽
  }
  return out;
});
function stripSelect(i: number): void {
  const f = spec.value?.frames;
  if (!f) return;
  const idx = stripPage.value * STRIP_PAGE + i;
  if (idx < f.count()) f.setFrame(idx);
}

// ─── 悬停弹层（倍率 / 音量 / 时间显示）：悬停开 + 120ms 延迟关桥；对齐触发件上方居中 ───
type PopKind = "" | "rate" | "volume" | "tc";
const pop = ref<PopKind>("");
const popLeft = ref(0); // anchor 内 x（hover 弹层中心，钳制在 anchor 内）
const noteLeft = ref(0); // 笔记弹层独立 x——hover 弹层换位不带着笔记跑（2026-09-09 三轮实机报）
const anchorRef = ref<HTMLElement>();
let closeTimer: number | undefined;

// 弹层选择器按当前 pop 类取（rate/tc 同为 .rp-root 需区分）；:not(leave-active) 排除离场残留——
// 150ms 离场动画内上一弹层仍在 DOM，不区分会拿错宽度做钳制（探针实证：rate 弹层宽把 volume 钳偏）
const POP_SEL: Record<string, string> = {
  rate: ".rp-root:not(.fbh-tc-pop)",
  volume: ".vp-root",
  tc: ".fbh-tc-pop",
  note: ".np-root",
};
async function placePop(ev?: Event, kind: string = "rate"): Promise<void> {
  const anchor = anchorRef.value;
  if (!anchor) return;
  const tRect = (ev?.currentTarget as HTMLElement | null | undefined)?.getBoundingClientRect();
  await nextTick(); // 弹层挂载后读宽
  const popEl = anchor.querySelector<HTMLElement>(`${POP_SEL[kind]}:not(.fbh-pop-leave-active)`);
  const w = popEl?.offsetWidth ?? 0;
  const aW = anchor.offsetWidth;
  const ideal = tRect
    ? tRect.left + tRect.width / 2 - anchor.getBoundingClientRect().left
    : aW / 2;
  const clamped = w > 0 && aW > w ? Math.min(Math.max(ideal, w / 2), aW - w / 2) : ideal;
  if (kind === "note") noteLeft.value = clamped;
  else popLeft.value = clamped;
}
const popStyle = computed(() => ({ left: `${popLeft.value}px` }));
const noteStyle = computed(() => ({ left: `${noteLeft.value}px` }));
function hoverPop(kind: PopKind, on: boolean, ev?: PointerEvent): void {
  window.clearTimeout(closeTimer);
  if (on) {
    pop.value = kind;
    void placePop(ev, kind);
  } else {
    closeTimer = window.setTimeout(() => (pop.value = ""), 120);
  }
}
function keepPop(): void {
  window.clearTimeout(closeTimer);
}
function delayClose(): void {
  closeTimer = window.setTimeout(() => (pop.value = ""), 120);
}
// 切出播放态（退出预览 / 换无播放条 kind）：弹层全收
watch(mode, (m) => {
  if (m !== "play") pop.value = "";
});
// 倍率：spec 双向；选中即关（旧弹窗口径）
const rateModel = computed({
  get: () => `${spec.value?.rate() ?? 1}x`,
  set: (v: string) => {
    const r = Number.parseFloat(v);
    if (Number.isFinite(r)) spec.value?.setRate(r);
    pop.value = "";
  },
});
// 音量：显示映射在 spec（静音显 0）；点击钮 = 切静音（旧主行为）
const volumeModel = computed({
  get: () => spec.value?.volumeGet?.() ?? 0,
  set: (v: number) => spec.value?.volumeSet?.(v),
});
// 时间显示切换（复用 RatePop 项样式；「帧数/时间码」↔ ui.timeDisplay，与查看方式菜单同值联动）
const TC_MODES = ["帧数", "时间码"];
const tcModel = computed({
  get: () => (ui.timeDisplay === "frames" ? "帧数" : "时间码"),
  set: (v: string) => {
    ui.timeDisplay = v === "帧数" ? "frames" : "timecode";
    pop.value = ""; // 选中即关
  },
});

// ─── 笔记弹层（确认制；帧号列格式化 = 旧 itemLabel 口径）───
const noteDraft = ref("");
const videoFrames = computed(() => ui.timeDisplay === "frames" && media.fps != null && media.fps > 0);
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
function onOpenNote(ev?: MouseEvent): void {
  const opening = !notes.open;
  notes.toggle(); // 打开即双暂停（store 口径）
  if (opening) void placePop(ev, "note"); // 对齐笔记钮
}
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
  <!-- 定位 = 底部居中距底 12、两侧各 12 呼吸（2026-09-09 用户定）；浮于内容上方、穿透空隙；
       全屏 = fixed 锚窗口（preview-root z-1500 铺满会盖住 content-area 内的 anchor，提 z 1600 压其上） -->
  <div ref="anchorRef" class="fbh-anchor" :class="{ fullscreen: preview.fullscreen }">
    <!-- 帧条：播放态且 spec 提供帧条时浮于播放条上方（同宽 = 同随 anchor 满宽，间距 12 归列 gap） -->
    <Transition name="fbh-strip">
      <FrameStrip
        v-if="stripShow"
        :frames="stripFrames"
        :current="stripCurrent"
        @select="stripSelect"
        @page-prev="spec?.frames?.stepFrame(-1)"
        @page-next="spec?.frames?.stepFrame(1)"
      />
    </Transition>
    <FloatBar
      ref="floatBarRef"
      :mode="mode"
      :fill="mode === 'play'"
      :visible="barVisible"
      v-bind="playAttrs"
      v-model:query="draft"
      :active="activeFilters"
      @update:progress="progressModel = $event"
      @filter-toggle="toggleFilter"
      @escape="onEscape"
      @toggle-play="spec?.togglePlay()"
      @seek-back="spec?.seekBack?.()"
      @seek-forward="spec?.seekForward?.()"
      @step-prev="spec?.stepPrev?.()"
      @step-next="spec?.stepNext?.()"
      @volume-click="spec?.toggleMute?.()"
      @volume-hover="(on: boolean, ev?: PointerEvent) => hoverPop('volume', on, ev)"
      @open-rate="(ev?: MouseEvent) => { pop = pop === 'rate' ? '' : 'rate'; if (pop === 'rate') void placePop(ev, 'rate'); }"
      @rate-hover="(on: boolean, ev?: PointerEvent) => hoverPop('rate', on, ev)"
      @tc-hover="(on: boolean, ev?: PointerEvent) => hoverPop('tc', on, ev)"
      @toggle-loop="spec?.toggleLoop?.()"
      @open-note="onOpenNote"
      @set-cover="spec?.setCover?.()"
      @open-fullscreen="spec?.toggleFullscreen?.()"
      @progress-drag-start="spec?.dragStart?.()"
      @progress-drag-end="spec?.dragEnd?.()"
      @action="preview.requestMdAction($event)"
      :editing="mdActive ? preview.mdEditing : undefined"
    >
      <!-- 音频语境：进度区换竖条波形（spec.wave = 音频族标志；音乐预览稿 prose「只在播放条上放波形图」） -->
      <template v-if="spec?.wave" #progress>
        <WaveProgress
          v-model="progressModel"
          @drag-start="spec?.dragStart?.()"
          @drag-end="spec?.dragEnd?.()"
        />
      </template>
    </FloatBar>

    <!-- 弹层：触发件上方居中（bottom = 栏高 42 + 间距 12），水平钳制 anchor 内；悬停开合 + 120ms 桥 -->
    <Transition name="fbh-pop">
      <RatePop
        v-if="mode === 'play' && pop === 'rate'"
        class="fbh-pop"
        :style="popStyle"
        v-model="rateModel"
        @pointerenter="keepPop"
        @pointerleave="delayClose"
      />
    </Transition>
    <Transition name="fbh-pop">
      <VolumePop
        v-if="mode === 'play' && pop === 'volume'"
        class="fbh-pop"
        :style="popStyle"
        v-model="volumeModel"
        @pointerenter="keepPop"
        @pointerleave="delayClose"
      />
    </Transition>
    <Transition name="fbh-pop">
      <RatePop
        v-if="mode === 'play' && pop === 'tc'"
        class="fbh-pop fbh-tc-pop"
        :style="popStyle"
        :rates="TC_MODES"
        v-model="tcModel"
        @pointerenter="keepPop"
        @pointerleave="delayClose"
      />
    </Transition>
    <Transition name="fbh-pop">
      <NotesPop
        v-if="mode === 'play' && notes.open"
        class="fbh-pop"
        :style="noteStyle"
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
/* 定位 = 底部居中距底 12、两侧各 12 呼吸（用户定）；浮于内容上方（z 1 压 stage），穿透空隙 */
.fbh-anchor {
  position: absolute;
  left: var(--n-space-12);
  right: var(--n-space-12);
  bottom: var(--n-space-12);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--n-space-12); /* 帧条与播放条间距 12（播放条-帧条稿 prose） */
  pointer-events: none;
  z-index: 1;
}
.fbh-anchor > * {
  pointer-events: auto;
}
/* 全屏：preview-root z-1500 fixed 铺满窗口会盖住 content-area 内的 anchor——锚改 fixed 窗口级、
   z 提 1600（通知条同档）压其上（2026-09-09 三轮实机报：全屏无播放条无法控制） */
.fbh-anchor.fullscreen {
  position: fixed;
  left: var(--n-space-12);
  right: var(--n-space-12);
  bottom: var(--n-space-12);
  z-index: 1600;
}
/* 帧条 / fill 播放条满 anchor 宽；搜索态壳自宽 + align center 居中（稿口径） */
.fbh-anchor > .fs-root {
  width: 100%;
}
/* 弹层：触发件上方居中（left 由 js 给触发件中心，钳制 anchor 内） */
.fbh-pop {
  position: absolute;
  bottom: calc(42px + var(--n-space-12));
  transform: translateX(-50%);
  max-width: 100%;
}
/* 弹层显隐 = 透明度 + 轻微缩放 150ms 缓入缓出（稿无弹层动效 prose；锚点在下，套旧 menu-pop 语义 + token 档） */
.fbh-pop-enter-active,
.fbh-pop-leave-active {
  transition:
    opacity var(--n-dur-150) var(--n-ease-in-out),
    transform var(--n-dur-150) var(--n-ease-in-out);
}
.fbh-pop-enter-from,
.fbh-pop-leave-to {
  opacity: 0;
  transform: translateX(-50%) scale(0.98);
}
/* 帧条出入场 = 透明度 + 下位移 4px 250ms（悬浮栏族档；mode 切换随播放条 morph 配套） */
.fbh-strip-enter-active,
.fbh-strip-leave-active {
  transition:
    opacity var(--n-dur-250) var(--n-ease-in-out),
    transform var(--n-dur-250) var(--n-ease-in-out);
}
.fbh-strip-enter-from,
.fbh-strip-leave-to {
  opacity: 0;
  transform: translateY(4px);
}
/* 帧条离场脱离流（absolute 锚原位的上方），不顶播放条位置 */
.fbh-strip-leave-active {
  position: absolute;
  left: 0;
  right: 0;
  bottom: calc(42px + var(--n-space-12));
}
</style>
