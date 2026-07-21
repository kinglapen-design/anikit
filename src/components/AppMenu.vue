<script setup lang="ts">
// 菜单容器（02_通用组件/07_右键菜单、03_UI 布局/05_查看方式）：
// C5 / 圆角 8 / 1px C4-15（inset）/ 内边距 4 / 子项间距 2，fixed 定位，外部点击 / Esc 关闭。
import { onBeforeUnmount, onMounted, ref } from "vue";

withDefaults(defineProps<{ x: number; y: number; width?: number }>(), { width: 175 });
const emit = defineEmits<{ close: [] }>();
const rootRef = ref<HTMLElement>();

function onPointerDown(e: PointerEvent): void {
  if (rootRef.value && !rootRef.value.contains(e.target as Node)) emit("close");
}
function onKeydown(e: KeyboardEvent): void {
  if (e.key === "Escape") emit("close");
}

onMounted(() => {
  window.addEventListener("pointerdown", onPointerDown, true);
  window.addEventListener("keydown", onKeydown, true);
});
onBeforeUnmount(() => {
  window.removeEventListener("pointerdown", onPointerDown, true);
  window.removeEventListener("keydown", onKeydown, true);
});
</script>

<template>
  <div
    ref="rootRef"
    class="app-menu"
    :style="{ left: `${x}px`, top: `${y}px`, width: `${width}px` }"
    @pointerdown.stop
  >
    <slot />
  </div>
</template>

<style scoped>
.app-menu {
  position: fixed;
  z-index: 2000;
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  padding: var(--space-4);
  border-radius: var(--radius-8);
  background: var(--c5);
  box-shadow: inset 0 0 0 1px var(--c4-15);
}
</style>
