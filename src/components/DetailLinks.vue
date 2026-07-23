<script setup lang="ts">
import { ref } from "vue";
// 详情模块 · 关联文件夹（02_通用组件/02_item 06，阶段 1 静态假数据）。
import SectionHeader from "./SectionHeader.vue";
import IconButton from "./IconButton.vue";
import AppIcon from "./AppIcon.vue";
const open = ref(true);
</script>

<template>
  <section class="detail-links detail-block">
    <SectionHeader title="关联文件夹" addable grip v-model:open="open" />
    <div v-if="open" class="body">
      <div class="link-item">
        <div class="row">
          <AppIcon name="folder-closed" class="deco" />
          <span class="label">人物</span>
          <span class="spacer" />
          <IconButton name="clear" small title="移除" class="del" />
        </div>
        <div class="sub">雪枝-走路.fla</div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.detail-links {
  display: flex;
  flex-direction: column;
}
/* item 容器：间距 8 */
.body {
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
}
/* 关联文件夹 item：竖向 / 内边距 6 6 6 8 / 圆角 8 / C4 / 1px border-1 / 间距 2 */
.link-item {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  padding: var(--space-6) var(--space-6) var(--space-6) var(--space-8);
  border-radius: var(--radius-8);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--border-1);
}
.link-item:hover {
  background: var(--font-icon-6);
}
.row {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  color: var(--font-icon-1);
}
.label {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.spacer {
  flex: 1;
}
.sub {
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--font-icon-3);
}
/* 删除 ×：常态 font-icon-3 / 悬停 font-icon-2（SVG 烘 100%，明暗全由 color 档位） */
.del {
  color: var(--font-icon-3);
}
.del:hover {
  color: var(--font-icon-2);
}
</style>
