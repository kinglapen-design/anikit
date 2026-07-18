<script setup lang="ts">
// 图标按钮（token 07）：无底色装饰（默认）/ 有底色装饰（filled）。
// 大 = 28×28（内边距 6），小 = 24×24（内边距 4）。
import AppIcon from "./AppIcon.vue";

withDefaults(
  defineProps<{ name: string; small?: boolean; filled?: boolean; disabled?: boolean; title?: string }>(),
  { small: false, filled: false, disabled: false, title: undefined },
);
defineEmits<{ click: [e: MouseEvent] }>();
</script>

<template>
  <button
    class="icon-btn"
    :class="{ small, filled }"
    :disabled="disabled"
    :title="title"
    @click="$emit('click', $event)"
  >
    <AppIcon :name="name" />
  </button>
</template>

<style scoped>
.icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  padding: var(--space-6);
  border-radius: var(--radius-6);
  color: var(--c4);
  flex: none;
  transition: background 0.12s ease;
}
.icon-btn.small {
  width: 24px;
  height: 24px;
  padding: var(--space-4);
}
.icon-btn:hover {
  background: var(--c4-7);
}
.icon-btn.filled {
  background: var(--c4-7);
}
.icon-btn.filled:hover {
  background: var(--c4-15);
}
.icon-btn:disabled {
  cursor: default;
  background: none;
  opacity: 0.5;
}
</style>
