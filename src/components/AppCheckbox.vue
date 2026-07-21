<script setup lang="ts">
// 单选框 / 复选框（02_通用组件/01_按钮 04）：按钮 16×16、线条 C4-15、圆角 4；
// 选中：边框品牌色-2 + 填充品牌色-1，文字品牌色-1。
import AppIcon from "./AppIcon.vue";

const model = defineModel<boolean>({ required: true });
defineProps<{ label: string }>();
</script>

<template>
  <button class="app-checkbox" :class="{ on: model }" @click="model = !model">
    <span class="box">
      <AppIcon v-if="model" name="check" :size="8" />
    </span>
    <span class="label">{{ label }}</span>
  </button>
</template>

<style scoped>
.app-checkbox {
  display: inline-flex;
  align-items: center;
  gap: var(--space-6);
  padding: 0;
  color: var(--c4-40);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  flex: none;
}
.box {
  width: 16px;
  height: 16px;
  border-radius: var(--radius-4);
  box-shadow: inset 0 0 0 1px var(--c4-15);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--c4);
  line-height: 0;
  flex: none;
}
.app-checkbox:hover .box {
  box-shadow: inset 0 0 0 1px var(--c4-40);
}
.app-checkbox.on {
  color: var(--c4-70);
}
.app-checkbox.on .box {
  box-shadow: inset 0 0 0 1px var(--brand-2);
  background: var(--brand-1);
}
/* 选中时对勾 C4 全亮（二级菜单对勾颜色独立，不受影响） */
.app-checkbox.on .box :deep(svg [stroke-opacity]) {
  stroke-opacity: 1;
}
</style>
