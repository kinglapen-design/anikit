<script setup lang="ts">
// 文件树 item（02_通用组件/02_item 02）：高 28；展开按钮容器宽 9（= 图标宽，高随 item），
// 无子项时保留占位保证装饰图标列对齐。
// 层级缩进 20px：子项展开箭头尖角（缩进 + 4.5）对齐父级装饰图标中心（缩进 + 25），
// 即缩进 = 9/2 + gap 2 + padding 6 + 图标 16/2 = 20.5 ≈ 20。
import AppIcon from "./AppIcon.vue";

withDefaults(
  defineProps<{
    icon: string;
    label: string;
    count?: number;
    depth?: number;
    expandable?: boolean;
    open?: boolean;
    selected?: boolean;
    last?: boolean;
    guide?: boolean;
  }>(),
  { count: undefined, depth: 0, expandable: false, open: false, selected: false, last: false, guide: false },
);
const emit = defineEmits<{ toggle: []; select: [] }>();
</script>

<template>
  <div
    class="tree-item"
    :class="{ leaf: guide && !expandable && depth > 0, last }"
    :style="{ paddingLeft: `${depth * 20}px` }"
  >
    <button class="expand" :class="{ placeholder: !expandable }" @click.stop="emit('toggle')">
      <AppIcon name="tree-expand-arrow" :size="9" class="chev" :class="{ open }" />
    </button>
    <button class="body" :class="{ selected }" @click="emit('select')">
      <AppIcon :name="icon" class="deco" />
      <span class="label">{{ label }}</span>
      <span v-if="count !== undefined" class="count">{{ count.toLocaleString() }}</span>
    </button>
  </div>
</template>

<style scoped>
.tree-item {
  position: relative;
  display: flex;
  align-items: center;
  gap: var(--space-2);
  height: 28px;
}
/* 展开按钮容器：宽 9（= 图标宽）/ 高 28（判定高度），图标 9×9 实心三角，展开时旋转 90°；
   有箭头的容器铺侧栏底色——遮住穿过的缩进竖线（视觉不穿过箭头，同 Eagle） */
.expand {
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 9px;
  height: 28px;
  color: var(--c4);
  flex: none;
}
.expand:not(.placeholder) {
  background: var(--c3);
}
/* 无子项时占位保留（藏图标不藏容器：叶子行的竖线要照常穿过） */
.expand.placeholder {
  pointer-events: none;
}
.expand.placeholder .chev {
  opacity: 0;
}
.chev {
  transition: transform 0.15s ease;
}
.chev.open {
  transform: rotate(90deg);
}
.expand:hover .chev :deep(svg [fill-opacity="0.4"]) {
  fill-opacity: 0.7;
}
/* L 形横线：仅最后一个叶子 item，x5 → x9（竖线由 ft-children 容器级绘制、
   止于本行中心，与此横线组成 L）；中间子项纯竖线，不画横线 */
.tree-item.leaf.last .expand::after {
  content: "";
  position: absolute;
  left: 5px;
  top: 50%;
  width: 4px;
  height: 1px;
  background: var(--c4-7);
}
/* item 容器：内边距 4 6 / 圆角 6 / 间距 6 */
.body {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  flex: 1;
  min-width: 0;
  padding: var(--space-4) var(--space-6);
  border-radius: var(--radius-6);
  color: var(--c4);
  transition: background 0.12s ease;
}
.body:hover {
  background: var(--c4-7);
}
.body.selected {
  background: var(--c4-15);
}
.body.selected .deco :deep(svg [stroke-opacity="0.7"]),
.body.selected .deco :deep(svg [fill-opacity="0.7"]) {
  stroke-opacity: 1;
  fill-opacity: 1;
}
.label {
  flex: 1;
  min-width: 0;
  text-align: left;
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.count {
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--c4-40);
  flex: none;
}
.body.selected .count {
  color: var(--c4);
}
</style>
