<script setup lang="ts">
import { ref } from "vue";
// 详情模块 · 基础信息（03_UI 布局/04_右侧边栏 03）：标题 / 注释 / 链接输入框（高 32）
// + 详情容器（两列：标题 font-icon-2 / 内容 font-icon-3，行距 2）。阶段 1 静态假数据。
import SectionHeader from "./SectionHeader.vue";
import IconButton from "./IconButton.vue";
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
  <section class="detail-info">
    <SectionHeader title="基础信息" v-model:open="open" />
    <div v-if="open" class="body">
      <div class="field">
        <input type="text" value="中继站场景" placeholder="输入标题" readonly />
      </div>
      <div class="field">
        <input type="text" placeholder="输入注释" readonly />
      </div>
      <div class="field with-btn">
        <input type="text" placeholder="http://" readonly />
        <IconButton name="open-link" title="打开链接" />
      </div>
      <div class="detail-table">
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
    </div>
  </section>
</template>

<style scoped>
/* 区块容器：内边距 8 / 间距 6 / 圆角 8 */
.detail-info {
  display: flex;
  flex-direction: column;
  gap: var(--space-6);
  padding: var(--space-8);
  border-radius: var(--radius-8);
}
/* 内容容器：间距 8 */
.body {
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
}
/* 输入框（03_输入框 01）：内边距 6 8 / 圆角 8 / C4 / 1px border-1；聚焦边框品牌色-1 */
.field {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  height: 32px;
  padding: var(--space-6) var(--space-8);
  border-radius: var(--radius-8);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--border-1);
}
/* 带链接按钮（03_输入框 02）：内边距 2 2 2 8 */
.field.with-btn {
  padding: var(--space-2) var(--space-2) var(--space-2) var(--space-8);
}
.field:focus-within {
  box-shadow: inset 0 0 0 1px var(--brand-1);
}
.field input {
  flex: 1;
  min-width: 0;
  color: var(--font-icon-1);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
}
.field input::placeholder {
  color: var(--font-icon-3);
}
/* 详情容器：内边距 8 / 间距 16 / 圆角 8；标题列拉伸居左、值列居右，行距 2 */
.detail-table {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: var(--space-2) var(--space-16);
  padding: var(--space-8);
  border-radius: var(--radius-8);
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
}
.k {
  color: var(--font-icon-2);
  white-space: nowrap;
}
.v {
  color: var(--font-icon-3);
}
/* 评分 item（02_item 09）：5 星 12×12 / 间距 4；常态线框 font-icon-3，选中线条 + 填充警告色 */
.stars {
  display: inline-flex;
  gap: var(--space-4);
}
.star {
  color: var(--font-icon-1);
}
.star :deep(svg [stroke-opacity="0.7"]) {
  stroke-opacity: 0.4;
}
.star.on :deep(path) {
  fill: var(--warning);
  stroke: var(--warning);
  stroke-opacity: 1;
}
</style>
