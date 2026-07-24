<script setup lang="ts">
// 菜单容器（02_通用组件/07_右键菜单、03_UI 布局/05_查看方式）：
// C3 / 圆角 8 / 1px border-1（inset）/ 内边距 8（2026-07-23 用户定）/ 子项间距 2 / 投影 --shadow-pop；
// fixed 定位，点击组外（group 之外）或 Esc 关闭；菜单项点击不自动关闭（连续操作）。
import { onBeforeUnmount, onMounted, ref } from "vue";

const props = withDefaults(
  defineProps<{ x: number; y: number; width?: number; pad?: string; gap?: string; group?: HTMLElement | null; trigger?: HTMLElement | null }>(),
  { width: 175, pad: "var(--space-8)", gap: "var(--space-2)", group: null, trigger: null },
);
const emit = defineEmits<{ close: [] }>();
const rootRef = ref<HTMLElement>();

function onPointerDown(e: PointerEvent): void {
  const t = e.target as Node;
  // 触发按钮本身不算组外（toggle 由按钮的 click 处理，避免"组外先关、click 又开"反复弹）
  if (props.trigger?.contains(t)) return;
  if (rootRef.value?.contains(t)) return;
  if (props.group && props.group !== rootRef.value && props.group.contains(t)) return;
  emit("close");
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
    :style="{ left: `${x}px`, top: `${y}px`, width: `${width}px`, padding: pad, gap }"
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
  border-radius: var(--radius-8);
  background: var(--c3);
  box-shadow:
    inset 0 0 0 1px var(--border-1),
    var(--shadow-pop);
  /* 弹出动画：快速缩放 + 透明度过渡，缓出曲线（2026-07-24 用户定） */
  animation: menu-pop 0.14s cubic-bezier(0.16, 1, 0.3, 1);
  transform-origin: top left;
}
@keyframes menu-pop {
  from {
    opacity: 0;
    transform: scale(0.96);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}
</style>
