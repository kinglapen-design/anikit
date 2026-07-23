<script setup lang="ts">
// 路径错误弹窗（03_UI 布局/08）：打开 / 创建资源库路径校验失败通用；
// 结构同 07（无输入框）：错误图标 24 + 标题"无效路径" + 提示 + 重新选择路径 / 取消。
import AppIcon from "./AppIcon.vue";

defineProps<{ message: string; busy: boolean }>();
const emit = defineEmits<{ retry: []; cancel: [] }>();
</script>

<template>
  <div class="error-card">
    <div class="title-row">
      <AppIcon name="error-large" :size="24" />
      <span class="title">无效路径</span>
    </div>
    <p class="hint">{{ message }}</p>
    <div class="actions">
      <button class="btn primary" :disabled="busy" @click="emit('retry')">重新选择路径</button>
      <button class="btn secondary" :disabled="busy" @click="emit('cancel')">取消</button>
    </div>
  </div>
</template>

<style scoped>
/* 卡片：宽度随最宽子项、上限 368（08_二级窗口通用规则），长路径消息换行 */
.error-card {
  box-sizing: border-box;
  width: fit-content;
  max-width: 368px;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  padding: var(--space-16);
  gap: var(--space-16);
  background: var(--c3);
  box-shadow:
    var(--shadow-pop),
    inset 0 0 0 1px var(--border-1);
  border-radius: var(--radius-8);
}
.title-row {
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: var(--space-6);
  height: 24px;
}
.title {
  font-size: var(--text-title-size);
  line-height: var(--text-title-line);
  font-weight: var(--text-title-weight);
  color: var(--font-icon-1);
}
.hint {
  margin: 0;
  max-width: 100%;
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  font-weight: var(--text-caption-weight);
  color: var(--font-icon-3);
  word-break: break-all; /* 长路径超 368 上限时换行（08 通用规则） */
}
/* 按钮组（01_按钮#07 大标题按钮-小按钮）：宽度按标题自适应（padding 8/16 / 高 38）/ 圆角 6 / 无图标 */
.actions {
  display: flex;
  flex-direction: row;
  justify-content: flex-end; /* 08 通用规则：按钮组右对齐 */
  gap: var(--space-16);
  width: 100%;
}
.btn {
  display: flex;
  flex-direction: row;
  justify-content: center;
  align-items: center;
  padding: 8px 16px;
  gap: var(--space-6);
  height: 38px;
  border: none;
  border-radius: var(--radius-6);
  cursor: pointer;
  font-family: var(--font-family);
  font-size: var(--text-title-size);
  line-height: var(--text-title-line);
  font-weight: var(--text-title-weight);
  color: var(--font-icon-1);
}
.btn.primary {
  background: var(--brand-1);
}
.btn.primary:hover {
  background: var(--brand-2);
}
.btn.secondary {
  background: var(--c5);
}
.btn.secondary:hover {
  background: var(--border-1);
}
.btn:disabled {
  cursor: default;
  opacity: 0.6;
}
</style>
