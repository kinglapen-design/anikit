<script setup lang="ts">
// 向导第二步（38 稿）：输入资源库名称 + 开启自动导入开关（值存库级 settings.json，行为 2.6 接）。
import TextInput from "./inputs/TextInput.vue";
import SwitchButton from "./buttons/SwitchButton.vue";

const name = defineModel<string>("name", { required: true });
const autoImport = defineModel<boolean>("autoImport", { required: true });
defineProps<{ error: string }>();
</script>

<template>
  <div class="step-library">
    <span class="field-label">输入资源库名称:</span>
    <TextInput v-model="name" placeholder="资源库名称" />
    <span class="field-label">开启自动导入</span>
    <SwitchButton v-model="autoImport" />
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
  gap: var(--n-space-12);
  width: 100%;
  flex: 1;
  min-height: 0;
}
.field-label {
  flex: none;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l2);
}
/* 步骤内联错误行（同 WizardStepSpace 口径）：absolute 横跨内容区，pointer-events none 防挡点击 */
.step-error {
  position: absolute;
  left: var(--space-48);
  right: var(--space-48);
  bottom: var(--n-space-8);
  margin: 0;
  pointer-events: none;
  text-align: center;
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  color: var(--n-danger);
}
</style>
