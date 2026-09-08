<script setup lang="ts">
// 播放条：稿为全量陈列（播放/暂停、快退/快进、上一帧/下一帧、声音/静音、时间码、进度条、
// 倍速、循环、笔记、设为封面、全屏），「根据上下文，选择保留哪些按钮」→ show prop 按组删减；
// 播放/暂停、声音/静音 = 单钮按态切换（playing / muted prop，稿两态图标并列是全量画法）。
// 图标 = 按钮族 IconButton bare 复用（18 实寸 亮色/三级，hover 升二级——稿 hover-按钮组口径）；
// 时间码 = 数字-大（当前 / 分隔 亮色/二级、总计 三级）；倍速 = 数字-大 三级，hover 升二级；
// 进度条 = BarSlider light（稿无填充段）。
// 快退/快进 = seek（原样 ◀◀ 快退 / flip-h ▶▶ 快进）；上/下帧 = step-frame（原样 |◀ 上一帧 / flip-h 下一帧）。
// 循环 / 笔记开启态稿未画 → 不改视觉（同按钮族禁用态口径，记 09 待办）。
// 业务接入轮（2026-09-09 预览悬浮栏接入）扩展：
// - disabled = 转码态：全控件禁用 + 进度区显「转码中…」（旧播放条口径：结构保持不跳变，提示放进度区）；
// - #progress 插槽 = 进度条替换口（音频波形进度，音乐预览稿 prose「只在播放条上放波形图」）；
// - .fbp-progress 改 width 327 + min-width 0（稿值，窄壳可缩）：宽度定值才有 max-content 贡献——
//   flex-basis 定值 + shrink>0 时 Blink 只报内容固有尺寸，FloatBar 探针实测会丢 327 稿值
//   （cdp-flex-contrib-exp.py 实测定案）；
// - title = v-tip（文案沿用旧播放条）；rate / volume / 时间码 带 pointerenter/leave emit（悬停弹层挂钩，
//   弹层定位与开合归业务壳 PreviewFloatBar）；进度拖动起止经 progress-drag-start/end 透传（scrubbing 挂钩）。
import IconButton from "../buttons/IconButton.vue";
import BarSlider from "./BarSlider.vue";
import "./bars.css";

type PlayGroup =
  | "transport" // 快退 / 快进
  | "step" // 上一帧 / 下一帧
  | "volume" // 声音 / 静音
  | "rate" // 倍速
  | "loop" // 循环播放
  | "note" // 笔记
  | "cover" // 设为封面
  | "fullscreen"; // 全屏

const props = withDefaults(
  defineProps<{
    playing?: boolean;
    muted?: boolean;
    loopOn?: boolean;
    fullscreen?: boolean; // 全屏态（title 文案两态：全屏 / 退出全屏）
    disabled?: boolean; // 转码态：全控件禁用 + 进度区「转码中…」
    rateText?: string;
    curr?: string; // 时间码当前（帧数 / 时长文本，父级格式化）
    total?: string; // 时间码总计
    show?: readonly PlayGroup[];
  }>(),
  {
    playing: false,
    muted: false,
    loopOn: false,
    fullscreen: false,
    disabled: false,
    rateText: "1x",
    curr: "00132",
    total: "00132",
    show: () => ["transport", "step", "volume", "rate", "loop", "note", "cover", "fullscreen"],
  },
);
const progress = defineModel<number>("progress", { default: 0 }); // 0~1
const emit = defineEmits<{
  "toggle-play": [];
  "seek-back": [];
  "seek-forward": [];
  "step-prev": [];
  "step-next": [];
  "volume-click": []; // 父级定行为（开音量弹层 / 切静音）
  "volume-hover": [on: boolean]; // 悬停开合音量弹层挂钩
  "open-rate": []; // 父级开倍率弹层（点击）
  "rate-hover": [on: boolean]; // 悬停开合倍率弹层挂钩
  "tc-hover": [on: boolean]; // 时间码悬停（时间显示切换弹层挂钩）
  "toggle-loop": [];
  "open-note": []; // 父级开笔记弹层
  "set-cover": [];
  "open-fullscreen": [];
  "progress-drag-start": []; // 进度拖动起止（scrubbing 挂钩，BarSlider 透传）
  "progress-drag-end": [];
}>();

function has(g: PlayGroup): boolean {
  return props.show.includes(g);
}
</script>

