<script setup lang="ts">
// 筛选弹窗 · chips 流式多选（扩展名 / 链接状态共用；稿 = 中间面板-多功能悬浮栏-筛选/扩展名 379×48、
// 链接 232×48）：壳 .fb-pop + padding 12 + gap 8 单行横排（稿无清除钮——取消靠再点 chip，
// 条件清零维度即失效；2026-10-09 业务接入定：扩展名候选多时 flex-wrap 换行 + 限 4 行滚轮，
// 标签稿「列表太长加滚轮」同口径）。chips = TagButton solid sm（未选 通用/五级；
// 选中 品牌/常态 + 立体感-主要按钮 + 一级文字，稿 prose「可以多选」）。
import TagButton from "../buttons/TagButton.vue";
import "./bars.css";

withDefaults(
  defineProps<{
    items?: readonly { key: string; label: string }[];
    selected?: readonly string[];
    forceHover?: string; // 预览页静态验收：某 chip hover
  }>(),
  { items: () => [], selected: () => [], forceHover: undefined },
);
const emit = defineEmits<{ toggle: [key: string] }>();
</script>

<template>
  <div class="fb-pop fp-root fp-chips">
    <TagButton
      v-for="it in items"
      :key="it.key"
      :label="it.label"
      :selected="selected.includes(it.key)"
      :force-state="forceHover === it.key ? 'hover' : ''"
      @click="emit('toggle', it.key)"
    />
  </div>
</template>

<style scoped>
.fp-chips {
  display: flex;
  flex-wrap: wrap;
  gap: var(--n-space-8);
  padding: var(--n-space-12);
  width: max-content;
  max-width: 380px; /* 稿宽上限（扩展名稿 379） */
  /* 限 4 行滚轮（行 24 × 4 + gap 8 × 3 + 上下 padding 12 × 2 = 144） */
  max-height: 144px;
  overflow-y: auto;
}
</style>
