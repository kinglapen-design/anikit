<script setup lang="ts">
// 详情模块 · 标签（02_通用组件/02_item 07 标签带删除按钮，阶段 1 静态假数据）。
import SectionHeader from "./SectionHeader.vue";
import IconButton from "./IconButton.vue";
import AppIcon from "./AppIcon.vue";

const tags = ["全部标签", "场景"];
</script>

<template>
  <section class="detail-tags">
    <SectionHeader title="标签" addable />
    <div class="body">
      <div v-for="t in tags" :key="t" class="tag-item">
        <AppIcon name="tag" class="deco" />
        <span class="label">{{ t }}</span>
        <span class="spacer" />
        <IconButton name="window-close" small title="删除标签" />
      </div>
    </div>
  </section>
</template>

<style scoped>
/* 区块容器：内边距 8 / 间距 6 / 圆角 8 */
.detail-tags {
  display: flex;
  flex-direction: column;
  gap: var(--space-6);
  padding: var(--space-8);
  border-radius: var(--radius-8);
}
/* 标签流式布局：间距 4，宽度不够换行 */
.body {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-4);
}
/* 标签 item：内边距 6 6 6 8 / 圆角 8 / C1-15 / 1px C4-15 / 间距 6 */
.tag-item {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  padding: var(--space-6) var(--space-6) var(--space-6) var(--space-8);
  border-radius: var(--radius-8);
  background: var(--c1-15);
  border: 1px solid var(--c4-15);
  color: var(--c4);
}
.tag-item:hover {
  background: var(--c1-7);
}
.label {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  white-space: nowrap;
}
.spacer {
  flex: 1;
}
</style>
