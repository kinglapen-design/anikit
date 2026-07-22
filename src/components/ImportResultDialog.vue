<script setup lang="ts">
// 导入结果弹窗：后台导入任务结束后，有失败项时列出清单（不吞错误，规范第 6 条）。
// 壳与零件沿用 09 导入弹窗（C5 / padding 16 / gap 16 / --shadow-pop）。
import type { FailedItem } from "../stores/content";

defineProps<{ failed: FailedItem[] }>();
const emit = defineEmits<{ close: [] }>();

function fileName(p: string): string {
  return p.split(/[\\/]/).pop() ?? p;
}
</script>

<template>
  <div class="result-mask">
    <div class="result-card">
      <h2 class="title">导入未完成</h2>
      <p class="tip">{{ failed.length }} 个未导入：</p>
      <div class="fail-list">
        <p v-for="(f, i) in failed" :key="i" class="fail-item">
          <template v-if="f.path">{{ fileName(f.path) }} —— </template>{{ f.reason }}
        </p>
      </div>
      <div class="actions">
        <button class="btn primary" @click="emit('close')">知道了</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* 遮罩：与 LibrarySetupModal 一致（C1-80） */
.result-mask {
  position: absolute;
  inset: 0;
  z-index: 200;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--c1-80);
}
/* 总容器（08_二级窗口通用规则）：宽度随最宽子项、上限 368，失败清单长文本换行 */
.result-card {
  box-sizing: border-box;
  width: fit-content;
  max-width: 368px;
  min-width: 320px;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  padding: var(--space-16);
  gap: var(--space-16);
  background: var(--c5);
  box-shadow:
    var(--shadow-pop),
    inset 0 0 0 1px var(--c4-15);
  border-radius: var(--radius-8);
}
.title {
  margin: 0;
  font-size: var(--text-title-size);
  line-height: var(--text-title-line);
  font-weight: var(--text-title-weight);
  color: var(--c4);
}
.tip {
  margin: 0;
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--c4-40);
}
/* 失败清单：副文本 C4-40，超高滚动 */
.fail-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  width: 100%;
  max-height: 160px;
  overflow-y: auto;
}
.fail-item {
  margin: 0;
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--c4-40);
  word-break: break-all;
}
.actions {
  display: flex;
  flex-direction: row;
  justify-content: flex-end; /* 08 通用规则：按钮组右对齐 */
  gap: var(--space-16);
  width: 100%;
}
.btn {
  display: flex;
  justify-content: center;
  align-items: center;
  padding: 8px 16px; /* 01_按钮#07 修订：宽度按标题自适应 */
  height: 38px;
  border: none;
  border-radius: var(--radius-6);
  cursor: pointer;
  font-family: var(--font-family);
  font-size: var(--text-title-size);
  line-height: var(--text-title-line);
  font-weight: var(--text-title-weight);
  color: var(--c4);
}
.btn.primary {
  background: var(--brand-1);
}
.btn.primary:hover {
  background: var(--brand-2);
}
</style>
