<script setup lang="ts">
// 单选框 / 复选框（02_通用组件/01_按钮 04）：按钮 16×16、线条 C4-15、圆角 4；
// 选中：边框品牌色-2 + 填充品牌色-1，文字品牌色-1。
const model = defineModel<boolean>({ required: true });
defineProps<{ label: string }>();
</script>

<template>
  <button class="app-checkbox" :class="{ on: model }" @click="model = !model">
    <span class="box">
      <svg v-if="model" width="8" height="8" viewBox="0 0 8 8">
        <path
          d="M1.5 4.3 3.2 6 6.5 2"
          stroke="currentColor"
          stroke-width="1.2"
          fill="none"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
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
  border: 1px solid var(--c4-15);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--c4);
  flex: none;
}
.app-checkbox:hover .box {
  border-color: var(--c4-40);
}
.app-checkbox.on {
  color: var(--brand-1);
}
.app-checkbox.on .box {
  border-color: var(--brand-2);
  background: var(--brand-1);
}
</style>
