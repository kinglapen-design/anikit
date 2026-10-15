<script setup lang="ts">
// MarkDown 大纲栏（2026-10-15 大纲轮，PRD 04_文件预览「md 大纲栏」；交互参照 = 飞书文档大纲）：
// 纯展示件——标题清单 / 当前节行号由 PreviewText 计算喂入，点击抛 jump(line)；样式定稿口径见速查 §71：
// 宽默认 168（设置面板左导航同宽，§70）**右缘可拖拽调宽 120-360**（分屏分割条同口径：5px 命中区 + 1px g3 线
// hover/拖拽品牌色，§22 拖拽线同色系；宽经 v-model:width 外流会话级保持）；item 26 高 / 圆角 6 / padding 4 8 /
// 正文 13/18 l2（菜单 item 同档，§3），缩进 12/级，单行省略——**截断才出悬停 tip**（实测 scrollWidth > clientWidth，
// v-tip 全文；未截断不扰）；hover 底 l-a3（菜单 item hover 同档）；当前节 cur = 底 l-a2 + 文字 l1（设置面板导航
// 选中同口径，选中态不叠 hover，§3）；**一级标题加大加粗**（2026-10-15 用户看稿定稿：fs-md-body 15 + fw-bold）；
// 空态「本文档没有标题」l3；底 66 悬浮栏遮挡补偿（§18）；滚动 = kit-scroll + ScrollThumb（§2）。
// forceHoverLine = preview 陈列演示态桥接。
import { nextTick, onMounted, ref, watch } from "vue";
import ScrollThumb from "./ScrollThumb.vue";
import type { MdOutlineItem } from "../lib/mdOutline";

const props = withDefaults(
  defineProps<{ items: MdOutlineItem[]; activeLine?: number | null; forceHoverLine?: number | null }>(),
  { activeLine: null, forceHoverLine: null },
);
const emit = defineEmits<{ jump: [line: number] }>();
// 栏宽（右缘拖拽，PreviewText 会话级保持；缺省 168 = §70 同宽）
const width = defineModel<number>("width", { default: 168 });

const scrollRef = ref<HTMLElement>();

// 截断实测（悬停 tip 仅截断时出）：items / 栏宽变化后重测——scrollWidth > clientWidth + 1 即截断
const truncatedLines = ref<Set<number>>(new Set());
function measureTruncated(): void {
  const root = scrollRef.value;
  if (!root) return;
  const s = new Set<number>();
  for (const el of root.querySelectorAll<HTMLElement>(".mo-item[data-line]")) {
    if (el.scrollWidth > el.clientWidth + 1) s.add(Number(el.dataset.line));
  }
  truncatedLines.value = s;
}
watch([() => props.items, width], () => void nextTick(measureTruncated));
onMounted(() => void nextTick(measureTruncated));

// 栏宽拖拽（§9 ③ 窗口监听口径；clamp 120-360）
function onWidthDrag(e: PointerEvent): void {
  e.preventDefault();
  const bar = e.currentTarget as HTMLElement;
  bar.classList.add("drag");
  const startX = e.clientX;
  const startW = width.value;
  const move = (ev: PointerEvent): void => {
    width.value = Math.min(360, Math.max(120, startW + ev.clientX - startX));
  };
  const up = (): void => {
    bar.classList.remove("drag");
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", up);
  };
  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", up);
}

// 当前节滚进视野（正文滚动联动高亮时，防高亮条目被甩出大纲视口）
watch(
  () => props.activeLine,
  async () => {
    if (props.activeLine == null) return;
    await nextTick();
    const root = scrollRef.value;
    const el = root?.querySelector<HTMLElement>(".mo-item.cur");
    if (!root || !el) return;
    const r = root.getBoundingClientRect();
    const er = el.getBoundingClientRect();
    if (er.top < r.top) root.scrollTop += er.top - r.top;
    else if (er.bottom > r.bottom) root.scrollTop += er.bottom - r.bottom;
  },
);
</script>

