<script setup lang="ts">
import { ref } from "vue";
// 详情模块 · 标签（02_通用组件/02_item 07 标签带删除按钮，阶段 1 静态假数据）。
import SectionHeader from "./SectionHeader.vue";
import IconButton from "./IconButton.vue";
import AppIcon from "./AppIcon.vue";

const tags = ["全部标签", "场景"];
const open = ref(true);
</script>

<template>
  <section class="detail-tags detail-block">
    <SectionHeader title="标签" addable grip v-model:open="open" />
    <div v-if="open" class="body">
      <div v-for="t in tags" :key="t" class="tag-item">
        <AppIcon name="text" class="deco" />
        <span class="label">{{ t }}</span>
        <span class="spacer" />
        <IconButton name="clear" small title="删除标签" class="del" />
      </div>
    </div>
  </section>
</template>

<style scoped>
.detail-tags {
  display: flex;
  flex-direction: column;
}
/* 标签流式布局：间距 8，宽度不够换行 */
.body {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-8);
}
/* 标签 item：内边距 4 4 4 8（2026-07-23 用户定）/ 圆角 8 / C4 / 1px border-1 / 间距 6 */
.tag-item {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  padding: var(--space-4) var(--space-4) var(--space-4) var(--space-8);
  border-radius: var(--radius-8);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--border-1);
  color: var(--font-icon-1);
}
.tag-item:hover {
  background: var(--c5);
}
.label {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  white-space: nowrap;
}
.spacer {
  flex: 1;
}
/* 删除 ×：常态 font-icon-3 / 悬停 font-icon-2（SVG 烘 100%，明暗全由 color 档位） */
.del {
  color: var(--font-icon-3);
}
.del:hover {
  color: var(--font-icon-2);
}
/* 装饰图标：font-icon-2（80%，同文件树 item，2026-07-23 用户定） */
.deco {
  color: var(--font-icon-2);
}
</style>
