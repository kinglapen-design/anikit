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
.detail-panel {
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
  padding: var(--space-8) 0;
}
</style>
