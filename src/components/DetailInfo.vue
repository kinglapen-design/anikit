<script setup lang="ts">
// 详情模块 · 基础信息（组件布局 06.3）：标题 / 注释 / 链接输入框 + 详情表（token 14）。
// 阶段 1 静态假数据，改名等交互阶段 4 接入。
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
</script>

<template>
  <section class="detail-info">
    <SectionHeader title="基础信息" />
    <div class="body">
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
.body {
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
  padding: var(--space-4) var(--space-8) 0;
}
/* 输入框（token 6 不带按钮）：内边距 6 8 / 圆角 8 / C1-15 / 1px C4-15 */
.field {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  padding: var(--space-6) var(--space-8);
  border-radius: var(--radius-8);
  background: var(--c1-15);
  border: 1px solid var(--c4-15);
}
/* 带链接按钮：内边距 2 2 2 8 */
.field.with-btn {
  padding: var(--space-2) var(--space-2) var(--space-2) var(--space-8);
}
.field:focus-within {
  border-color: var(--brand-1);
}
.field input {
  flex: 1;
  min-width: 0;
  color: var(--c4);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
}
.field input::placeholder {
  color: var(--c4-40);
}
/* 详情表（token 14）：标题 C4-70 / 内容 C4-40，行距 4 */
.detail-table {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: var(--space-4) var(--space-8);
  padding: var(--space-8);
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
}
.k {
  color: var(--c4-70);
  white-space: nowrap;
}
.v {
  color: var(--c4-40);
}
/* 评分（token 20）：5 星 12×12，选中线框 + 填充品牌色-1 */
.stars {
  display: inline-flex;
  gap: var(--space-2);
}
.star.on {
  color: var(--brand-1);
}
.star.on :deep(svg [stroke-opacity="0.4"]),
.star.on :deep(svg [fill-opacity="0.07"]),
.star.on :deep(svg [fill-opacity="0.4"]) {
  stroke-opacity: 1;
  fill-opacity: 1;
}
</style>
