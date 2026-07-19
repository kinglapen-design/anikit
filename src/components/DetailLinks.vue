<script setup lang="ts">
// 详情模块 · 关联文件夹（02_通用组件/02_item 06，阶段 1 静态假数据）。
import SectionHeader from "./SectionHeader.vue";
import IconButton from "./IconButton.vue";
import AppIcon from "./AppIcon.vue";
</script>

<template>
  <section class="detail-links">
    <SectionHeader title="关联文件夹" addable />
    <div class="body">
      <div class="link-item">
        <div class="row">
          <AppIcon name="folder-closed" class="deco" />
          <span class="label">人物</span>
          <span class="spacer" />
          <IconButton name="window-close" small title="移除" />
        </div>
        <div class="sub">雪枝-走路.fla</div>
      </div>
    </div>
  </section>
</template>

<style scoped>
/* 区块容器：内边距 8 / 间距 6 / 圆角 8 */
.detail-links {
  display: flex;
  flex-direction: column;
  gap: var(--space-6);
  padding: var(--space-8);
  border-radius: var(--radius-8);
}
/* item 容器：间距 8 */
.body {
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
}
/* 关联文件夹 item：竖向 / 内边距 6 6 6 8 / 圆角 8 / C1-15 / 1px C4-15 / 间距 2 */
.link-item {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  padding: var(--space-6) var(--space-6) var(--space-6) var(--space-8);
  border-radius: var(--radius-8);
  background: var(--c1-15);
  border: 1px solid var(--c4-15);
}
.link-item:hover {
  background: var(--c1-7);
}
.row {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  color: var(--c4);
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
  color: var(--c4-40);
}
</style>
