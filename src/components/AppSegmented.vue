<script setup lang="ts">
// 切换按钮/分段控件（figma-css 01_按钮 2.4 小 / 2.5 大）：
// 轨道 C1 底 + 暗-悬停边框 radius 6，padding 2 gap 4；
// 选中块 C4 底 + 暗-悬停边框 radius 4（token：切换按钮内按钮圆角 4），文字常态档；未选中文字标签档，hover C3；
// 选中块滑动位移（00_要求：切换按钮要有位移动画）。
import { computed } from "vue";

const model = defineModel<string>({ required: true });
const props = withDefaults(
  defineProps<{ options: readonly { value: string; label: string }[]; small?: boolean; disabled?: boolean }>(),
  { small: false, disabled: false },
);

// 禁用态（3.5：排序"自定义"时升降切换禁用）：整体降档 + 不响应，选中滑块保留当前位
function pick(value: string): void {
  if (props.disabled) return;
  model.value = value;
}

const index = computed(() => Math.max(0, props.options.findIndex((o) => o.value === model.value)));
const n = computed(() => Math.max(1, props.options.length));
// 滑块宽度 = (100% - 两侧padding 4px - 间隙 (n-1)*4px) / n；位移 = index × (块宽 + 4px 间隙)
const thumbStyle = computed(() => ({
  width: `calc((100% - 4px - ${(n.value - 1) * 4}px) / ${n.value})`,
  transform: `translateX(calc(${index.value} * (100% + 4px)))`,
}));
</script>

<template>
  <div class="app-seg" :class="{ small, disabled }" role="tablist">
    <span class="thumb" :style="thumbStyle" />
    <button
      v-for="o in options"
      :key="o.value"
      class="item"
      :class="{ on: o.value === model }"
      role="tab"
      :disabled="disabled"
      @click="pick(o.value)"
    >
      {{ o.label }}
    </button>
  </div>
</template>

<style scoped>
.app-seg {
  position: relative;
  display: flex;
  align-items: stretch;
  padding: var(--space-2);
  gap: var(--space-4);
  border-radius: var(--radius-6);
  background: var(--c1);
  box-shadow: inset 0 0 0 1px var(--line-hover);
  flex: none;
}
.thumb {
  position: absolute;
  top: var(--space-2);
  bottom: var(--space-2);
  left: var(--space-2);
  border-radius: var(--radius-4);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--line-hover);
  transition: transform 0.2s ease;
  pointer-events: none;
}
.item {
  position: relative;
  flex: 1;
  min-width: 0; /* 允许缩到 min-content 以下（1.4 走查：nowrap 文字把 item 撑到 52 > 均分 48，
                   容器溢出且 thumb 仍按公式 48 计算 → 选中块与文字错位，2026-08-11 CDP 实测定位） */
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--space-6) var(--space-8);
  border-radius: var(--radius-4);
  color: var(--text-label);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  white-space: nowrap; /* 防换行（1.4 查看方式"自适应"三字在 48 宽稿值内换行，2026-08-11 走查） */
  transition:
    color 0.15s ease,
    background 0.15s ease;
}
.app-seg.small .item {
  padding: var(--space-4) var(--space-8);
  font-size: var(--text-label-size);
  line-height: var(--text-label-line);
}
/* 禁用态：整体降档不响应（hover 不再出 C3） */
.app-seg.disabled {
  opacity: 0.5;
}
.app-seg.disabled .item {
  cursor: default;
}
.app-seg.disabled .item:hover {
  background: transparent;
}
.item:hover {
  background: var(--c3);
}
.item.on {
  color: var(--text-normal);
  background: transparent;
}
</style>
