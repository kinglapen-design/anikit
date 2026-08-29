<script setup lang="ts">
// 分段控制器——docs/UI 重构/Design Token/组件/按钮/ 分段控制器-图标 / 文字 两稿：
// icon = 容器 亮色/半透明-3 高 38（右侧边栏 详情/链接 切换），段高 30 图标 14；
// text = 容器 通用/三级 高 40（字体预览 预览/大小/字形/其他），段高 32 按钮文字；
// 段均分 flex 1；选中 = 亮色/半透明-1 + 立体感-通用 滑块 thumb（位移 300ms 形态切换 --n-dur-300）+ 内容升 亮色/一级；
// 未选 亮色/三级，hover 段底 亮色/半透明-3；hover 过渡 150ms（--n-dur-150）。
import { computed } from "vue";
import AppIcon from "../AppIcon.vue";

const model = defineModel<string>({ required: true });
const props = defineProps<{
  options: readonly { value: string; label?: string; icon?: string }[];
  variant?: "icon" | "text";
}>();

const index = computed(() => Math.max(0, props.options.findIndex((o) => o.value === model.value)));
const n = computed(() => Math.max(1, props.options.length));
// 滑块宽 = (100% - 两侧 padding 8 - 间隙 (n-1)×4) / n；位移 = index × (块宽 + 4 间隙)
const thumbStyle = computed(() => ({
  width: `calc((100% - 8px - ${(n.value - 1) * 4}px) / ${n.value})`,
  transform: `translateX(calc(${index.value} * (100% + 4px)))`,
}));
</script>

<template>
  <div class="seg" :class="variant ?? 'icon'" role="tablist">
    <span class="thumb" :style="thumbStyle" />
    <button
      v-for="o in options"
      :key="o.value"
      class="seg-item"
      :class="{ on: o.value === model }"
      role="tab"
      @click="model = o.value"
    >
      <AppIcon v-if="o.icon" :name="o.icon" :size="18" />
      <span v-else class="seg-lbl">{{ o.label }}</span>
    </button>
  </div>
</template>

<style scoped>
.seg {
  position: relative;
  display: flex;
  align-items: stretch;
  padding: var(--n-space-4);
  gap: var(--n-space-4);
  border-radius: var(--n-radius-8);
  flex: none;
  width: 100%;
}
.seg.icon {
  height: 38px;
  background: var(--n-l-a3);
}
.seg.text {
  height: 40px;
  background: var(--n-g3);
}
.thumb {
  position: absolute;
  top: var(--n-space-4);
  bottom: var(--n-space-4);
  left: var(--n-space-4);
  border-radius: var(--n-radius-6);
  background: var(--n-l-a1);
  box-shadow: var(--n-effect-raised);
  transition: transform var(--n-dur-300) var(--n-ease-in-out);
  pointer-events: none;
}
.seg-item {
  position: relative;
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--n-space-6);
  border-radius: var(--n-radius-6);
  color: var(--n-l3);
  font-size: var(--n-fs-btn);
  line-height: var(--n-lh-btn);
  white-space: nowrap;
  transition:
    background var(--n-dur-150) var(--n-ease-in-out),
    color var(--n-dur-150) var(--n-ease-in-out);
}
.seg-item:hover {
  background: var(--n-l-a3);
}
.seg-item.on,
.seg-item.on:hover {
  background: transparent;
  color: var(--n-l1);
}
</style>
