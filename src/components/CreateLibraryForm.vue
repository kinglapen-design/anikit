<script setup lang="ts">
// 创建资源库表单（03_UI 布局/07）：名称输入 + 选择保存路径 / 取消；输入框默认聚焦。
// 复用场景：首次打开模态、汉堡菜单创建、切换资源库创建。
// 校验（Eagle 式，不做按钮禁用）：提交时校验名称，不合规则输入框描边错误色 + 下方提示行；
// 规则同 Windows 文件夹命名（特殊符号 / 结尾空格点 / 系统保留名），输入即清除错误。
import { nextTick, onMounted, ref } from "vue";
import AppIcon from "./AppIcon.vue";

const props = defineProps<{ name: string; busy: boolean }>();
const emit = defineEmits<{ "update:name": [value: string]; submit: [name: string]; cancel: [] }>();

const inputEl = ref<HTMLInputElement | null>(null);
const errorMsg = ref("");
onMounted(() => {
  void nextTick(() => inputEl.value?.focus());
});

// Windows 文件夹命名规则（后端 create 命令同样把关，此处做即时提示）
const ILLEGAL_CHARS = /[\\/:*?"<>|]/;
const RESERVED_NAMES = /^(con|prn|aux|nul|com[1-9]|lpt[1-9])$/i;

function validateName(name: string): string {
  if (!name) return "名称不能留空";
  if (ILLEGAL_CHARS.test(name)) return '名称不能包含 \\ / : * ? " < > | 特殊符号';
  if (/[. ]$/.test(name)) return "名称不能以空格或点结尾";
  if (RESERVED_NAMES.test(name)) return "该名称为系统保留名，请换一个";
  return "";
}

function onInput(e: Event): void {
  errorMsg.value = ""; // 输入即清除错误态
  emit("update:name", (e.target as HTMLInputElement).value);
}

function onSubmit(): void {
  if (props.busy) return;
  const name = props.name.trim();
  const msg = validateName(name);
  if (msg) {
    errorMsg.value = msg;
    return;
  }
  emit("submit", name);
}
</script>

<template>
  <div class="form-card">
    <div class="title-row">
      <!-- 切换资源库同款：24×24 品牌色圆角底 + 16×16 图标（CSS 实现，无需单独导出） -->
      <span class="badge"><AppIcon name="library-badge" :size="16" /></span>
      <span class="title">创建资源库</span>
    </div>
    <p class="hint">请输入资源库的名称，点击下方的选择保存路径</p>
    <div class="field">
      <input
        ref="inputEl"
        class="name-input"
        :class="{ error: errorMsg }"
        type="text"
        :value="name"
        placeholder="请输入资源库名称..."
        @input="onInput"
        @keydown.enter="onSubmit"
      />
      <p v-if="errorMsg" class="error-tip">{{ errorMsg }}</p>
    </div>
    <div class="actions">
      <button class="btn primary" :disabled="busy" @click="onSubmit">选择保存路径</button>
      <button class="btn secondary" :disabled="busy" @click="emit('cancel')">取消</button>
    </div>
  </div>
</template>

<style scoped>
/* 卡片：上限 368（08_二级窗口通用规则；内容设计为撑满型按上限取值）/ C3 / 描边 + 投影 / 圆角 8 */
.form-card {
  box-sizing: border-box;
  width: 368px;
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
/* 标题容器：图标 24 + 大标题，gap 6 */
.title-row {
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: var(--space-6);
  height: 24px;
}
/* 图标容器（同 02_左侧边栏#03 切换资源库）：24×24 / padding 4 / 品牌色-1 / 圆角 6 */
.badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  box-sizing: border-box;
  width: 24px;
  height: 24px;
  padding: var(--space-4);
  border-radius: var(--radius-6);
  background: var(--brand-1);
  color: var(--font-icon-1);
  flex: none;
}
.title {
  font-size: var(--text-title-size);
  line-height: var(--text-title-line);
  font-weight: var(--text-title-weight);
  color: var(--font-icon-1);
}
/* 提示文字：副文本 font-icon-3 */
.hint {
  margin: 0;
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  font-weight: var(--text-caption-weight);
  color: var(--font-icon-3);
}
/* 输入框 + 错误提示行：撑满父容器 */
.field {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  width: 100%;
}
/* 输入框（03_输入框#01 通用）：C4 / 1px border-1 / 圆角 8 / focus 品牌色-1 / 错误态 --error */
.name-input {
  box-sizing: border-box;
  width: 100%;
  height: 32px;
  padding: 6px var(--space-8);
  border: none;
  outline: none;
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--border-1);
  border-radius: var(--radius-8);
  font-family: var(--font-family);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  font-weight: var(--text-body-weight);
  color: var(--font-icon-1);
}
.name-input::placeholder {
  color: var(--font-icon-3);
}
.name-input:focus {
  box-shadow: inset 0 0 0 1px var(--brand-1);
}
.name-input.error,
.name-input.error:focus {
  box-shadow: inset 0 0 0 1px var(--error);
}
/* 错误提示行：副文本、错误色（Eagle 式，见 07 文档） */
.error-tip {
  margin: 0;
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  font-weight: var(--text-caption-weight);
  color: var(--error);
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
