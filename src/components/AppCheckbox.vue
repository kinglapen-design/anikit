<script setup lang="ts">
// 单选框 / 复选框（02_通用组件/01_按钮 04）：按钮 16×16、线条 border-1、圆角 4；
// 选中：边框品牌色-2 + 填充品牌色-1，文字品牌色-1。
// tone：menu（默认）文字 font-icon-3/70；dialog（09 导入弹窗）文字 font-icon-2/C4，与提示文字区分。
import AppIcon from "./AppIcon.vue";

const model = defineModel<boolean>({ required: true });
withDefaults(defineProps<{ label: string; tone?: "menu" | "dialog" }>(), { tone: "menu" });
</script>

<template>
  <button class="app-checkbox" :class="{ on: model, dialog: tone === 'dialog' }" @click="model = !model">
    <span class="box">
      <AppIcon v-if="model" name="check" :size="8" />
    </span>
    <span class="label">{{ label }}</span>
  </button>
</template>

<style scoped>
.app-checkbox {
  display: inline-flex;
  align-items: center;
  gap: var(--space-6);
  padding: 0;
  color: var(--font-icon-3);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  flex: none;
}
.box {
  width: 16px;
  height: 16px;
  border-radius: var(--radius-4);
  box-shadow: inset 0 0 0 1px var(--border-1);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--font-icon-1);
  line-height: 0;
  flex: none;
}
.app-checkbox:hover .box {
  box-shadow: inset 0 0 0 1px var(--font-icon-3);
}
.app-checkbox.on {
  color: var(--font-icon-2);
}
/* dialog 语气（09 导入弹窗）：未选中 font-icon-2 / 选中 C4，与提示文字（font-icon-3）区分 */
.app-checkbox.dialog {
  color: var(--font-icon-2);
}
.app-checkbox.dialog.on {
  color: var(--font-icon-1);
}
.app-checkbox.on .box {
  box-shadow: inset 0 0 0 1px var(--brand-2);
  background: var(--brand-1);
}
/* 选中时对勾 font-icon-1 全亮（SVG 烘 100%，无需再提 opacity；二级菜单对勾颜色独立） */
.app-checkbox.on .box {
  color: var(--font-icon-1);
}
</style>
