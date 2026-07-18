<script setup lang="ts">
// 详情模块 · 提示词（组件布局 06.6 有提示词态，阶段 1 静态假数据）。
import SectionHeader from "./SectionHeader.vue";
import IconButton from "./IconButton.vue";

const prompt =
  "1.基础信息：参考图1是主角，参考图2是故事发生的场景。\n2.氛围与情绪：主角拿着剑斩杀恶龙，突显恶龙和主角之间的大小对比，突出巨龙的压迫感，强调战斗的紧张感，黄沙弥漫，渲染神秘感。";
</script>

<template>
  <section class="detail-prompt">
    <SectionHeader title="提示词" />
    <div class="body">
      <textarea class="prompt-text" :value="prompt" readonly rows="6" />
      <div class="actions">
        <IconButton name="copy-prompt" filled title="复制提示词" />
        <IconButton name="open-prompt-editor" filled title="打开提示词编辑器" />
        <IconButton name="delete-prompt" filled title="删除提示词" />
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
/* AI 提示词输入框（token 6.3）：内边距 8 / 圆角 8 / C1-15 / 1px C4-15 / 最多 6 行滚动 */
.prompt-text {
  width: 100%;
  padding: var(--space-8);
  border-radius: var(--radius-8);
  background: var(--c1-15);
  border: 1px solid var(--c4-15);
  color: var(--c4-40);
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  resize: none;
}
.prompt-text:focus {
  border-color: var(--brand-1);
}
.prompt-text::-webkit-scrollbar {
  width: 4px;
}
.prompt-text::-webkit-scrollbar-thumb {
  background: var(--c4-15);
  border-radius: 2px;
}
.actions {
  display: flex;
  gap: var(--space-4);
}
</style>
