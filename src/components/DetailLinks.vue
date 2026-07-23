<script setup lang="ts">
import { ref } from "vue";
// 详情模块 · 关联文件夹（02_通用组件/02_item 06，2026-07-23 用户定一栏布局）：
// 文件夹名（图标 + 名称）与文件名一栏；文件名与 X 一组（间距 4），
// X 常态不占位隐藏（空间留给文件名），悬停 item 时文件名左移露出 X。阶段 1 静态假数据。
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
        <AppIcon name="folder-closed" class="deco" />
        <span class="label">人物</span>
        <span class="spacer" />
        <span class="file-group">
          <span class="sub">雪枝-走路.fla</span>
          <IconButton name="clear" small title="移除" class="del" />
        </span>
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
/* 关联文件夹 item：一栏 / 内边距 4 4 4 8 / 圆角 8 / C4 / 1px border-1 / 间距 6 */
.link-item {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  padding: var(--space-4) var(--space-4) var(--space-4) var(--space-8);
  border-radius: var(--radius-8);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--border-1);
  color: var(--font-icon-1);
}
.link-item:hover {
  background: var(--c5);
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
/* 文件名 + X 一组：无间距（X 自身内边距 4 兼作间隔；X 隐藏时文件名的右 4 与左 8 补齐一致） */
.file-group {
  display: flex;
  align-items: center;
  flex: none;
  min-width: 0;
}
.sub {
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--font-icon-3);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
  padding-right: var(--space-4);
}
/* 装饰图标：font-icon-2（80%，同文件树 item，2026-07-23 用户定） */
.deco {
  color: var(--font-icon-2);
}
/* X：常态不占位隐藏（宽度 0），悬停 item 时展开 24px 露出（文件名随之左移）；
   选择器带 .link-item 提权，覆盖 IconButton.small 的 width 24 */
.link-item .del {
  width: 0;
  padding-left: 0;
  padding-right: 0;
  overflow: hidden;
  opacity: 0;
  color: var(--font-icon-3);
  transition:
    width 0.12s ease,
    opacity 0.12s ease,
    color 0.12s ease;
}
.link-item:hover .del {
  width: 24px;
  opacity: 1;
}
.del:hover {
  color: var(--font-icon-2);
}
</style>
