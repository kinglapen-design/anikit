<script setup lang="ts">
// 视图切换按钮组（02_通用组件/01_按钮 05）：撑满父容器、按钮均分，永远有一个选中；
// 切换时选中底色 border-1 平移动画，点击瞬间先取消 hover 再平移。
import { computed, ref } from "vue";
import AppIcon from "./AppIcon.vue";

export interface ViewOption {
  value: string;
  icon: string;
  title: string;
}

const props = defineProps<{ options: ViewOption[] }>();
const model = defineModel<string>({ required: true });
const index = computed(() => Math.max(0, props.options.findIndex((o) => o.value === model.value)));

// 点击瞬间取消 hover（避免平移经过的按钮残留 hover 底色）
const sliding = ref(false);
let timer: ReturnType<typeof setTimeout> | undefined;
function select(value: string): void {
  if (value === model.value) return;
  sliding.value = true;
  clearTimeout(timer);
  timer = setTimeout(() => (sliding.value = false), 200);
  model.value = value;
}
</script>

<template>
  <div class="view-switch" :class="{ sliding }">
    <span class="indicator" :style="{ transform: `translateX(calc(${index} * (100% + 4px)))` }" />
    <button
      v-for="o in options"
      :key="o.value"
      class="item"
      :class="{ on: o.value === model }"
      :title="o.title"
      @click="select(o.value)"
    >
      <AppIcon :name="o.icon" />
    </button>
  </div>
</template>

<style scoped>
.view-switch {
  position: relative;
  display: flex;
  gap: var(--space-4);
  padding: var(--space-4);
  border-radius: var(--radius-8);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--border-1);
  width: 100%;
}
/* 选中底色：宽度 = 均分按钮宽，按（自身宽 + 间距 4）平移 */
.indicator {
  position: absolute;
  left: var(--space-4);
  top: var(--space-4);
  width: calc((100% - 20px) / 4);
  height: 28px;
  border-radius: var(--radius-6);
  background: var(--border-1);
  transition: transform 0.18s ease;
}
.item {
  position: relative;
  z-index: 1;
  flex: 1;
  height: 28px;
  padding: var(--space-6) 0;
  border-radius: var(--radius-6);
  /* 视图切换图标常态 font-icon-3，选中 font-icon-1（SVG 烘 100%，明暗全由 color 档位） */
  color: var(--font-icon-3);
  transition: background 0.12s ease;
}
.item:hover {
  background: var(--font-icon-6);
}
.sliding .item:hover {
  background: none;
}
.item.on {
  color: var(--font-icon-1);
}
</style>
