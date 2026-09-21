<script setup lang="ts">
// 组名称输入框（分组管理 / 新建分组共用衍生，2026-09-21 用户定两弹窗统一抽件）：
// 稿口径（分组管理 02_样式标注「名称输入框」）= 30 高 / padding 6·12 / 通用/四级底 /
// 文字亮色/一级；基件 TextInput sm 提权（描边 半透明-3 与圆角 8 已合稿不动，左对齐归基件统一口径）。
import TextInput from "./inputs/TextInput.vue";

withDefaults(defineProps<{ placeholder?: string }>(), { placeholder: "未命名分组" });
const model = defineModel<string>({ default: "" });
</script>

<template>
  <TextInput v-model="model" size="sm" class="group-name-input" :placeholder="placeholder" />
</template>

<style scoped>
/* 提权压 .ti-root.sm（高度 26→30 / padding 4·12→6·12 / 底 g2→g4）——class 落子组件根，
   双类选择器 + 父 scoped id 压 .ti-root.sm（GroupManageDialog 原衍生覆盖同机制） */
.group-name-input.ti-root {
  flex: 1;
  min-width: 0;
  height: 30px;
  padding: var(--n-space-6) var(--n-space-12);
  background: var(--n-g4);
}
.group-name-input :deep(.ti-inner) {
  color: var(--n-l1);
}
</style>
