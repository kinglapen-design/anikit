<script setup lang="ts">
// 搜索框（02_通用组件/03_输入框 03）：阶段 1 仅 UI，搜索功能阶段 4 接入。
// 搜索图标 + 展开小箭头是"搜索方式"菜单入口（非搜索按钮）：hover 只改颜色不给底色，
// 避免用户以为要点按钮才搜索（实时输入实时搜索）。
import { ref } from "vue";
import AppIcon from "./AppIcon.vue";
import IconButton from "./IconButton.vue";

const text = ref("");
const focused = ref(false);
</script>

<template>
  <div class="search-bar" :class="{ focused }">
    <button class="lead" v-tip="'搜索方式'">
      <span class="lead-icon"><AppIcon name="search" /></span>
      <AppIcon name="chevron-small" :size="8" class="up" />
    </button>
    <input
      v-model="text"
      type="text"
      placeholder="输入文字搜索当前文件夹子项，tab 切换全局和当前目录..."
      @focus="focused = true"
      @blur="focused = false"
    />
    <button v-if="text" class="clear" v-tip="'清除'" @click="text = ''">
      <AppIcon name="clear" />
    </button>
    <IconButton name="chevron-small" title="展开" class="expand-btn" />
  </div>
</template>

<style scoped>
/* 总容器：内边距 4 / 间距 6 / 圆角 8 / C4 / 1px border-1；聚焦边框品牌色-1 */
.search-bar {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  padding: var(--space-4);
  border-radius: var(--radius-8);
  /* 底色 C3（2026-07-23 用户定，原 C4） */
  background: var(--c3);
  box-shadow: inset 0 0 0 1px var(--border-1);
}
.search-bar.focused {
  box-shadow: inset 0 0 0 1px var(--brand-1);
}
/* 搜索方式入口（搜索图标容器 28×28 + 展开小箭头朝上，间距 2）：无按钮底色，
   常态 font-icon-3 / 悬停 font-icon-1（仅颜色；SVG 烘 100%，明暗全由 color 档位） */
.lead {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  color: var(--font-icon-3);
  flex: none;
}
/* 搜索图标容器：28×28（内边距 6），与右侧展开按钮对齐 */
.lead-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  padding: var(--space-6);
}
.lead:hover {
  color: var(--font-icon-1);
}
.up {
  transform: rotate(180deg);
}
/* 清除按钮（小 24×24）：常态 font-icon-3 / 悬停 font-icon-2（颜色），hover 底色 font-icon-6 */
.clear {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  padding: var(--space-4);
  border-radius: var(--radius-4);
  color: var(--font-icon-3);
  flex: none;
  transition:
    background 0.12s ease,
    color 0.12s ease;
}
.clear:hover {
  background: var(--font-icon-6);
  color: var(--font-icon-2);
}
/* 最右侧展开按钮（图标按钮-大 28×28）：图标 16×16 朝上，常态 font-icon-3 / hover font-icon-1 */
.expand-btn {
  color: var(--font-icon-3);
}
.expand-btn:hover {
  color: var(--font-icon-1);
}
.expand-btn :deep(svg) {
  transform: rotate(180deg);
}
input {
  flex: 1;
  min-width: 0;
  color: var(--font-icon-1);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
}
input::placeholder {
  color: var(--font-icon-placeholder);
}
</style>
