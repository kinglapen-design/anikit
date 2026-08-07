<script setup lang="ts">
// 复选框（figma-css 01_按钮 1.4）：框 14×14 圆角 4；
// 未选中 C4 底 + 暗-悬停边框（hover C5），选中品牌色底 + 亮-选中边框 + 高亮对勾（透明度过渡）；
// 标题未选中 标签档 / 选中 常态档。
import AppIcon from "./AppIcon.vue";

const model = defineModel<boolean>({ required: true });
defineProps<{ label?: string }>();
</script>

<template>
  <button class="app-checkbox" :class="{ on: model }" @click="model = !model">
    <span class="box">
      <AppIcon v-if="model" name="check" :size="10" class="mark" />
    </span>
    <span v-if="label" class="label">{{ label }}</span>
  </button>
</template>

<style scoped>
.app-checkbox {
  display: inline-flex;
  align-items: center;
  gap: var(--space-6);
  padding: 0;
  height: 20px;
  color: var(--text-label);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  flex: none;
}
.app-checkbox.on {
  color: var(--text-normal);
}
.box {
  width: 14px;
  height: 14px;
  border-radius: var(--radius-4);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--line-hover);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-highlight);
  line-height: 0;
  flex: none;
  transition:
    background 0.15s ease,
    box-shadow 0.15s ease;
}
.app-checkbox:hover .box {
  background: var(--c5);
}
.mark {
  transition: opacity 0.15s ease;
}
.app-checkbox.on .box {
  background: var(--brand);
  box-shadow: inset 0 0 0 1px var(--line-active);
}
</style>
