<script setup lang="ts">
// 筛选弹窗 · 颜色（稿 = 中间面板-多功能悬浮栏-筛选/颜色 250×339）：单选色条件（后端 colors 数组取首项）。
// ① 色盘 226×223 = SV 盘（水平 白→纯色 = 饱和 0→100、垂直 亮→黑 = 明度 100→0，色相 = 当前色色相；
//    稿位图落地 = canvas 三层渐变自绘 + 选择环定位，2026-10-09 四轮）；点击 / 拖动取色。
// ② HEX 行：16×16 色块（TextInput leading slot；空态 = 白底斜杠，稿 prose「清除后色块变成白色加一条斜杠」）
//    + HEX 输入框 + 清除钮（TagButton solid md）——清除 = 清空颜色筛选。
// ③ RGB 行：三框 50 宽（HSL 模式 = H 0-360 / S 0-100 / L 0-100）+ RGB/HSL 切换钮（稿 1_点击-模式切换，
//    钮面 = 当前模式）。输入提交 = focusout / Enter，非法值不生效（草稿保留）。
import { computed, onMounted, ref, watch } from "vue";
import TagButton from "../buttons/TagButton.vue";
import TextInput from "../inputs/TextInput.vue";
import "./bars.css";

const props = withDefaults(defineProps<{ color?: string | null }>(), { color: null });
const emit = defineEmits<{ setColor: [hex: string | null] }>();

