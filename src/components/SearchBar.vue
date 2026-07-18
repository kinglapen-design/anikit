<script setup lang="ts">
// 搜索框（token 12）：阶段 1 仅 UI，搜索功能阶段 4 接入。
import { ref } from "vue";
import AppIcon from "./AppIcon.vue";

const text = ref("");
const focused = ref(false);
</script>

<template>
  <div class="search-bar" :class="{ focused }">
    <button class="icon-slot" title="搜索">
      <AppIcon name="search" />
    </button>
    <input
      v-model="text"
      type="text"
      placeholder="输入文字搜索当前文件夹子项，/ 切换筛选模式，// 全局搜索"
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
.search-bar {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  padding: var(--space-6);
  border-radius: var(--radius-8);
  background: var(--c1-50);
  border: 1px solid var(--c4-15);
}
.search-bar.focused {
  border-color: var(--brand-1);
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
.icon-slot:hover :deep(svg [stroke-opacity="0.4"]),
.icon-slot:hover :deep(svg [fill-opacity="0.4"]) {
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
