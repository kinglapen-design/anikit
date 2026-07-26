<script setup lang="ts">
// 图标按钮（02_通用组件/01_按钮）：无底色装饰（默认）/ 有底色装饰（filled）。
// 大 = 28×28（内边距 6），小 = 24×24（内边距 4）；danger 用于窗口控制关闭（hover 错误色）。
import AppIcon from "./AppIcon.vue";

withDefaults(
  defineProps<{ name: string; small?: boolean; filled?: boolean; danger?: boolean; active?: boolean; disabled?: boolean; title?: string }>(),
  { small: false, filled: false, danger: false, active: false, disabled: false, title: undefined },
);
defineEmits<{ click: [e: MouseEvent] }>();
</script>

<template>
  <button
    class="icon-btn"
    :class="{ small, filled, danger, active }"
    :disabled="disabled"
    v-tip="title"
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
  color: var(--font-icon-1);
  flex: none;
  transition: background 0.12s ease;
}
/* 图标按钮-小（24×24）：圆角 4 */
.icon-btn.small {
  width: 24px;
  height: 24px;
  padding: var(--space-4);
  border-radius: var(--radius-4);
}
.icon-btn:hover {
  background: var(--font-icon-6);
}
.icon-btn.filled {
  background: var(--c5);
}
.icon-btn.filled:hover {
  background: var(--border-1);
}
.icon-btn.danger:hover {
  background: var(--error);
}
/* 激活态（5.2：笔记浮层开着 / 循环播放开启）：品牌紫 */
.icon-btn.active {
  color: var(--brand-1);
}
.icon-btn:disabled {
  cursor: default;
  background: none;
  opacity: 0.5;
}
</style>
