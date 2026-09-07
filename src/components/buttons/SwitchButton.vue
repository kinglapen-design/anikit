<script setup lang="ts">
// 开关按钮——docs/06_UI重构/Design Token/组件/按钮/开关按钮.md：
// 轨道 30×16 胶囊圆角 padding 2，圆钮 12×12；
// 关 = 通用/四级底 + 通用/六级描边 + 圆钮亮色/二级居左，
// 开 = 品牌/常态底（无描边）+ 圆钮亮色/一级居右（右开左关）；
// 横移动画 250ms 小位移（稿 prose：开关横移动画），底色/描边过渡 150ms。
// hover / 禁用态稿未给：disabled 仅不响应点击、无视觉差异，待补稿。
const model = defineModel<boolean>({ required: true });
withDefaults(defineProps<{ disabled?: boolean }>(), { disabled: false });
</script>

<template>
  <button class="kit-switch" :class="{ on: model }" :disabled="disabled" @click="model = !model">
    <span class="knob" />
  </button>
</template>

<style scoped>
.kit-switch {
  display: flex;
  align-items: center;
  width: 30px;
  height: 16px;
  padding: var(--n-space-2);
  border-radius: 999px;
  background: var(--n-g4);
  box-shadow: inset 0 0 0 1px var(--n-g6);
  flex: none;
  transition:
    background var(--n-dur-150) var(--n-ease-in-out),
    box-shadow var(--n-dur-150) var(--n-ease-in-out);
}
.kit-switch.on {
  background: var(--n-brand);
  box-shadow: none;
}
.knob {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: var(--n-l2);
  flex: none;
  transition:
    transform var(--n-dur-250) var(--n-ease-in-out),
    background var(--n-dur-150) var(--n-ease-in-out);
}
/* 开：圆钮平移至右端（轨道宽 - 两侧 padding - 圆钮宽） */
.kit-switch.on .knob {
  transform: translateX(14px);
  background: var(--n-l1);
}
.kit-switch:disabled {
  cursor: default;
}
</style>
