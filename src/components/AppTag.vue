<script setup lang="ts">
// 属性标签（figma-css 01_按钮 2.3）：C4 底 + 暗-悬停边框 radius 6，padding 4/8 gap 6，标签字 常态档；
// 常态隐藏删除按钮，hover：background C5 / 删除按钮右至左滑入（clear 12px 标签档）。
import AppIcon from "./AppIcon.vue";

defineProps<{ label: string }>();
defineEmits<{ remove: [] }>();
</script>

<template>
  <span class="app-tag">
    <span class="label">{{ label }}</span>
    <button class="remove" v-tip="'删除'" @click.stop="$emit('remove')">
      <AppIcon name="clear" :size="12" />
    </button>
  </span>
</template>

<style scoped>
.app-tag {
  display: inline-flex;
  align-items: center;
  padding: var(--space-4) var(--space-8);
  border-radius: var(--radius-6);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--line-hover);
  color: var(--text-normal);
  font-size: var(--text-label-size);
  line-height: var(--text-label-line);
  flex: none;
  transition: background 0.15s ease;
}
.app-tag:hover {
  background: var(--c5);
}
/* 删除按钮右至左滑入：宽度 0→18px（12 图标 + 6 间隙） */
.remove {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 0;
  margin-left: 0;
  overflow: hidden;
  color: var(--text-label);
  flex: none;
  transition:
    width 0.15s ease,
    margin-left 0.15s ease;
}
.app-tag:hover .remove {
  width: 12px;
  margin-left: var(--space-6);
}
</style>
