<script setup lang="ts">
// 菜单容器（7.1 二十四轮 2026-09-01 按新稿 组件/上下文菜单 容器定案替换，旧 figma-css 04 稿口径废除）：
// 通用/三级 #242424 底（--n-g3）/ 圆角 8 / 1px 通用/五级 #2E2E2E 描边（inset 内画不占空间）
// / 小阴影 --n-shadow-sm；内边距 6 / gap 0
//（分组间距由 ContextMenu 组容器承担：组内 gap 2 / 组 padding 上下 4 首末 0 / 组间 1px 线）。
// 宽度（2026-09-01 二十五轮用户定，废 300 定宽——定宽致短内容右侧大片空白）：
// 不传 width 时内容自适应（fit-content，容器宽由最长 item 决定；item 内 名称→36px→右件 最小间距
// 由 MenuItem 承担），视口宽封顶 100vw-8（超长 label 回 ellipsis）；width 仍可按面板形态定宽
//（ViewOptionsMenu 404 面板），pad/gap 亦可覆盖。
// fixed 定位，点击组外（group 之外）或 Esc 关闭；菜单项点击不自动关闭（连续操作）。
// 边界钳制（1.3，03 稿注"菜单不可超出窗口边缘"）：挂载/位移后量实际尺寸，右/下缘溢出回收进窗口
//（留 4 呼吸），弹出动画 origin 随回收方位翻转；超出窗口高度时封顶 100vh-8 并纵向滚动。
// 尺寸用 offsetWidth/offsetHeight（布局尺寸）——getBoundingClientRect 会带上 menu-pop 的 scale(0.96)
// 动画初帧，底部溢出被低估 4% 导致钳制失效（2026-08-11 走查）。
// pop = false 关闭弹出动画（级联 flyout 每次 hover 换父项重建，回放动画会闪）。
// width/pad/gap 仍可按面板形态覆盖（ViewOptionsMenu 404 面板、FolderIconGrid 网格面板）。
// tone（2026-09-05 走查：linkList 二级复用悬停 tip 观感）= "menu" 默认（g3 实底 + inset g5 描边 +
// 圆角 8）/ "tip"（悬停-tip 壳：通用/半透明-1 底 + 1px 亮色/半透明-3 真 border + blur 小 + 圆角 6，
// 配 pad="0" 行贴边——linkList 面板用，与卡片徽章 hover 弹窗 LinkListPopover 同壳）。
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";

const props = withDefaults(
  defineProps<{ x: number; y: number; width?: number; pad?: string; gap?: string; group?: HTMLElement | null; trigger?: HTMLElement | null; pop?: boolean; tone?: "menu" | "tip" }>(),
  { width: undefined, pad: "var(--n-space-6)", gap: "0", group: null, trigger: null, pop: true, tone: "menu" },
);
const emit = defineEmits<{ close: [] }>();
const rootRef = ref<HTMLElement>();
// flyout 等调用方需渲染后实测宽度（内容自适应场景）：暴露根元素（ContextMenu 左飞修正用）
defineExpose({ root: rootRef });
// 实际定位（钳制后可能偏移）与动画原点（默认左上）
const pos = ref({ left: props.x, top: props.y });
const origin = ref("top left");

function clampPosition(): void {
  const el = rootRef.value;
  if (!el) return;
  // 复位上次钳制留下的内联样式，量原始尺寸
  el.style.maxHeight = "";
  el.style.overflowY = "";
  const w = el.offsetWidth;
  const h = el.offsetHeight;
  let { left, top } = pos.value;
  let ox = "left";
  let oy = "top";
  if (h > window.innerHeight - 8) {
    // 高过窗口：封顶 + 滚动，钉顶
    el.style.maxHeight = `${window.innerHeight - 8}px`;
    el.style.overflowY = "auto";
    top = 4;
  } else if (top + h > window.innerHeight - 4) {
    top = Math.max(4, window.innerHeight - 4 - h);
    oy = "bottom";
  }
  if (left + w > window.innerWidth - 4) {
    left = Math.max(4, window.innerWidth - 4 - w);
    ox = "right";
  }
  pos.value = { left, top };
  origin.value = `${oy} ${ox}`;
}

// x/y props 变化（菜单未关又在别处弹出）：复位定位重新钳制
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
    :class="{ pop, tip: tone === 'tip' }"
    :style="{ left: `${pos.left}px`, top: `${pos.top}px`, width: width ? `${width}px` : undefined, padding: pad, gap, transformOrigin: origin }"
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
  width: fit-content; /* 内容自适应（最长 item 决定）；内联 width 覆盖（定宽面板） */
  max-width: calc(100vw - 8px); /* 视口封顶：超长 label 回 ellipsis，不撑出窗口 */
  border-radius: var(--n-radius-8);
  background: var(--n-g3);
  box-shadow:
    inset 0 0 0 1px var(--n-g5),
    var(--n-shadow-sm);
}
/* tip 壳（linkList 二级 = 悬停-tip 观感，与徽章 hover 弹窗 LinkListPopover 同口径）：
   overflow hidden = 行 hover 直角底裁进圆角（2026-09-05 走查抓出溢出线） */
.app-menu.tip {
  border: 1px solid var(--n-l-a3);
  border-radius: var(--n-radius-6);
  background: var(--n-g-a1);
  box-shadow: var(--n-shadow-sm);
  backdrop-filter: blur(var(--n-blur-sm));
  overflow: hidden;
}
/* 弹出动画：快速缩放 + 透明度过渡，缓出曲线（2026-07-24 用户定）；flyout 等逐帧位移场景用 pop=false 关闭 */
.app-menu.pop {
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
