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
// 超高滚动的滚轮 = ScrollThumb（2026-09-13 实机走查补，贴菜单右缘）——fixed 定位上移到
// .app-menu-anchor 锚点包裹层，滚轮与滚动容器平级（放容器内会随内容滚动漂移）；
// 消费方 class 透传随模板根落包裹层（PluginListPanel 壳覆盖随之改 :deep 口径）。
// 尺寸用 offsetWidth/offsetHeight（布局尺寸）——getBoundingClientRect 会带上 menu-pop 的 scale(0.96)
// 动画初帧，底部溢出被低估 4% 导致钳制失效（2026-08-11 走查）。
// pop = false 关闭弹出动画（级联 flyout 每次 hover 换父项重建，回放动画会闪）。
// width/pad/gap 仍可按面板形态覆盖（ViewOptionsMenu 404 面板、FolderIconGrid 网格面板）。
// tone（2026-09-05 走查：linkList 二级复用悬停 tip 观感）= "menu" 默认（g3 实底 + inset g5 描边 +
// 圆角 8）/ "tip"（悬停-tip 壳：通用/半透明-1 底 + 1px 亮色/半透明-3 真 border + blur 小 + 圆角 6，
// 配 pad="0" 行贴边——linkList 面板用，与卡片徽章 hover 弹窗 LinkListPopover 同壳）。
// group（2026-09-05 菜单栏主菜单多级级联）支持数组：各级 flyout 根元素全列为"组内"，点任一级不关菜单。
// minW（2026-09-05 用户实机定）：一级条目菜单最小宽 250（内容自适应太窄显得轻）——仅根级条目菜单传，
// flyout 与面板形态（iconPicker 186 等稿值定宽）不传保持自适应。
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import ScrollThumb from "./ScrollThumb.vue";

const props = withDefaults(
  defineProps<{ x: number; y: number; width?: number; minW?: number; pad?: string; gap?: string; group?: HTMLElement | (HTMLElement | null | undefined)[] | null; trigger?: HTMLElement | null; pop?: boolean; tone?: "menu" | "tip" }>(),
  { width: undefined, minW: undefined, pad: "var(--n-space-6)", gap: "0", group: null, trigger: null, pop: true, tone: "menu" },
);
const emit = defineEmits<{ close: [] }>();
const rootRef = ref<HTMLElement>();
const wrapRef = ref<HTMLElement>(); // 锚点包裹层（ScrollThumb 定位锚 + 组外判定含滚轮）
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
  // 锚点包裹层内含菜单本体与悬浮滚轮（拖滚轮不关菜单，2026-09-13 实机走查）
  if (wrapRef.value?.contains(t)) return;
  // 组内元素（级联 flyout 等）不算外部：group 支持单元素或数组（多级 flyout 各级根元素，2026-09-05）
  const groups = Array.isArray(props.group) ? props.group : [props.group];
  for (const g of groups) {
    if (g && g !== rootRef.value && g.contains(t)) return;
    // 组内菜单的悬浮滚轮也不算组外：滚轮在该菜单锚点包裹层内（.app-menu 外、flyout 根元素外），
    // 按其所属锚点是否包住组内元素判定（2026-09-13——拖 flyout 滚轮误关整组修复）
    if (g && t instanceof Element && t.closest(".app-menu-anchor")?.contains(g)) return;
  }
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
  <!-- 锚点包裹层（2026-09-13 实机走查：超高菜单补悬浮滚轮）：fixed 定位/z-index 上移本层，
       shrink-wrap 菜单本体并兼 ScrollThumb 定位锚——thumb 必须与滚动容器（.app-menu）平级，
       放进滚动容器内会随内容滚动漂移（absolute 子元素随 scroller 滚动 + offsetTop 含视口偏移） -->
  <div
    ref="wrapRef"
    class="app-menu-anchor"
    :style="{ left: `${pos.left}px`, top: `${pos.top}px` }"
  >
    <div
      ref="rootRef"
      class="app-menu"
      :class="{ pop, tip: tone === 'tip' }"
      :style="{ width: width ? `${width}px` : undefined, minWidth: minW ? `${minW}px` : undefined, padding: pad, gap, transformOrigin: origin }"
      @pointerdown.stop
    >
      <slot />
    </div>
    <!-- 超高菜单（clampPosition 给 .app-menu 设 overflowY=auto）的悬浮滚轮：贴菜单右缘，
         盖右缘 6px padding 区可接受（定案口径，不做负 margin）；.kit-scroll 不挂（overflow 由 JS 控制） -->
    <ScrollThumb :body="rootRef" />
  </div>
</template>

<style scoped>
/* 锚点包裹层：fixed 定位与 z-index 自 .app-menu 上移本层（2026-09-13 实机走查加滚轮）；
   无宽 → shrink-wrap 菜单本体，滚轮 right:0 即菜单右缘 */
.app-menu-anchor {
  position: fixed;
  z-index: 2000;
}
.app-menu {
  position: relative; /* 承接原 fixed 的包含块语义（内部绝对定位后代锚点不变） */
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
