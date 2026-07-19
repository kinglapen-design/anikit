<script setup lang="ts">
// 搜索框（02_通用组件/03_输入框 03）：阶段 1 仅 UI，搜索功能阶段 4 接入。
import { ref } from "vue";
import AppIcon from "./AppIcon.vue";

const text = ref("");
const focused = ref(false);
</script>

<template>
  <div class="search-bar" :class="{ focused }">
    <span class="lead">
      <button class="icon-slot" title="搜索">
        <AppIcon name="search" />
      </button>
      <AppIcon name="chevron-small" :size="8" class="lead-chev" />
    </span>
    <input
      v-model="text"
      type="text"
      placeholder="输入文字搜索当前文件夹子项，tab 切换全局和当前目录..."
      @focus="focused = true"
      @blur="focused = false"
    />
    <button v-if="text" class="icon-slot small" title="清除" @click="text = ''">
      <AppIcon name="clear" />
    </button>
    <button class="icon-slot" title="展开">
      <AppIcon name="chevron-small" :size="8" class="up" />
    </button>
  </div>
</template>

<style scoped>
/* 总容器：内边距 6 / 间距 6 / 圆角 8 / C1-50 / 1px C4-15；聚焦边框品牌色-1 */
.search-bar {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  padding: var(--space-6);
  border-radius: var(--radius-8);
  background: var(--c1-50);
  box-shadow: inset 0 0 0 1px var(--c4-15);
}
.search-bar.focused {
  box-shadow: inset 0 0 0 1px var(--brand-1);
}
/* 搜索图标容器：搜索图标 + 展开图标（8×8），间距 2 */
.lead {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  flex: none;
}
.lead:hover :deep(svg [stroke-opacity="0.7"]),
.lead:hover :deep(svg [fill-opacity="0.7"]) {
  stroke-opacity: 1;
  fill-opacity: 1;
}
.lead-chev {
  color: var(--c4);
}
.icon-slot {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  padding: var(--space-6);
  border-radius: var(--radius-6);
  color: var(--c4);
  flex: none;
  transition: background 0.12s ease;
}
.icon-slot.small {
  width: 24px;
  height: 24px;
  padding: var(--space-4);
}
.icon-slot:hover {
  background: var(--c4-7);
}
.icon-slot:hover :deep(svg [stroke-opacity="0.7"]),
.icon-slot:hover :deep(svg [fill-opacity="0.7"]) {
  stroke-opacity: 1;
  fill-opacity: 1;
}
.up {
  transform: rotate(180deg);
}
input {
  flex: 1;
  min-width: 0;
  color: var(--c4);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
}
input::placeholder {
  color: var(--c4-40);
}
</style>
