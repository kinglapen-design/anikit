<script setup lang="ts">
// 按钮（figma-css 01_按钮 1.6~1.8 图标+标题按钮 / 2.1~2.2 标题按钮）：
// variant：primary = 重要（品牌色底 + 亮-选中边框 + 高亮标题）/ panel = 二级窗口（C4 底，默认）/ sidebar = 侧边栏（C5 底）；
// size：large（padding 8/12）/ small（padding 4/8，带图标时 6/8 + 图标 12 + 标签字）；
// 字档规则（figma）：大且无图标 = 标题档 700；小且有图标 = 标签档；其余 = 正文档；
// hover：panel/primary 叠 暗-悬停（panel 实为 C5，按 figma 2.1/2.2 hover background C5）、sidebar 叠 暗-悬停，标题/图标轻微下移（00_要求：区分分块与按钮）。
import { computed } from "vue";
import AppIcon from "./AppIcon.vue";

const props = withDefaults(
  defineProps<{
    label: string;
    icon?: string;
    variant?: "primary" | "panel" | "sidebar";
    size?: "large" | "small";
    disabled?: boolean;
    title?: string;
  }>(),
  { icon: undefined, variant: "panel", size: "large", disabled: false, title: undefined },
);
defineEmits<{ click: [e: MouseEvent] }>();

const iconSize = computed(() => (props.size === "small" ? 12 : 18));
// 字档：大+无图标 = 标题档；小+有图标 = 标签档；其余正文档
const fontTier = computed(() => {
  if (props.size === "large" && !props.icon) return "title";
  if (props.size === "small" && props.icon) return "label";
  return "body";
});
</script>

<template>
  <button
    class="app-btn"
    :class="[variant, size, `font-${fontTier}`]"
    :disabled="disabled"
    v-tip="title"
    @click="$emit('click', $event)"
  >
    <span class="inner">
      <AppIcon v-if="icon" :name="icon" :size="iconSize" />
      <span class="label">{{ label }}</span>
    </span>
  </button>
</template>

<style scoped>
.app-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-6);
  flex: none;
  transition: background 0.15s ease;
}
.app-btn.large {
  padding: var(--space-8) var(--space-12);
}
.app-btn.small {
  padding: var(--space-4) var(--space-8);
}
.inner {
  display: inline-flex;
  align-items: center;
  gap: var(--space-6);
  transition: transform 0.15s ease;
}
/* hover：标题/图标轻微下移（00_要求：与分块区分） */
.app-btn:hover .inner {
  transform: translateY(1px);
}
/* 字档 */
.font-title {
  font-size: var(--text-title-size);
  line-height: var(--text-title-line);
  font-weight: var(--text-title-weight);
}
.font-body {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
}
.font-label {
  font-size: var(--text-label-size);
  line-height: var(--text-label-line);
}
/* 变体 */
.app-btn.panel {
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--line-hover);
  color: var(--text-normal);
}
.app-btn.panel:hover {
  background: var(--c5);
}
.app-btn.sidebar {
  background: var(--c5);
  box-shadow: inset 0 0 0 1px var(--line-hover);
  color: var(--text-normal);
}
.app-btn.sidebar:hover {
  background: linear-gradient(var(--line-hover), var(--line-hover)), var(--c5);
}
.app-btn.primary {
  background: var(--brand);
  box-shadow: inset 0 0 0 1px var(--line-active);
  color: var(--text-highlight);
}
.app-btn.primary:hover {
  background: linear-gradient(var(--line-hover), var(--line-hover)), var(--brand);
}
.app-btn:disabled {
  cursor: default;
  opacity: 0.5;
}
.app-btn:disabled .inner {
  transform: none;
}
</style>
