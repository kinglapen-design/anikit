<script setup lang="ts">
// 筛选弹窗 · 比例（稿 = 中间面板-多功能悬浮栏-筛选/比例 164×148）：预设三档（横图 / 方形 / 竖图）
// 多选 + 尾行自定义「宽:高」（输入 40×26 两框 + 生效对勾，稿：自定义行带对勾 = 自定义条件亦参与
// 选中态）；item = .fp-item 列表族口径（bars.css）；按稿无清除钮（预设再点取消、自定义两框清空撤销）。
// 提交时机 = focusout / Enter（两框齐全且为正数才生效；单框半成品不提交）。
import { computed, ref, watch } from "vue";
import AppIcon from "../AppIcon.vue";
import TextInput from "../inputs/TextInput.vue";
import "./bars.css";

const props = withDefaults(
  defineProps<{
    selected?: readonly string[]; // 预设 key + 自定义 "w:h" 混合（后端 ratios 数组原样）
    forceHover?: string; // 预览页静态验收：某档 hover
  }>(),
  { selected: () => [], forceHover: undefined },
);
const emit = defineEmits<{ toggle: [key: string]; setCustom: [ratio: string | null] }>();

const PRESETS = [
  { key: "landscape", label: "横图" },
  { key: "square", label: "方形" },
  { key: "portrait", label: "竖图" },
] as const;

// 自定义比 = selected 里首个非预设项（"w:h"）；外部变化（清除等）回填输入框
const customOf = computed(() => props.selected.find((s) => !PRESETS.some((p) => p.key === s)) ?? null);
const cw = ref("");
const ch = ref("");
watch(
  customOf,
  (v) => {
    const [a, b] = v?.split(":") ?? ["", ""];
    cw.value = a ?? "";
    ch.value = b ?? "";
  },
  { immediate: true },
);
function commit(): void {
  const a = Number(cw.value);
  const b = Number(ch.value);
  if (cw.value.trim() && ch.value.trim() && a > 0 && b > 0) emit("setCustom", `${a}:${b}`);
  else if (!cw.value.trim() && !ch.value.trim()) emit("setCustom", null);
  // 单框填写 = 半成品不提交（草稿保留）
}
// 自定义反选：对勾点击 = 取消自定义并清空两框（2026-10-09 走查）
function clearCustom(): void {
  cw.value = "";
  ch.value = "";
  emit("setCustom", null);
}
</script>

<template>
  <div class="fb-pop fp-root fp-ratio">
    <div class="fp-list">
      <button
        v-for="p in PRESETS"
        :key="p.key"
        class="fp-item"
        :class="[{ on: selected.includes(p.key), 'force-hover': forceHover === p.key }]"
        @click="emit('toggle', p.key)"
      >
        <span>{{ p.label }}</span>
        <AppIcon name="check" :size="18" class="fp-check" />
      </button>
      <div class="fp-custom" v-tip="'自定义宽高比'">
        <TextInput v-model="cw" size="sm" class="fp-num" @focusout="commit" @keydown.enter="commit" />
        <span class="fp-colon">:</span>
        <TextInput v-model="ch" size="sm" class="fp-num" @focusout="commit" @keydown.enter="commit" />
        <!-- 自定义反选（2026-10-09 走查）：对勾可点 = 取消自定义并清框；未生效时隐形不可点 -->
        <button class="fp-uncheck" :class="{ on: customOf }" v-tip="customOf ? '取消自定义' : undefined" :disabled="!customOf" @click="clearCustom">
          <AppIcon name="check" :size="18" />
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.fp-ratio {
  width: 164px;
}
/* 自定义两框（稿 40×26） */
.fp-num {
  width: 40px;
  flex: none;
}
/* 自定义对勾钮（复用 .fp-check 显隐口径；生效时可点 = 取消自定义，未生效隐形不响应） */
.fp-uncheck {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: none;
  width: 18px;
  height: 18px;
  padding: 0;
  color: var(--n-l1);
  opacity: 0;
  pointer-events: none;
  transition: opacity var(--n-dur-150) var(--n-ease-in-out);
}
.fp-uncheck.on {
  opacity: 1;
  pointer-events: auto;
  cursor: pointer;
}
</style>
