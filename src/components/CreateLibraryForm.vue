<script setup lang="ts">
// 创建资源库表单（03_UI 布局/07）：名称输入 + 选择保存路径 / 取消；输入框默认聚焦。
// 复用场景：首次打开模态、汉堡菜单创建、切换资源库创建。
import { nextTick, onMounted, ref } from "vue";
import AppIcon from "./AppIcon.vue";

const props = defineProps<{ name: string; busy: boolean }>();
const emit = defineEmits<{ "update:name": [value: string]; submit: [name: string]; cancel: [] }>();

const inputEl = ref<HTMLInputElement | null>(null);
onMounted(() => {
  void nextTick(() => inputEl.value?.focus());
});

function onSubmit(): void {
  const trimmed = props.name.trim();
  if (trimmed && !props.busy) emit("submit", trimmed);
}
</script>

<template>
  <div class="form-card">
    <div class="title-row">
      <AppIcon name="library-badge" :size="24" />
      <span class="title">创建资源库</span>
    </div>
    <p class="hint">请输入资源库的名称，点击下方的选择保存路径</p>
    <input
      ref="inputEl"
      class="name-input"
      type="text"
      :value="name"
      placeholder="请输入资源库名称..."
      @input="emit('update:name', ($event.target as HTMLInputElement).value)"
      @keydown.enter="onSubmit"
    />
    <div class="actions">
      <button class="btn primary" :disabled="!name.trim() || busy" @click="onSubmit">
        选择保存路径
      </button>
      <button class="btn secondary" :disabled="busy" @click="emit('cancel')">取消</button>
    </div>
  </div>
</template>

<style scoped>
/* 卡片：408 宽 / C5 / 1px C4-15 描边（inset box-shadow，规范 4）+ 二级窗口投影 / 圆角 8 */
.form-card {
  box-sizing: border-box;
  width: 408px;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  padding: var(--space-16);
  gap: var(--space-16);
  background: var(--c5);
  box-shadow:
    0 4px 16px rgba(0, 0, 0, 0.4),
    inset 0 0 0 1px var(--c4-15);
  border-radius: var(--radius-8);
}
/* 标题容器：图标 24 + 大标题，gap 6 */
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
/* 提示文字：副文本 C4-40 */
.hint {
  margin: 0;
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  font-weight: var(--text-caption-weight);
  color: var(--c4-40);
}
/* 输入框（03_输入框#01 通用）：336×32 / C1-15 / 1px C4-15 / 圆角 8 / focus 描边品牌色-1 */
.name-input {
  box-sizing: border-box;
  width: 336px;
  height: 32px;
  padding: 6px var(--space-8);
  border: none;
  outline: none;
  background: var(--c1-15);
  box-shadow: inset 0 0 0 1px var(--c4-15);
  border-radius: var(--radius-8);
  font-family: var(--font-family);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  font-weight: var(--text-body-weight);
  color: var(--c4);
}
.name-input::placeholder {
  color: var(--c4-40);
}
.name-input:focus {
  box-shadow: inset 0 0 0 1px var(--brand-1);
}
/* 按钮组（01_按钮#07 大标题按钮-小按钮）：160×34 / 圆角 6 / 无图标 */
.actions {
  display: flex;
  flex-direction: row;
  gap: var(--space-16);
  width: 336px;
}
.btn {
  display: flex;
  flex-direction: row;
  justify-content: center;
  align-items: center;
  padding: 6px 0;
  gap: var(--space-6);
  width: 160px;
  height: 34px;
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
