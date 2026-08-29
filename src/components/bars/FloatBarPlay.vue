<script setup lang="ts">
// 播放条：稿为全量陈列（播放/暂停、快退/快进、上一帧/下一帧、声音/静音、时间码、进度条、
// 倍速、循环、笔记、设为封面、全屏），「根据上下文，选择保留哪些按钮」→ show prop 按组删减；
// 播放/暂停、声音/静音 = 单钮按态切换（playing / muted prop，稿两态图标并列是全量画法）。
// 图标 = 按钮族 IconButton bare 复用（18 实寸 亮色/三级，hover 升二级——稿 hover-按钮组口径）；
// 时间码 = 数字-大（当前 / 分隔 亮色/二级、总计 三级）；倍速 = 数字-大 三级，hover 升二级；
// 进度条 = BarSlider light（稿无填充段），宽 327 = 稿值链还原（实际宽度随上下文，业务接入轮定）。
// 快退/快进 = seek（原样 ◀◀ 快退 / flip-h ▶▶ 快进）；上/下帧 = step-frame（原样 |◀ 上一帧 / flip-h 下一帧）。
// 循环 / 笔记开启态稿未画 → 不改视觉（同按钮族禁用态口径，记 09 待办）。
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
    rateText?: string;
    curr?: string; // 时间码当前（帧数 / 时长文本，父级格式化）
    total?: string; // 时间码总计
    show?: readonly PlayGroup[];
  }>(),
  {
    playing: false,
    muted: false,
    loopOn: false,
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
  "open-rate": []; // 父级开倍率弹层
  "toggle-loop": [];
  "open-note": []; // 父级开笔记弹层
  "set-cover": [];
  "open-fullscreen": [];
}>();

function has(g: PlayGroup): boolean {
  return props.show.includes(g);
}
</script>

<template>
  <div class="fb-mode fbp-root">
    <IconButton :name="playing ? 'pause' : 'play'" variant="bare" @click="emit('toggle-play')" />
    <template v-if="has('transport')">
      <IconButton name="seek" variant="bare" @click="emit('seek-back')" />
      <IconButton name="seek" variant="bare" class="flip" @click="emit('seek-forward')" />
    </template>
    <template v-if="has('step')">
      <IconButton name="step-frame" variant="bare" @click="emit('step-prev')" />
      <IconButton name="step-frame" variant="bare" class="flip" @click="emit('step-next')" />
    </template>
    <IconButton
      v-if="has('volume')"
      :name="muted ? 'mute' : 'volume'"
      variant="bare"
      @click="emit('volume-click')"
    />
    <div class="fbp-tc">
      <span class="cur">{{ curr }}</span>
      <span class="sep">/</span>
      <span class="tot">{{ total }}</span>
    </div>
    <div class="fbp-progress">
      <BarSlider v-model="progress" tone="light" />
    </div>
    <button v-if="has('rate')" class="fbp-rate" @click="emit('open-rate')">{{ rateText }}</button>
    <IconButton
      v-if="has('loop')"
      name="loop"
      variant="bare"
      :class="{ on: loopOn }"
      @click="emit('toggle-loop')"
    />
    <IconButton v-if="has('note')" name="note" variant="bare" @click="emit('open-note')" />
    <IconButton v-if="has('cover')" name="set-cover" variant="bare" @click="emit('set-cover')" />
    <IconButton
      v-if="has('fullscreen')"
      name="fullscreen"
      variant="bare"
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

/* 进度条：宽 327 = 稿值（播放条 799 宽链还原；自适应宽度归业务接入轮） */
.fbp-progress {
  flex: none;
  width: 327px;
  display: flex;
  align-items: center;
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
</style>
