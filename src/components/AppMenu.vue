<script setup lang="ts">
// 菜单容器（figma-css 04_右键菜单 + 布局稿菜单共性）：C2 底 / 圆角 8 / 1.5px 亮-选中边框（inset）/
// 内边距 12（布局稿菜单容器统一）/ 子项间距 2 / 投影 --shadow-pop；
// fixed 定位，点击组外（group 之外）或 Esc 关闭；菜单项点击不自动关闭（连续操作）。
// 边界钳制（1.3，03 稿注"菜单不可超出窗口边缘"）：挂载后量实际尺寸，右/下缘溢出回收进窗口
//（留 4 呼吸），弹出动画 origin 随回收方位翻转；超出窗口高度时封顶 100vh-8 并纵向滚动。
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";

const props = withDefaults(
  defineProps<{ x: number; y: number; width?: number; pad?: string; gap?: string; group?: HTMLElement | null; trigger?: HTMLElement | null }>(),
  { width: 175, pad: "var(--space-12)", gap: "var(--space-2)", group: null, trigger: null },
);
const emit = defineEmits<{ close: [] }>();
const rootRef = ref<HTMLElement>();
// 实际定位（钳制后可能偏移）与动画原点（默认左上）
const pos = ref({ left: props.x, top: props.y });
const origin = ref("top left");

function clampPosition(): void {
  const el = rootRef.value;
  if (!el) return;
  // 复位上次钳制留下的内联样式，量原始尺寸
  el.style.maxHeight = "";
  el.style.overflowY = "";
  const r = el.getBoundingClientRect();
  let { left, top } = pos.value;
  let ox = "left";
  let oy = "top";
  if (r.height > window.innerHeight - 8) {
    // 高过窗口：封顶 + 滚动，钉顶
    el.style.maxHeight = `${window.innerHeight - 8}px`;
    el.style.overflowY = "auto";
    top = 4;
  } else if (r.bottom > window.innerHeight - 4) {
    top = Math.max(4, top - (r.bottom - (window.innerHeight - 4)));
    oy = "bottom";
  }
  if (r.right > window.innerWidth - 4) {
    left = Math.max(4, left - (r.right - (window.innerWidth - 4)));
    ox = "right";
  }
  pos.value = { left, top };
  origin.value = `${oy} ${ox}`;
}

// x/y props 变化（菜单未关又在别处弹出 / flyout 换父项）：复位定位重新钳制
watch(
  () => [props.x, props.y],
  ([x, y]) => {
    pos.value = { left: x, top: y };
    void nextTick(clampPosition);
  },
);

function onPointerDown(e: PointerEvent): void {
  const t = e.target as Node;
  // 触发按钮本身不算组外（toggle 由按钮的 click 处理，避免"组外先关、click 又开"反复弹）
  if (props.trigger?.contains(t)) return;
  if (rootRef.value?.contains(t)) return;
  if (props.group && props.group !== rootRef.value && props.group.contains(t)) return;
  emit("close");
}
function onKeydown(e: KeyboardEvent): void {
  // 菜单内输入框（1.3 搜索框）的 Esc 豁免：由输入框自己处理（先清空、空再关）
  const t = e.target as HTMLElement;
  if (e.key === "Escape" && t.tagName === "INPUT" && rootRef.value?.contains(t)) return;
  if (e.key === "Escape") emit("close");
}

onMounted(() => {
  window.addEventListener("pointerdown", onPointerDown, true);
  window.addEventListener("keydown", onKeydown, true);
  void nextTick(clampPosition);
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
    :style="{ left: `${pos.left}px`, top: `${pos.top}px`, width: `${width}px`, padding: pad, gap, transformOrigin: origin }"
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
  background: var(--c2);
  box-shadow:
    inset 0 0 0 var(--border-strong) var(--line-active),
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