<template>
  <div class="fb-mode fbp-root">
    <IconButton
      :name="playing ? 'pause' : 'play'"
      variant="bare"
      :title="playing ? '暂停' : '播放'"
      :disabled="disabled"
      @click="emit('toggle-play')"
    />
    <template v-if="has('transport')">
      <IconButton name="seek" variant="bare" title="快退 5 秒" :disabled="disabled" @click="emit('seek-back')" />
      <IconButton name="seek" variant="bare" class="flip" title="快进 5 秒" :disabled="disabled" @click="emit('seek-forward')" />
    </template>
    <template v-if="has('step')">
      <IconButton name="step-frame" variant="bare" :title="{ text: '上一帧', keys: ',' }" :disabled="disabled" @click="emit('step-prev')" />
      <IconButton name="step-frame" variant="bare" class="flip" :title="{ text: '下一帧', keys: '.' }" :disabled="disabled" @click="emit('step-next')" />
    </template>
    <IconButton
      v-if="has('volume')"
      :name="muted ? 'mute' : 'volume'"
      variant="bare"
      :title="muted ? '取消静音' : '静音'"
      :disabled="disabled"
      @click="emit('volume-click')"
      @pointerenter="emit('volume-hover', true)"
      @pointerleave="emit('volume-hover', false)"
    />
    <div
      class="fbp-tc"
      v-tip="'时间显示'"
      @pointerenter="emit('tc-hover', true)"
      @pointerleave="emit('tc-hover', false)"
    >
      <span class="cur">{{ curr }}</span>
      <span class="sep">/</span>
      <span class="tot">{{ total }}</span>
    </div>
    <div class="fbp-progress">
      <!-- 转码态：进度区显提示（旧播放条口径：结构保持不跳变），控件全禁用 -->
      <span v-if="disabled" class="fbp-hint">转码中…</span>
      <slot v-else name="progress">
        <BarSlider
          v-model="progress"
          tone="light"
          @drag-start="emit('progress-drag-start')"
          @drag-end="emit('progress-drag-end')"
        />
      </slot>
    </div>
    <button
      v-if="has('rate')"
      class="fbp-rate"
      :disabled="disabled"
      @click="emit('open-rate')"
      @pointerenter="emit('rate-hover', true)"
      @pointerleave="emit('rate-hover', false)"
    >
      {{ rateText }}
    </button>
    <IconButton
      v-if="has('loop')"
      name="loop"
      variant="bare"
      title="循环播放"
      :class="{ on: loopOn }"
      :disabled="disabled"
      @click="emit('toggle-loop')"
    />
    <IconButton v-if="has('note')" name="note" variant="bare" title="笔记" :disabled="disabled" @click="emit('open-note')" />
    <IconButton
      v-if="has('cover')"
      name="set-cover"
      variant="bare"
      title="当前画面设为预览图"
      :disabled="disabled"
      @click="emit('set-cover')"
    />
    <IconButton
      v-if="has('fullscreen')"
      name="fullscreen"
      variant="bare"
      :title="fullscreen ? '退出全屏' : '全屏'"
      :disabled="disabled"
      @click="emit('open-fullscreen')"
    />
  </div>
</template>

<style scoped>
.fbp-root {
  flex: none;
  width: 100%; /* 恒满壳宽（壳 = 唯一宽度动画源，FloatBar 探针实测内容宽） */
}

/* flip-h 变体（快进 ▶▶ / 下一帧 ▶|，图标朝向口径见组件头注） */
.fbp-root .flip :deep(.app-icon) {
  transform: scaleX(-1);
}

/* 时间码：数字-大；当前 / 分隔 亮色/二级、总计 亮色/三级；内部 gap 4（稿值） */
.fbp-tc {
  display: flex;
  align-items: center;
  gap: var(--n-space-4);
  flex: none;
  font-family: var(--n-font-mono);
  font-size: var(--n-fs-num-lg);
  line-height: var(--n-lh-num-lg);
  font-weight: var(--n-fw-medium);
}
.fbp-tc .cur,
.fbp-tc .sep {
  color: var(--n-l2);
}
.fbp-tc .tot {
  color: var(--n-l3);
}

/* 进度条：width 327 稿值 + 默认 flex 0 1 auto + min-width 0——业务接入轮自适应（2026-09-09）：
   壳窄可缩（shrink 1）；probe（父 max-content）贡献 327 稿值——注意 flex-basis 定值 + shrink>0 时
   Blink 贡献只报内容固有尺寸（BarSlider 内容宽 0 → 整条被探针丢宽），width 定值才有贡献
   （cdp-flex-contrib-exp.py 实测定案：basis 案 0 / width 案 327） */
.fbp-progress {
  width: 327px;
  min-width: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}

/* 转码提示（disabled 态进度区）：次要文字档 亮色/三级（稿无此态，随笔记摘录档就近套） */
.fbp-hint {
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  color: var(--n-l3);
  white-space: nowrap;
  pointer-events: none;
}

/* 倍速文字钮：数字-大 亮色/三级，hover 升二级（稿 18×18 容器，宽随文案） */
.fbp-rate {
  flex: none;
  min-width: 18px;
  height: 18px;
  padding: 0;
  font-family: var(--n-font-mono);
  font-size: var(--n-fs-num-lg);
  line-height: var(--n-lh-num-lg);
  font-weight: var(--n-fw-medium);
  color: var(--n-l3);
  white-space: nowrap;
  transition: color var(--n-dur-150) var(--n-ease-in-out);
}
.fbp-rate:hover {
  color: var(--n-l2);
}
.fbp-rate:disabled {
  cursor: default;
  pointer-events: none;
}
</style>
