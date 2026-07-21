<script setup lang="ts">
// 菜单项（02_通用组件/02_item 03 右键菜单 item）：内边距 4 6 / 圆角 6 / hover C4-7。
// 一级项：图标 + 标题 + 快捷键标签或二级菜单提示右箭头（8×8）；
// 二级项（checkable）：8×8 对勾占位（选中显示，未选中隐藏但占位保证标题列对齐）。
import AppIcon from "./AppIcon.vue";

withDefaults(
  defineProps<{
    icon?: string;
    label: string;
    shortcut?: string;
    arrow?: boolean;
    checkable?: boolean;
    checked?: boolean;
  }>(),
  { icon: undefined, shortcut: undefined, arrow: false, checkable: false, checked: false },
);
const emit = defineEmits<{ select: [] }>();
</script>

<template>
  <button class="menu-item" @click="emit('select')">
    <span class="main">
      <span v-if="checkable" class="check-slot">
        <svg v-if="checked" width="8" height="8" viewBox="0 0 8 8">
          <path
            d="M1.5 4.3 3.2 6 6.5 2"
            stroke="currentColor"
            stroke-width="1"
            fill="none"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
      </span>
      <AppIcon v-else-if="icon" :name="icon" class="deco" />
      <span class="label">{{ label }}</span>
    </span>
    <span v-if="shortcut" class="shortcut">{{ shortcut }}</span>
    <AppIcon v-if="arrow" name="submenu-arrow" :size="8" class="arrow" />
  </button>
</template>

<style scoped>
.menu-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-16);
  width: 100%;
  padding: var(--space-4) var(--space-6);
  border-radius: var(--radius-6);
  color: var(--c4);
  transition: background 0.12s ease;
}
.menu-item:hover {
  background: var(--c4-7);
}
/* 基础 item：图标 + 标题，间距 6 */
.main {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  flex: 1;
  min-width: 0;
}
.deco {
  flex: none;
}
.check-slot {
  width: 8px;
  height: 8px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--c4);
  flex: none;
}
.label {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.shortcut {
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--c4-40);
  flex: none;
}
.arrow {
  flex: none;
}
</style>
