<script setup lang="ts">
// 搜索框（1.2 重构，figma-css 01_主窗口 11648 搜索框段）：通栏 46 / C3 / 上边框 暗-悬停——
// 搜索按钮 + 占位文案（快捷键引导）+ 快捷键徽章组（/ 筛选 · // 搜索 · ~ 全局）+ 历史下拉按钮。
// 搜索功能 3.6（分词搜索）接入，本轮为样式壳；输入框保留输入行为（无搜索响应）。
import { ref } from "vue";
import AppIcon from "./AppIcon.vue";

const text = ref("");
</script>

<template>
  <div class="search-bar">
    <button class="search-btn" v-tip="'搜索'">
      <AppIcon name="search" :size="18" />
    </button>
    <input
      v-model="text"
      type="text"
      placeholder="/ 筛选看板    // 分词搜索    ~ 搜索全局"
    />
    <div class="kbd-group">
      <span class="kbd">/ 筛选</span>
      <span class="kbd">// 搜索</span>
      <span class="kbd">~ 全局</span>
    </div>
    <button class="drop-btn" v-tip="'搜索历史'">
      <AppIcon name="nav-history" :size="18" class="rot" />
    </button>
  </div>
</template>

<style scoped>
/* 总容器（figma 11648）：通栏 46 / padding 8 16 / 间距 6 / C3 / 上边框 暗-悬停 */
.search-bar {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  padding: var(--space-8) var(--space-16);
  flex: none;
  background: var(--c3);
  border-top: 1px solid var(--line-hover);
}
/* 搜索按钮（figma 11675）：30×30 padding 6 圆角 6，图标标签档 */
.search-btn,
.drop-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  padding: var(--space-6);
  border-radius: var(--radius-6);
  color: var(--text-label);
  flex: none;
  transition: background 0.15s ease;
}
.search-btn:hover,
.drop-btn:hover {
  background: var(--line-hover);
}
/* 下拉按钮图标（figma 11752 整体 rotate 90°） */
.rot {
  transform: rotate(90deg);
}
input {
  flex: 1;
  min-width: 0;
  color: var(--text-normal);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  background: transparent;
}
input::placeholder {
  color: var(--text-faint);
}
/* 快捷键徽章组（figma 11808 等）：高 25 / padding 4 8 / C3 + 暗-悬停边框 / 圆角 6 / 极淡档 */
.kbd-group {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  flex: none;
}
.kbd {
  display: inline-flex;
  align-items: center;
  padding: var(--space-4) var(--space-8);
  height: 25px;
  box-sizing: border-box;
  background: var(--c3);
  border: 1px solid var(--line-hover);
  border-radius: var(--radius-6);
  font-size: var(--text-label-size);
  line-height: var(--text-label-line);
  color: var(--text-faint);
  white-space: nowrap;
}
</style>
