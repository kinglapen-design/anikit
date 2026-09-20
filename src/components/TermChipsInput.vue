<script setup lang="ts">
// 分词 chips 输入（33/34 稿名称规则范围区，2026-08-19 复验反馈 7：原单输入框空格分隔不可见、
// 多分词要加很多规则行——改设计稿 chips 形态）：已提交词 = 品牌色底 chip（× 删除）；
// 提交时机 = 回车 / 空格 / 「+」钮 / 失焦（有草稿时）；空草稿 Backspace 删尾 chip。
// IME 组合期（isComposing）不响应回车/空格——中文输入选词确认不误提交。
import { ref } from "vue";
import AppIcon from "./AppIcon.vue";
import IconButton from "./buttons/IconButton.vue";

const props = withDefaults(defineProps<{ terms: string[]; placeholder?: string }>(), {
  placeholder: "请输入分词",
});
const emit = defineEmits<{ add: [value: string]; remove: [value: string] }>();

const draft = ref("");

function commit(): void {
  const v = draft.value.trim();
  if (!v) return;
  draft.value = "";
  emit("add", v);
}
function onKeydown(e: KeyboardEvent): void {
  if (e.isComposing) return; // IME 组合期：回车/空格是选词，不提交
  if (e.key === "Enter" || e.key === " ") {
    e.preventDefault();
    commit();
    return;
  }
  if (e.key === "Backspace" && draft.value === "" && props.terms.length > 0) {
    emit("remove", props.terms[props.terms.length - 1]);
  }
}
</script>

<template>
  <div class="term-chips">
    <span v-for="t in terms" :key="t" class="term-chip">
      <span class="term-text">{{ t }}</span>
      <button class="term-del" title="删除分词" @click="emit('remove', t)">
        <AppIcon name="clear" :size="12" />
      </button>
    </span>
    <input
      v-model="draft"
      class="term-input"
      :placeholder="placeholder"
      @keydown="onKeydown"
      @blur="commit"
    />
    <IconButton variant="bare" size="sm" name="add-large" title="添加分词" class="term-add" @click="commit" />
  </div>
</template>

<style scoped>
.term-chips {
  display: flex;
  align-items: center;
  gap: var(--space-8);
  flex-wrap: wrap;
  flex: 1;
  min-width: 0;
}
/* 已提交分词 chip（选中态口径：品牌色底 + 1px 亮-选中 + 高亮字，同范围 chips .on） */
.term-chip {
  display: inline-flex;
  align-items: center;
  gap: var(--space-6);
  height: 28px;
  box-sizing: border-box;
  padding: var(--space-4) var(--space-8);
  border-radius: var(--radius-6);
  background: var(--brand);
  box-shadow: inset 0 0 0 1px var(--line-active);
  color: var(--text-highlight);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  white-space: nowrap;
  flex: none;
}
.term-del {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  color: var(--text-highlight);
  opacity: 0.7;
  transition: opacity 0.12s ease;
}
.term-del:hover {
  opacity: 1;
}
/* 输入框：裸形（透明底无边框，设计稿）；高度与 chip 行平齐 */
.term-input {
  flex: 1;
  min-width: 72px;
  height: 28px;
  background: transparent;
  color: var(--text-normal);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
}
.term-input::placeholder {
  color: var(--text-label);
}
</style>
