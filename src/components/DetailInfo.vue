<script setup lang="ts">
import { ref } from "vue";
// 详情模块 · 基础信息（03_UI 布局/04_右侧边栏 03，2026-07-23 用户定重组）：
// 输入框已移入颜色信息区块；本区块 = 副标题（grip 抓手变体）+ 详情容器
// （两列：标题左对齐 font-icon-2 / 值右对齐 font-icon-3，行距 2）。阶段 1 静态假数据。
import SectionHeader from "./SectionHeader.vue";
import AppIcon from "./AppIcon.vue";

const rows = [
  { k: "尺寸", v: "1920 x 1080" },
  { k: "文件大小", v: "123 MB" },
  { k: "格式", v: "PSD" },
  { k: "添加日期", v: "2026/07/12 14:26" },
  { k: "创建日期", v: "2026/07/12 14:26" },
  { k: "修改日期", v: "2026/07/12 14:26" },
];
const open = ref(true);
</script>

<template>
  <section class="detail-info detail-block">
    <SectionHeader title="基础信息" grip v-model:open="open" />
    <div v-if="open" class="detail-table">
      <span class="k">评分</span>
      <span class="v stars">
        <AppIcon
          v-for="i in 5"
          :key="i"
          name="favorites-view"
          :size="12"
          class="star"
          :class="{ on: i <= 2 }"
        />
      </span>
      <template v-for="r in rows" :key="r.k">
        <span class="k">{{ r.k }}</span>
        <span class="v">{{ r.v }}</span>
      </template>
    </div>
  </section>
</template>

<style scoped>
.detail-info {
  display: flex;
  flex-direction: column;
}
/* 详情容器：两列网格 / 列间距 16 / 行距 2；标题列居左、值列右对齐 */
.detail-table {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: var(--space-2) var(--space-16);
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
}
.k {
  color: var(--font-icon-2);
  white-space: nowrap;
}
.v {
  color: var(--font-icon-3);
  justify-self: end;
  text-align: right;
}
/* 评分 item（02_item 09）：5 星 12×12 / 间距 4；常态线框 font-icon-3，选中线条 + 填充警告色 */
.stars {
  display: inline-flex;
  gap: var(--space-4);
}
.star {
  color: var(--font-icon-3);
}
.star.on :deep(path) {
  fill: var(--warning);
  stroke: var(--warning);
}
</style>
