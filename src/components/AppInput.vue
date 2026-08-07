<script setup lang="ts">
// 输入框（figma-css 11_输入框）：small = padding 4/8、C2 底、28 高；medium = padding 6/8、C1 底、32 高；
// large = padding 8/12、C2 底、36 高；圆角 8（token：输入框归 8px 组；figma 导出值 6，以 token 为准）；
// 边框常态 暗-悬停 1px，聚焦 品牌色 1.5px 淡入淡出（00_要求；内阴影模拟防抖动）；
// 文本正文，占位符极淡档。
withDefaults(
  defineProps<{ size?: "small" | "medium" | "large"; placeholder?: string; disabled?: boolean }>(),
  { size: "medium", placeholder: undefined, disabled: false },
);
const model = defineModel<string>({ default: "" });
</script>

<template>
  <input v-model="model" class="app-input" :class="size" :placeholder="placeholder" :disabled="disabled" />
</template>

<style scoped>
.app-input {
  width: 100%;
  border-radius: var(--radius-8);
  box-shadow: inset 0 0 0 1px var(--line-hover);
  color: var(--text-normal);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  transition: box-shadow 0.2s ease;
}
.app-input.small {
  padding: var(--space-4) var(--space-8);
  background: var(--c2);
}
.app-input.medium {
  padding: var(--space-6) var(--space-8);
  background: var(--c1);
}
.app-input.large {
  padding: var(--space-8) var(--space-12);
  background: var(--c2);
}
.app-input::placeholder {
  color: var(--text-faint);
}
/* 聚焦：品牌色 1.5px 边框淡入（00_要求；失焦由 transition 自然淡出） */
.app-input:focus {
  box-shadow:
    inset 0 0 0 1px var(--line-hover),
    inset 0 0 0 var(--border-strong) var(--brand);
}
.app-input:disabled {
  cursor: default;
  color: var(--text-faint);
}
</style>
