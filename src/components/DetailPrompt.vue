<script setup lang="ts">
import { ref } from "vue";
// 详情模块 · 提示词（03_UI 布局/04_右侧边栏 03 有提示词态，阶段 1 静态假数据）。
// 输入框高 125（AI 搜索框 6 行滚动）；按钮组三按钮均分填充。
import SectionHeader from "./SectionHeader.vue";
import IconButton from "./IconButton.vue";

const prompt =
  "1.基础信息：参考图1是主角，参考图2是故事发生的场景。\n2.氛围与情绪：主角拿着剑斩杀恶龙，突显恶龙和主角之间的大小对比，突出巨龙的压迫感，强调战斗的紧张感，黄沙弥漫，渲染神秘感。";
const open = ref(true);
</script>

<template>
  <section class="detail-prompt">
    <SectionHeader title="提示词" v-model:open="open" />
    <div v-if="open" class="body">
      <textarea class="prompt-text" :value="prompt" readonly rows="6" />
      <div class="actions">
        <IconButton name="copy-prompt" filled title="复制提示词" class="action" />
        <IconButton name="open-prompt-editor" filled title="打开提示词编辑器" class="action" />
        <IconButton name="delete-prompt" filled title="删除提示词" class="action" />
      </div>
    </div>
  </section>
</template>

<style scoped>
/* 区块容器：内边距 8 / 间距 6 / 圆角 8 */
.detail-prompt {
  display: flex;
  flex-direction: column;
  gap: var(--space-6);
  padding: var(--space-8);
  border-radius: var(--radius-8);
}
/* 内容容器：间距 8 */
.body {
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
}
/* AI 搜索框（03_输入框 04）：内边距 8 / 圆角 8 / C4 / 1px border-1 / 最多 6 行滚动 */
.prompt-text {
  width: 100%;
  height: 125px;
  padding: var(--space-8);
  border-radius: var(--radius-8);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--border-1);
  color: var(--font-icon-3);
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  resize: none;
}
.prompt-text:focus {
  box-shadow: inset 0 0 0 1px var(--brand-1);
}
.prompt-text::-webkit-scrollbar {
  width: 4px;
}
.prompt-text::-webkit-scrollbar-thumb {
  background: var(--border-1);
  border-radius: 2px;
}
/* 按钮组：间距 4，三按钮均分填充 */
.actions {
  display: flex;
  gap: var(--space-4);
}
.action {
  flex: 1;
}
</style>
