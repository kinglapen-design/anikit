<script setup lang="ts">
import { ref } from "vue";
// 详情模块 · 颜色信息（03_UI 布局/04_右侧边栏 03）：图片容器（圆角 6 / 1px border-1 + 格式角标）
// + 色块容器（胶囊圆角 14 / C4）。阶段 1 静态假数据。
import SectionHeader from "./SectionHeader.vue";

// 假色板：贴近参考图的 10 色
const colors = ["#3A6EA5", "#2B9FD9", "#7A5C3E", "#A53E3E", "#3E8A57", "#6B4F8A", "#C0A23E", "#4E4E52", "#8A8D99", "#1F2023"];
const open = ref(true);
</script>

<template>
  <section class="detail-colors">
    <SectionHeader title="颜色信息" v-model:open="open" />
    <div v-if="open" class="body">
      <div class="thumb">
        <span class="badge" :style="{ color: 'var(--fmt-psd)' }">PSD</span>
      </div>
      <div class="swatches">
        <span v-for="c in colors" :key="c" class="swatch" :style="{ background: c }" v-tip="c" />
      </div>
    </div>
  </section>
</template>

<style scoped>
/* 区块容器：内边距 8 / 间距 6 / 圆角 8（02_通用组件/05_区块） */
.detail-colors {
  display: flex;
  flex-direction: column;
  gap: var(--space-6);
  padding: var(--space-8);
  border-radius: var(--radius-8);
}
/* 颜色信息容器：间距 8 */
.body {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-8);
}
/* 图片容器：高 120 / 圆角 6 / 1px border-1 */
.thumb {
  position: relative;
  width: 100%;
  height: 120px;
  border-radius: var(--radius-6);
  box-shadow: inset 0 0 0 1px var(--border-1);
  background: hsl(215 30% 22%);
}
/* 格式角标：左上 4px / 内边距 2 4 / 圆角 4 / C1 */
.badge {
  position: absolute;
  top: 4px;
  left: 4px;
  padding: var(--space-2) var(--space-4);
  border-radius: var(--radius-4);
  background: var(--c1);
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--font-icon-2);
  pointer-events: none;
}
/* 色块容器（02_item 08）：胶囊圆角 14 / C4 / 1px border-1 / 内边距 4 / 间距 4 */
.swatches {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-4);
  padding: var(--space-4);
  border-radius: 14px;
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--border-1);
}
.swatch {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  box-shadow: inset 0 0 0 1px var(--border-1);
  flex: none;
}
.swatch:hover {
  box-shadow: inset 0 0 0 1px var(--font-icon-2);
}
</style>