<template>
  <aside class="md-outline" :style="{ width: `${width}px` }">
    <div ref="scrollRef" class="mo-scroll kit-scroll">
      <template v-if="items.length">
        <button
          v-for="h in items"
          :key="h.line"
          type="button"
          class="mo-item"
          :class="{ cur: h.line === activeLine, l1: h.level === 1, 'force-hover': h.line === forceHoverLine }"
          :style="{ paddingLeft: `${8 + (h.level - 1) * 12}px` }"
          :data-line="h.line"
          v-tip="truncatedLines.has(h.line) ? h.text : undefined"
          @click="emit('jump', h.line)"
        >
          {{ h.text }}
        </button>
      </template>
      <p v-else class="mo-empty">本文档没有标题</p>
    </div>
    <ScrollThumb :body="scrollRef" />
    <!-- 栏宽拖拽缝（分屏分割条同口径；z 12 压 ScrollThumb 的 z 10——缝线 2px 重叠区指针归拖拽） -->
    <div class="mo-drag" @pointerdown="onWidthDrag" />
  </aside>
</template>

<style scoped>
.md-outline {
  position: relative; /* ScrollThumb / 拖拽缝锚（§2：relative 包裹层 + 滚动容器 + 悬浮件） */
  flex: none;
  min-height: 0;
  display: flex;
  /* 进场动画（稿无数值就近套档：150ms 缓出 + 8px 滑移，速查 §71；v-if 关闭瞬时——出向无可过渡挂载点） */
  animation: mo-in var(--n-dur-150) var(--n-ease-out);
}
@keyframes mo-in {
  from {
    opacity: 0;
    transform: translateX(-8px);
  }
}
/* 栏宽拖拽缝（分屏分割条同口径：5px 命中区跨缘 + 1px g3 线，hover / 拖拽 = 品牌色，§22 拖拽线同色系） */
.mo-drag {
  position: absolute;
  right: -2px;
  top: 0;
  bottom: 0;
  width: 5px;
  cursor: col-resize;
  z-index: 12; /* 压 ScrollThumb（z 10）——缝线重叠区指针归拖拽 */
}
.mo-drag::before {
  content: "";
  position: absolute;
  left: 2px;
  top: 0;
  bottom: 0;
  width: 1px;
  background: var(--n-g3);
}
.mo-drag:hover::before,
.mo-drag.drag::before {
  background: var(--n-brand);
}
.mo-scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: var(--n-space-12) var(--n-space-8) 66px; /* 底 66 = 悬浮栏遮挡补偿（§18 同口径） */
  display: flex;
  flex-direction: column;
  gap: 2px; /* 菜单组内 item 间距同档 */
}
.mo-item {
  flex: none;
  display: block;
  width: 100%;
  height: 26px;
  padding: var(--n-space-4) var(--n-space-8);
  border-radius: var(--n-radius-6);
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l2);
  text-align: left;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  transition:
    background var(--n-dur-150) var(--n-ease-in-out),
    color var(--n-dur-150) var(--n-ease-in-out);
}
.mo-item:hover,
.mo-item.force-hover {
  background: var(--n-l-a3); /* 菜单 item hover 同档 */
}
.mo-item.cur {
  background: var(--n-l-a2); /* 设置面板导航选中 = a2 底 + l1 文字（§70 同口径） */
  color: var(--n-l1);
}
.mo-item.cur:hover {
  background: var(--n-l-a2); /* 选中态行 hover 不再变色（§3） */
}
/* 一级标题加大加粗（2026-10-15 用户看稿定稿）：fs-md-body 15 + fw-bold，行高行宽不变 */
.mo-item.l1 {
  font-size: var(--n-fs-md-body);
  font-weight: var(--n-fw-bold);
}
.mo-empty {
  padding: var(--n-space-12) var(--n-space-8);
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  color: var(--n-l3);
}
</style>
