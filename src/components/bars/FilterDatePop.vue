<script setup lang="ts">
// 筛选弹窗 · 日期（稿 = 中间面板-多功能悬浮栏-筛选/日期 207×238；添加日期 / 修改日期两按钮共用）：
// 预设六档（当天 / 昨天 / 最近一周 / 最近一月 / 最近一年 / 更早）多选 + 尾行自定义年月日
// （年 55 / 月 40 / 日 40 三框 + 生效对勾；任意组合生效——只填年 = 该年全部，接口 11 口径）；
// item = .fp-item 列表族口径（bars.css）；按稿无清除钮。提交时机 = focusout / Enter，
// 任一框有正整数即生效、全空 = 撤销自定义。
import { ref, watch } from "vue";
import AppIcon from "../AppIcon.vue";
import TextInput from "../inputs/TextInput.vue";
import "./bars.css";

interface DateYmd {
  year?: number;
  month?: number;
  day?: number;
}

const props = withDefaults(
  defineProps<{
    buckets?: readonly string[]; // 预设档位选中集（后端 DateCond.buckets 原样）
    custom?: DateYmd | null; // 自定义年月日（后端 DateCond.custom 原样）
    forceHover?: string;
  }>(),
  { buckets: () => [], custom: null, forceHover: undefined },
);
const emit = defineEmits<{ toggle: [key: string]; setCustom: [ymd: DateYmd | null] }>();

const PRESETS = [
  { key: "today", label: "当天" },
  { key: "yesterday", label: "昨天" },
  { key: "week", label: "最近一周" },
  { key: "month", label: "最近一月" },
  { key: "year", label: "最近一年" },
  { key: "earlier", label: "更早" },
] as const;

// 自定义三框（外部变化回填）
const cy = ref("");
const cm = ref("");
const cd = ref("");
watch(
  () => props.custom,
  (c) => {
    cy.value = c?.year != null ? String(c.year) : "";
    cm.value = c?.month != null ? String(c.month) : "";
    cd.value = c?.day != null ? String(c.day) : "";
  },
  { immediate: true },
);
function commit(): void {
  const year = cy.value.trim() ? Number(cy.value) : undefined;
  const month = cm.value.trim() ? Number(cm.value) : undefined;
  const day = cd.value.trim() ? Number(cd.value) : undefined;
  const valid = (v: number | undefined, max: number) => v === undefined || (Number.isInteger(v) && v >= 1 && v <= max);
  if (year === undefined && month === undefined && day === undefined) {
    emit("setCustom", null);
  } else if (valid(year, 9999) && valid(month, 12) && valid(day, 31)) {
    emit("setCustom", { year, month, day });
  }
  // 非法值（月 13 / 日 40 等）= 半成品不提交（草稿保留）
}
// 自定义反选：对勾点击 = 取消自定义并清空三框（2026-10-09 走查）
function clearCustom(): void {
  cy.value = "";
  cm.value = "";
  cd.value = "";
  emit("setCustom", null);
}
</script>

<template>
  <div class="fb-pop fp-root fp-date">
    <div class="fp-list">
      <button
        v-for="p in PRESETS"
        :key="p.key"
        class="fp-item"
        :class="[{ on: buckets.includes(p.key), 'force-hover': forceHover === p.key }]"
        @click="emit('toggle', p.key)"
      >
        <span>{{ p.label }}</span>
        <AppIcon name="check" :size="18" class="fp-check" />
      </button>
      <div class="fp-custom" v-tip="'自定义日期（年 / 月 / 日任意组合）'">
        <TextInput v-model="cy" size="sm" class="fp-year" placeholder="年" @focusout="commit" @keydown.enter="commit" />
        <TextInput v-model="cm" size="sm" class="fp-md" placeholder="月" @focusout="commit" @keydown.enter="commit" />
        <TextInput v-model="cd" size="sm" class="fp-md" placeholder="日" @focusout="commit" @keydown.enter="commit" />
        <!-- 自定义反选（2026-10-09 走查）：对勾可点 = 取消自定义并清框；未生效时隐形不可点 -->
        <button class="fp-uncheck" :class="{ on: custom }" v-tip="custom ? '取消自定义' : undefined" :disabled="!custom" @click="clearCustom">
          <AppIcon name="check" :size="18" />
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.fp-date {
  width: 207px;
}
/* 自定义三框（稿 年 55 / 月 40 / 日 40） */
.fp-year {
  width: 55px;
  flex: none;
}
.fp-md {
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
