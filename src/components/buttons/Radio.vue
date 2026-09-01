<script setup lang="ts">
// 单选框——docs/UI 重构/Design Token/组件/按钮/单选框.md：
// 外圈 18×18 圆形（通用/一级底 + 通用/五级描边，hover 未选中描边 通用/六级——2026-09-01 用户定，
// 与选中外圈同档），内 padding 2；
// 选中 = 内圆 14×14 品牌/常态底 + 亮色/半透明-1 描边，弹出过冲动画（档同复选框）；
// 文字 = 正文-加粗 13/18：选中 亮色/二级、未选中 亮色/三级（色过渡 150ms）。
// v-model 值不等时点击选中（成组使用，每组一个 v-model 值）。
// 选中态 hover / 禁用态稿未给，待补稿。
import { computed } from "vue";

const model = defineModel<string | number | boolean>({ required: true });
const props = withDefaults(
  defineProps<{ value: string | number | boolean; label?: string; forceState?: "" | "hover" }>(),
  { forceState: "" },
);
const checked = computed(() => model.value === props.value);
</script>

<template>
  <button
    class="kit-radio"
    :class="[{ on: checked }, forceState ? `force-${forceState}` : '']"
    @click="model = value"
  >
    <span class="ring">
      <span class="dot" />
    </span>
    <span v-if="label" class="label">{{ label }}</span>
  </button>
</template>

<style scoped>
.kit-radio {
  display: inline-flex;
  align-items: center;
  gap: var(--n-space-6);
  padding: 0;
  height: 18px;
  flex: none;
  color: var(--n-l3);
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  font-weight: var(--n-fw-bold);
  transition: color var(--n-dur-150) var(--n-ease-in-out);
}
.kit-radio.on {
  color: var(--n-l2);
}
.ring {
  width: 18px;
  height: 18px;
  padding: var(--n-space-2);
  border-radius: 50%;
  background: var(--n-g1);
  box-shadow: inset 0 0 0 1px var(--n-g5);
  display: flex;
  align-items: center;
  justify-content: center;
  flex: none;
  transition: box-shadow var(--n-dur-150) var(--n-ease-in-out);
}
.kit-radio:hover:not(.on) .ring,
.kit-radio.force-hover:not(.on) .ring,
.kit-radio.on .ring {
  box-shadow: inset 0 0 0 1px var(--n-g6);
}
.dot {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: var(--n-brand);
  box-shadow: inset 0 0 0 1px var(--n-l-a1);
  transform: scale(0);
  transition: transform var(--n-dur-150) var(--n-ease-in);
}
.kit-radio.on .dot {
  transform: scale(1);
  transition: transform var(--n-dur-250) cubic-bezier(0.34, 1.56, 0.64, 1);
}
</style>
