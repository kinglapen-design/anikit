<script setup lang="ts">
// 批量重命名输入行（36/37 稿）：随分段模式单显一行输入——
// 序号命名 = 名称 + 起始编号（位宽自动）；替换命名 = 查找文本 + 替换为（纯文本）。
// 分段选择器在底部按钮行（DuplicateDialog）；规则只作用词干，实时驱动各行新名预览。
import AppInput from "./AppInput.vue";

defineProps<{ mode: "sequence" | "replace" }>();
const name = defineModel<string>("name", { default: "" });
const start = defineModel<string>("start", { default: "1" });
const find = defineModel<string>("find", { default: "" });
const replace = defineModel<string>("replace", { default: "" });
</script>

<template>
  <div class="rename-bar">
    <!-- 序号命名（37 稿：名称 通栏 + 起始编号 82 定宽） -->
    <template v-if="mode === 'sequence'">
      <label class="field grow">
        <span class="flabel">名称:</span>
        <AppInput v-model="name" size="large" placeholder="输入名称" />
      </label>
      <label class="field start">
        <span class="flabel">起始编号:</span>
        <AppInput v-model="start" size="large" />
      </label>
    </template>
    <!-- 替换命名（37 稿：查找文本 / 替换为 双通栏） -->
    <template v-else>
      <label class="field grow">
        <span class="flabel">查找文本:</span>
        <AppInput v-model="find" size="large" />
      </label>
      <label class="field grow">
        <span class="flabel">替换为:</span>
        <AppInput v-model="replace" size="large" />
      </label>
    </template>
  </div>
</template>

<style scoped>
/* 输入行（36/37 稿）：60 高 / padding 12 24 / gap 12 */
.rename-bar {
  display: flex;
  align-items: center;
  gap: var(--space-12);
  padding: var(--space-12) var(--space-24);
  height: 60px;
  flex: none;
}
.field {
  display: flex;
  align-items: center;
  gap: var(--space-8);
  min-width: 0;
}
.field.grow {
  flex: 1;
}
.field.start {
  flex: none;
  width: 150px;
}
.field.start :deep(.app-input) {
  width: 82px;
}
/* 标签：正文 / 文字-标签色 */
.flabel {
  flex: none;
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-label);
}
/* 36/37 稿输入框-大 = C1 底（AppInput large 默认 C2，本弹窗壳为 C2 故按稿改 C1） */
.rename-bar :deep(.app-input.large) {
  background: var(--c1);
}
</style>