// ---------- 颜色换算（组件内私有：hex ↔ rgb ↔ hsv/hsl） ----------
function hexToRgb(hex: string): { r: number; g: number; b: number } | null {
  const h = hex.trim().replace(/^#/, "");
  if (!/^[0-9a-fA-F]{6}$/.test(h)) return null;
  return { r: parseInt(h.slice(0, 2), 16), g: parseInt(h.slice(2, 4), 16), b: parseInt(h.slice(4, 6), 16) };
}
function rgbToHex(r: number, g: number, b: number): string {
  const p = (v: number): string => Math.round(Math.min(255, Math.max(0, v))).toString(16).padStart(2, "0").toUpperCase();
  return `#${p(r)}${p(g)}${p(b)}`;
}
function rgbToHsv(r: number, g: number, b: number): { h: number; s: number; v: number } {
  r /= 255;
  g /= 255;
  b /= 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const d = max - min;
  let h = 0;
  if (d > 0) {
    if (max === r) h = 60 * (((g - b) / d) % 6);
    else if (max === g) h = 60 * ((b - r) / d + 2);
    else h = 60 * ((r - g) / d + 4);
  }
  if (h < 0) h += 360;
  return { h, s: max === 0 ? 0 : d / max, v: max };
}
function hsvToRgb(h: number, s: number, v: number): { r: number; g: number; b: number } {
  const c = v * s;
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
  const m = v - c;
  let r = 0;
  let g = 0;
  let b = 0;
  if (h < 60) [r, g, b] = [c, x, 0];
  else if (h < 120) [r, g, b] = [x, c, 0];
  else if (h < 180) [r, g, b] = [0, c, x];
  else if (h < 240) [r, g, b] = [0, x, c];
  else if (h < 300) [r, g, b] = [x, 0, c];
  else [r, g, b] = [c, 0, x];
  return { r: (r + m) * 255, g: (g + m) * 255, b: (b + m) * 255 };
}
function rgbToHsl(r: number, g: number, b: number): { h: number; s: number; l: number } {
  r /= 255;
  g /= 255;
  b /= 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  const d = max - min;
  if (d === 0) return { h: 0, s: 0, l: l * 100 };
  const s = d / (1 - Math.abs(2 * l - 1));
  return { h: rgbToHsv(r * 255, g * 255, b * 255).h, s: s * 100, l: l * 100 };
}
function hslToRgb(h: number, s: number, l: number): { r: number; g: number; b: number } {
  s /= 100;
  l /= 100;
  const c = (1 - Math.abs(2 * l - 1)) * s;
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
  const m = l - c / 2;
  let r = 0;
  let g = 0;
  let b = 0;
  const hh = ((h % 360) + 360) % 360;
  if (hh < 60) [r, g, b] = [c, x, 0];
  else if (hh < 120) [r, g, b] = [x, c, 0];
  else if (hh < 180) [r, g, b] = [0, c, x];
  else if (hh < 240) [r, g, b] = [0, x, c];
  else if (hh < 300) [r, g, b] = [x, 0, c];
  else [r, g, b] = [c, 0, x];
  return { r: (r + m) * 255, g: (g + m) * 255, b: (b + m) * 255 };
}

// ---------- 状态 ----------
const PAD_W = 226;
const PAD_H = 223;
const DEFAULT_HUE = 204; // 品牌蓝（#0F89EC）——无色条件时的色盘底色
const hue = ref(DEFAULT_HUE); // 色盘色相（SV 盘内取色不改它，仅外部改色相时跟随）
const mode = ref<"RGB" | "HSL">("RGB");
const hexDraft = ref("");
const chanDraft = ref<[string, string, string]>(["", "", ""]);

const padRef = ref<HTMLCanvasElement>();

// 外部条件回流：同步色块 / HEX / 三框草稿与色盘色相（hue 差 < 1° 视为色盘取色回环，不重绘防抖）
watch(
  () => props.color,
  (c) => {
    const rgb = c ? hexToRgb(c) : null;
    hexDraft.value = rgb ? c!.toUpperCase() : "";
    if (rgb) {
      const hsv = rgbToHsv(rgb.r, rgb.g, rgb.b);
      if (Math.abs(hsv.h - hue.value) > 1) hue.value = hsv.h;
      if (mode.value === "RGB") chanDraft.value = [String(rgb.r), String(rgb.g), String(rgb.b)];
      else {
        const hsl = rgbToHsl(rgb.r, rgb.g, rgb.b);
        chanDraft.value = [String(Math.round(hsl.h)), String(Math.round(hsl.s)), String(Math.round(hsl.l))];
      }
    } else {
      chanDraft.value = ["", "", ""];
    }
  },
  { immediate: true },
);

// 色盘绘制：纯色底 + 白水平渐变（左→右透明）+ 黑垂直渐变（上→下不透明）
function drawPad(): void {
  const ctx = padRef.value?.getContext("2d");
  if (!ctx) return;
  ctx.fillStyle = `hsl(${hue.value}, 100%, 50%)`;
  ctx.fillRect(0, 0, PAD_W, PAD_H);
  const gw = ctx.createLinearGradient(0, 0, PAD_W, 0);
  gw.addColorStop(0, "rgba(255,255,255,1)");
  gw.addColorStop(1, "rgba(255,255,255,0)");
  ctx.fillStyle = gw;
  ctx.fillRect(0, 0, PAD_W, PAD_H);
  const gb = ctx.createLinearGradient(0, 0, 0, PAD_H);
  gb.addColorStop(0, "rgba(0,0,0,0)");
  gb.addColorStop(1, "rgba(0,0,0,1)");
  ctx.fillStyle = gb;
  ctx.fillRect(0, 0, PAD_W, PAD_H);
}
onMounted(drawPad);
watch(hue, drawPad);

// 选择环位置（无色不显示）
const ringPos = computed(() => {
  const rgb = props.color ? hexToRgb(props.color) : null;
  if (!rgb) return null;
  const hsv = rgbToHsv(rgb.r, rgb.g, rgb.b);
  return { x: hsv.s * PAD_W, y: (1 - hsv.v) * PAD_H };
});

// 色盘取色（点击 / 拖动）：x → 饱和、y → 明度
function pickAt(e: PointerEvent): void {
  const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
  const s = Math.min(1, Math.max(0, (e.clientX - rect.left) / rect.width));
  const v = 1 - Math.min(1, Math.max(0, (e.clientY - rect.top) / rect.height));
  const rgb = hsvToRgb(hue.value, s, v);
  emit("setColor", rgbToHex(rgb.r, rgb.g, rgb.b));
}
function onPadDown(e: PointerEvent): void {
  (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  pickAt(e);
}
function onPadMove(e: PointerEvent): void {
  if (e.buttons > 0) pickAt(e);
}

// HEX / 三框提交
function commitHex(): void {
  const rgb = hexToRgb(hexDraft.value);
  if (rgb) emit("setColor", rgbToHex(rgb.r, rgb.g, rgb.b));
}
function commitChan(): void {
  const [a, b, c] = chanDraft.value.map((s) => Number(s));
  if (chanDraft.value.some((s) => !s.trim()) || [a, b, c].some((v) => !Number.isFinite(v) || v < 0)) return;
  if (mode.value === "RGB") {
    if (a > 255 || b > 255 || c > 255) return;
    emit("setColor", rgbToHex(a, b, c));
  } else {
    if (a > 360 || b > 100 || c > 100) return;
    const rgb = hslToRgb(a, b, c);
    emit("setColor", rgbToHex(rgb.r, rgb.g, rgb.b));
  }
}
function toggleMode(): void {
  mode.value = mode.value === "RGB" ? "HSL" : "RGB";
  // 模式切换后按当前色回填新通道草稿
  const rgb = props.color ? hexToRgb(props.color) : null;
  if (!rgb) return;
  if (mode.value === "RGB") chanDraft.value = [String(rgb.r), String(rgb.g), String(rgb.b)];
  else {
    const hsl = rgbToHsl(rgb.r, rgb.g, rgb.b);
    chanDraft.value = [String(Math.round(hsl.h)), String(Math.round(hsl.s)), String(Math.round(hsl.l))];
  }
}

const CHANNELS = computed(() => (mode.value === "RGB" ? (["R", "G", "B"] as const) : (["H", "S", "L"] as const)));
const swatchStyle = computed(() => (props.color ? { background: props.color } : undefined));
</script>

<template>
  <div class="fb-pop fp-root fp-color">
    <!-- 色盘（SV 盘：水平饱和、垂直明度、色相随当前色；选择环 = 空心圆环稿样） -->
    <div class="fp-pad" @pointerdown="onPadDown" @pointermove="onPadMove">
      <canvas ref="padRef" :width="PAD_W" :height="PAD_H"></canvas>
      <span v-if="ringPos" class="fp-ring" :style="{ left: `${ringPos.x}px`, top: `${ringPos.y}px` }"></span>
    </div>
    <!-- HEX 行（色块 leading + 输入框 + 清除；空态色块 = 白底斜杠，稿 prose） -->
    <div class="fp-row">
      <TextInput v-model="hexDraft" size="lg" placeholder="#RRGGBB" @focusout="commitHex" @keydown.enter="commitHex">
        <template #leading>
          <span class="fp-swatch" :class="{ empty: !color }" :style="swatchStyle"></span>
        </template>
      </TextInput>
      <TagButton label="清除" size="md" @click="emit('setColor', null)" />
    </div>
    <!-- RGB / HSL 行（三框 + 模式切换钮，钮面 = 当前模式，稿 1_点击-模式切换） -->
    <div class="fp-row">
      <div class="fp-trio">
        <TextInput
          v-for="(ch, i) in CHANNELS"
          :key="ch"
          v-model="chanDraft[i]"
          size="lg"
          class="fp-chan"
          :placeholder="ch"
          @focusout="commitChan"
          @keydown.enter="commitChan"
        />
      </div>
      <TagButton :label="mode" size="md" class="fp-mode" @click="toggleMode" />
    </div>
  </div>
</template>

<style scoped>
.fp-color {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
  padding: var(--n-space-12);
  width: 250px;
}
/* 色盘：226×223 稿值，圆角 8 与壳同档收边 */
.fp-pad {
  position: relative;
  width: 226px;
  height: 223px;
  border-radius: var(--n-radius-8);
  overflow: hidden;
  cursor: crosshair;
  touch-action: none;
}
.fp-pad canvas {
  display: block;
}
/* 选择环（稿：空心圆环骑盘缘） */
.fp-ring {
  position: absolute;
  width: 14px;
  height: 14px;
  border: 2px solid #fff;
  border-radius: 50%;
  transform: translate(-50%, -50%);
  box-shadow: 0 0 2px rgba(0, 0, 0, 0.6);
  pointer-events: none;
}
.fp-row {
  display: flex;
  align-items: center;
  gap: var(--n-space-12);
}
/* HEX 色块（16×16 圆角 4 + 1px 亮色/半透明-1 边；空态 = 白底斜杠，稿 prose） */
.fp-swatch {
  flex: none;
  width: 16px;
  height: 16px;
  border-radius: var(--n-radius-4);
  border: 1px solid var(--n-l-a1);
  background: #fff;
}
.fp-swatch.empty {
  background: linear-gradient(to top right, #fff calc(50% - 1px), var(--n-l3) 50%, #fff calc(50% + 1px));
}
.fp-trio {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
  flex: 1;
  min-width: 0;
}
/* 三框各 50 宽固定（稿值）——不定 flex:1：TextInput 根恒 width:100%，均分撑满会把切换钮挤溢出 */
.fp-trio .fp-chan {
  width: 50px;
  flex: none;
}
</style>
