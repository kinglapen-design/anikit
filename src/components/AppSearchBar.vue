<script setup lang="ts">
// 搜索框（figma-css 11_输入框 3 / 04_右键菜单 1）：大输入框（C1 底、padding 8/12、36 高、圆角 8）
// + 搜索图标（标签档）+ 清除按钮（有内容时显示，clear 12 标签档，hover 叠暗-悬停）；
// 聚焦 品牌色 1.5px 边框淡入淡出；占位符极淡档。
import AppIcon from "./AppIcon.vue";

withDefaults(defineProps<{ placeholder?: string }>(), { placeholder: "搜索" });
const model = defineModel<string>({ default: "" });
</script>

<template>
  <div class="app-search">
    <AppIcon name="search" :size="18" class="lead" />
    <input v-model="model" class="field" :placeholder="placeholder" />
    <button v-if="model" class="clear" title="清除" @click="model = ''">
      <AppIcon name="clear" :size="12" />
    </button>
  </div>
</template>

<style scoped>
.app-search {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  padding: var(--space-8) var(--space-12);
  border-radius: var(--radius-8);
  background: var(--c1);
  box-shadow: inset 0 0 0 1px var(--line-hover);
  transition: box-shadow 0.2s ease;
}
/* 聚焦：品牌色 1.5px 边框淡入（内阴影模拟防抖动） */
.app-search:focus-within {
  box-shadow:
    inset 0 0 0 1px var(--line-hover),
    inset 0 0 0 var(--border-strong) var(--brand);
}
.lead {
  color: var(--text-label);
  flex: none;
}
.field {
  flex: 1;
  min-width: 0;
  color: var(--text-normal);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
}
.field::placeholder {
  color: var(--text-faint);
}
.clear {
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
.clear:hover {
  background: var(--line-hover);
}
</style>
