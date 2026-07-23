<script setup lang="ts">
// 详情面板（PRD 04-5）：按模块组装，各模块独立、可增删（模块化设计）。
import DetailColors from "./DetailColors.vue";
import DetailInfo from "./DetailInfo.vue";
import DetailLinks from "./DetailLinks.vue";
import DetailTags from "./DetailTags.vue";
import DetailPrompt from "./DetailPrompt.vue";
</script>

<template>
  <div class="detail-panel">
    <DetailColors />
    <DetailInfo />
    <DetailLinks />
    <DetailTags />
    <DetailPrompt />
  </div>
</template>

<style scoped>
/* 详情容器：区块统一规则（2026-07-23 用户定）——内边距 12 16 / 间距 8 /
   通栏分割线 border-2 以 inset box-shadow 绘制（在背景之上，hover 底色 C2 盖不住；
   首个区块不画）；侧栏外层无 padding，分割线横贯 */
.detail-panel {
  display: flex;
  flex-direction: column;
}
.detail-panel :deep(.detail-block) {
  gap: var(--space-8);
  padding: 12px 16px;
  box-shadow: inset 0 1px 0 0 var(--border-2);
}
.detail-panel :deep(.detail-block:first-child) {
  box-shadow: none;
}
/* 颜色信息区块（pinned：永久置顶、无副标题、不参与拖拽排序）：
   内边距 8 16 12 16 / 间距 16 */
.detail-panel :deep(.detail-block.pinned) {
  gap: var(--space-16);
  padding: var(--space-8) var(--space-16) 12px;
}
/* 可移动区块 hover：底色 C2（详情面板区块悬停底色，2026-07-23 用户定） */
.detail-panel :deep(.detail-block:not(.pinned)):hover {
  background: var(--c2);
}
</style>
