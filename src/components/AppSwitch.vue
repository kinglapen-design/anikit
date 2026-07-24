<script setup lang="ts">
// 开关按钮（02_通用组件/01_按钮 08）：50×28 胶囊 / 圆钮 20×20；
// 未选中 font-icon-5（选中色），选中品牌色-1，圆钮 font-icon-1。
const model = defineModel<boolean>({ required: true });
</script>

<template>
  <button class="app-switch" :class="{ on: model }" @click="model = !model">
    <span class="knob" />
  </button>
</template>

<style scoped>
.app-switch {
  display: flex;
  align-items: center;
  width: 50px;
  height: 28px;
  padding: var(--space-4);
  border-radius: 14px;
  background: var(--font-icon-5);
  transition: background 0.2s ease;
  flex: none;
}
.app-switch.on {
  background: var(--brand-1);
  justify-content: flex-end;
}
.knob {
  width: 20px;
  height: 20px;
  border-radius: 999px;
  background: var(--font-icon-1);
  flex: none;
}
</style>
