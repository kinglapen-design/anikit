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
/* 卡片样式与 07 创建表单一致（C5 / 描边 + 投影 / 圆角 8 / padding 16 / gap 16） */
.error-card {
  box-sizing: border-box;
  width: 408px;
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
  color: var(--c4);
}
.hint {
  margin: 0;
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  font-weight: var(--text-caption-weight);
  color: var(--c4-40);
}
/* 按钮组（01_按钮#07 大标题按钮-小按钮）：宽度按标题自适应（padding 8/16 / 高 38）/ 圆角 6 / 无图标 */
.actions {
  display: flex;
  flex-direction: row;
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
  color: var(--c4);
}
.btn.primary {
  background: var(--brand-1);
}
.btn.primary:hover {
  background: var(--brand-2);
}
.btn.secondary {
  background: var(--c4-7);
}
.btn.secondary:hover {
  background: var(--c4-15);
}
.btn:disabled {
  cursor: default;
  opacity: 0.6;
}
</style>
