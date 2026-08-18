<script setup lang="ts">
// 搜索框（figma-css 11_输入框 3 / 04_右键菜单 1）：大输入框（C1 底、padding 8/12、36 高、圆角 6）
// + 搜索图标（标签档）+ 清除按钮（有内容时显示，clear 12 标签档，hover 叠暗-悬停）；
// 聚焦 品牌色 1.5px 边框 = ::after 状态层 opacity 淡入淡出（2026-08-19 残留修复，踩坑 036：
// box-shadow 颜色过渡是重绘型动画，GPU 损伤区偶发漏覆盖致失焦后品牌色像素残留）；占位符极淡档。
import AppIcon from "./AppIcon.vue";

withDefaults(defineProps<{ placeholder?: string }>(), { placeholder: "搜索" });
const model = defineModel<string>({ default: "" });
</script>

<template>
  <div class="app-search">
    <AppIcon name="search" :size="18" class="lead" />
    <input v-model="model" class="field" :placeholder="placeholder" />
    <button v-if="model" class="clear" v-tip="'清除'" @click="model = ''">
      <AppIcon name="clear" :size="12" />
    </button>
  </div>
</template>

<style scoped>
.app-search {
  position: relative; /* 聚焦层挂点 */
  display: flex;
  align-items: center;
  gap: var(--space-6);
  padding: var(--space-8) var(--space-12);
  border-radius: var(--radius-6);
  background: var(--c1);
  box-shadow: inset 0 0 0 1px var(--line-hover);
}
/* 聚焦：品牌色 1.5px 边框淡入（::after 状态层 opacity 过渡，踩坑 036） */
.app-search::after {
  content: "";
  position: absolute;
  inset: 0;
  border: var(--border-strong) solid var(--brand);
  border-radius: var(--radius-6);
  pointer-events: none;
  opacity: 0;
  transition: opacity 0.2s ease;
}
.app-search:focus-within::after {
  opacity: 1;
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
