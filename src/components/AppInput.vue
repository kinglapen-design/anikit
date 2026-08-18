<script setup lang="ts">
// 输入框（figma-css 11_输入框）：small = padding 4/8、C2 底、28 高；medium = padding 6/8、C1 底、32 高；
// large = padding 8/12、C2 底、36 高；圆角 6（figma 导出值，2026-08-07 用户定：token 8px 口径作废）；
// 边框常态 暗-悬停 1px，聚焦 品牌色 1.5px 淡入淡出（00_要求；内阴影模拟防抖动）；
// 文本正文，占位符极淡档。
// clearable（2026-08-18，36/37 稿输入框内"清除 ×"，09 待办收口）：有内容时尾部 20×20 清除钮
//（AppSearchBar 同款：标签档、hover 暗-悬停罩）。包裹壳仅 clearable 时渲染——
// 非 clearable 保持裸 input 根元素（CreateLibraryModal 的 $el.focus() 依赖，勿合并分支）。
import AppIcon from "./AppIcon.vue";

withDefaults(
  defineProps<{ size?: "small" | "medium" | "large"; placeholder?: string; disabled?: boolean; clearable?: boolean }>(),
  { size: "medium", placeholder: undefined, disabled: false, clearable: false },
);
const model = defineModel<string>({ default: "" });
</script>

<template>
  <div v-if="clearable" class="app-input-shell" :class="[size, { disabled }]">
    <input v-model="model" class="app-input inner" :placeholder="placeholder" :disabled="disabled" />
    <button v-if="model && !disabled" class="clear" v-tip="'清除'" @click="model = ''">
      <AppIcon name="clear" :size="12" />
    </button>
  </div>
  <input v-else v-model="model" class="app-input" :class="size" :placeholder="placeholder" :disabled="disabled" />
</template>

<style scoped>
.app-input {
  width: 100%;
  border-radius: var(--radius-6);
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

/* clearable 壳（36/37 稿）：底色/边框/聚焦由壳承担，内嵌 input 透明去框；
   清除钮 = AppSearchBar 同款（20×20 标签档，hover 暗-悬停罩） */
.app-input-shell {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  width: 100%;
  border-radius: var(--radius-6);
  box-shadow: inset 0 0 0 1px var(--line-hover);
  transition: box-shadow 0.2s ease;
}
.app-input-shell.small {
  padding: var(--space-4) var(--space-8);
  background: var(--c2);
}
.app-input-shell.medium {
  padding: var(--space-6) var(--space-8);
  background: var(--c1);
}
.app-input-shell.large {
  padding: var(--space-8) var(--space-12);
  background: var(--c2);
}
.app-input-shell:focus-within {
  box-shadow:
    inset 0 0 0 1px var(--line-hover),
    inset 0 0 0 var(--border-strong) var(--brand);
}
.app-input-shell.disabled {
  color: var(--text-faint);
}
.app-input-shell .app-input.inner {
  flex: 1;
  min-width: 0;
  padding: 0;
  background: transparent;
  border-radius: 0;
  box-shadow: none;
}
.app-input-shell .app-input.inner:focus {
  box-shadow: none;
}
.app-input-shell .clear {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  border-radius: var(--radius-6);
  color: var(--text-label);
  flex: none;
  transition: background 0.15s ease;
}
.app-input-shell .clear:hover {
  background: var(--line-hover);
}
</style>
