<script setup lang="ts">
// 向导第二步（38 稿）：输入资源库名称 + 开启自动导入开关（值存库级 settings.json，行为 2.6 接）。
import AppInput from "./AppInput.vue";
import AppSwitch from "./AppSwitch.vue";

const name = defineModel<string>("name", { required: true });
const autoImport = defineModel<boolean>("autoImport", { required: true });
defineProps<{ error: string }>();
</script>

<template>
  <div class="step-library">
    <span class="field-label">输入资源库名称：</span>
    <AppInput v-model="name" placeholder="资源库名称" />
    <span class="field-label">开启自动导入</span>
    <AppSwitch v-model="autoImport" />
    <p v-if="error" class="step-error">{{ error }}</p>
  </div>
</template>

<style scoped>
/* 内容区（38 稿）：row / justify end / align center / padding 0 48 / gap 12 */
.step-library {
  position: relative;
  box-sizing: border-box;
  display: flex;
  flex-direction: row;
  justify-content: flex-end;
  align-items: center;
  padding: 0 var(--space-48);
  gap: var(--space-12);
  width: 100%;
  flex: 1;
  min-height: 0;
}
.field-label {
  flex: none;
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-normal);
}
/* 步骤内联错误行（同 WizardStepSpace 口径） */
.step-error {
  position: absolute;
  left: var(--space-48);
  right: var(--space-48);
  bottom: var(--space-8);
  margin: 0;
  text-align: center;
  font-size: var(--text-label-size);
  line-height: var(--text-label-line);
  color: var(--error);
}
</style>
