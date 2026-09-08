<script setup lang="ts">
// 条形滑块——播放条 进度条 / 播放条-音量滑块 / 工具栏-尺寸滑块 三稿合一（轨道 + 圆形拖钮）：
// tone：light = 播放条进度条（轨道 亮色/半透明-3，拖钮 1px 描边同档；**已播放填充段 = 亮色/三级**
//       ——2026-09-09 四轮用户定稿，原「无填充段」口径作废）；
//       dark = 音量滑块（轨道 通用/四级，拖钮 1px 描边同档；无填充）；
//       size = 工具栏尺寸滑块（轨道 通用/三级，拖钮无描边——内容容器-工具栏稿值；无填充）。
// 轨道 = 6px 高圆头横条（稿 height 0 + border 3 的等效渲染，圆头 99px 同卡片波形先例，非新造 token）；
// 拖钮 10px 圆形 亮色/二级底 + 1px 描边（圆形 = 稿椭圆语义，figma-css 未导 radius）。
// 交互 = pointerdown 跳值 + window pointermove 拖动（窗口监听同款，速查表 §9 ③）；hover/按下态稿未画 → 不做。
// 宽度恒 100% 随父容器（播放条进度 327 / 音量 100 均为父级稿值语境）。
import { ref } from "vue";

withDefaults(defineProps<{ tone?: "light" | "dark" | "size" }>(), { tone: "light" });
const model = defineModel<number>({ default: 0 }); // 0~1
// 拖动起止（2026-09-09 预览悬浮栏接入轮补）：视频 scrub 等业务需在拖动期挂 scrubbing 旗标
// （拖动期 timeupdate 不回写、松手对齐真实位置，previewMedia.setScrubbing 口径）
const emit = defineEmits<{ "drag-start": []; "drag-end": [] }>();

const rootEl = ref<HTMLElement>();

function setFromX(clientX: number): void {
  const el = rootEl.value;
  if (!el) return;
  const r = el.getBoundingClientRect();
  model.value = Math.min(1, Math.max(0, (clientX - r.left) / r.width));
}

function onDown(e: PointerEvent): void {
  setFromX(e.clientX);
  emit("drag-start");
  const move = (ev: PointerEvent) => setFromX(ev.clientX);
  const up = (): void => {
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", up);
    emit("drag-end");
  };
  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", up);
}
</script>

<template>
  <div ref="rootEl" class="bs-root" :class="tone" @pointerdown="onDown">
    <div class="bs-track" />
    <!-- 已播放填充段 = 亮色/三级（仅 light 播放条档，2026-09-09 四轮用户定稿）；音量/尺寸档无 -->
    <div v-if="tone === 'light'" class="bs-fill" :style="{ width: `${model * 100}%` }" />
    <div class="bs-knob" :style="{ left: `${model * 100}%` }" />
  </div>
</template>

<style scoped>
.bs-root {
  position: relative;
  display: flex;
  align-items: center;
  width: 100%;
  height: 10px;
  cursor: pointer;
}
.bs-track {
  width: 100%;
  height: 6px;
  border-radius: 99px; /* 圆头稿值字面量（同卡片波形 99px 先例，非新造 token） */
}
.bs-root.light .bs-track {
  background: var(--n-l-a3);
}
/* 已播放填充段（仅 light 播放条档）：亮色/三级，轨道同高同圆头，左缘到拖钮位置（2026-09-09 四轮用户定稿） */
.bs-fill {
  position: absolute;
  left: 0;
  top: 2px; /* 轨道纵向居中位（root 高 10、轨道高 6） */
  height: 6px;
  border-radius: 99px; /* 圆头同轨道口径 */
  background: var(--n-l3);
  pointer-events: none;
}
.bs-root.dark .bs-track {
  background: var(--n-g4);
}
/* size = 工具栏尺寸滑块：轨道 通用/三级，拖钮无描边（稿值） */
.bs-root.size .bs-track {
  height: 3px; /* 稿滑轨 = 3px 线（figma height 0 + border 3 = 线宽 3，非盒高 6——2026-08-30 用户实机纠正） */
  background: var(--n-g3);
}
.bs-root.size .bs-knob {
  border: none;
}
.bs-knob {
  position: absolute;
  top: 0;
  width: 10px;
  height: 10px;
  box-sizing: border-box;
  border-radius: 50%;
  background: var(--n-l2);
  transform: translateX(-50%);
}
.bs-root.light .bs-knob {
  border: 1px solid var(--n-l-a3);
}
.bs-root.dark .bs-knob {
  border: 1px solid var(--n-g4);
}
</style>
