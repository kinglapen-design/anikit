<script setup lang="ts">
// 详情模块 · 颜色信息（token 8）：缩略图 + 色块矩阵（阶段 1 静态假数据）。
import SectionHeader from "./SectionHeader.vue";

// 假色板：贴近参考图的 10 色
const colors = ["#3A6EA5", "#2B9FD9", "#7A5C3E", "#A53E3E", "#3E8A57", "#6B4F8A", "#C0A23E", "#4E4E52", "#8A8D99", "#1F2023"];
</script>

<template>
  <section class="detail-colors">
    <SectionHeader title="颜色信息" />
    <div class="body">
      <div class="thumb" />
      <div class="swatches">
        <span
          v-for="c in colors"
          :key="c"
          class="swatch"
          :style="{ background: c }"
          :title="c"
        />
      </div>
    </div>
  </section>
</template>

<style scoped>
/* 内容容器：内边距 8 / 子项间距 4 */
.body {
  display: flex;
  flex-direction: column;
  gap: var(--space-6);
  padding: var(--space-4) var(--space-8) 0;
}
.thumb {
  width: 100%;
  aspect-ratio: 16 / 9;
  border-radius: var(--radius-6);
  border: 1px solid var(--c4-15);
  background: hsl(215 30% 22%);
}
/* 色块容器：胶囊形 / C1-15 / 1px C4-15 / 内边距 4 / 间距 4 */
.swatches {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-4);
  padding: var(--space-4);
  border-radius: 999px;
  background: var(--c1-15);
  border: 1px solid var(--c4-15);
}
.swatch {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  border: 1px solid var(--c4-15);
  flex: none;
}
.swatch:hover {
  border-color: var(--c4);
}
</style>
