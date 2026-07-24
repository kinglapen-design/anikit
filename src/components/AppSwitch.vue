<script setup lang="ts">
// 开关按钮（02_通用组件/01_按钮 08）：标准 50×28 圆钮 20 / 小号 40×24 圆钮 16（small prop）；
// 未选中 font-icon-5（选中色），选中品牌色-1，圆钮 font-icon-1，滑动过渡 0.2s（2026-07-24 用户定）。
withDefaults(defineProps<{ small?: boolean }>(), { small: false });
const model = defineModel<boolean>({ required: true });
</script>

<template>
  <button class="app-switch" :class="{ on: model, small }" @click="model = !model">
    <span class="knob" />
  </button>
</template>

<style scoped>
.app-switch {
  display: flex;
  align-items: center;
  width: var(--switch-w);
  height: var(--switch-h);
  padding: var(--space-4);
  border-radius: 999px;
  background: var(--font-icon-5);
  transition: background 0.2s ease;
  flex: none;
}
.app-switch.small {
  width: var(--switch-small-w);
  height: var(--switch-small-h);
}
.app-switch.on {
  background: var(--brand-1);
}
.knob {
  width: var(--switch-knob);
  height: var(--switch-knob);
  border-radius: 999px;
  background: var(--font-icon-1);
  flex: none;
  transition: transform 0.2s ease;
}
.app-switch.small .knob {
  width: var(--switch-small-knob);
  height: var(--switch-small-knob);
}
/* 滑动：选中时圆钮平移至右端（轨道宽 - 圆钮 - 两侧内边距） */
.app-switch.on .knob {
  transform: translateX(calc(var(--switch-w) - var(--switch-knob) - var(--space-8)));
}
.app-switch.small.on .knob {
  transform: translateX(calc(var(--switch-small-w) - var(--switch-small-knob) - var(--space-8)));
}
</style>
