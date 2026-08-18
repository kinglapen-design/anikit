<script setup lang="ts">
// 通用确认弹窗（6.3）：轻量一次性确认——警示前置（文件管理器打开）/ 可撤销操作说明
//（删除文件夹）/ 不可逆操作二次确认（彻底删除，danger 主按钮错误色）。
// 由 selection.confirmPrompt 驱动，宿主 App.vue。
// 2026-08-13 用户定：统一 AppModal plain 壳（无 X 纯标题，同 27_导入 弹窗族），
// 按钮序主左次右（确认/导入在左、取消在右），旧 del-* 手写卡片废除。
// 2026-08-13 用户定：高危操作走 AppModal alert 危险模板（figma 09：48 状态徽章横排 + 提示音）——
// confirmPrompt.alert 设置即切换（首个用例 = 文件管理器打开警示）。
import AppModal from "./AppModal.vue";
import AppButton from "./AppButton.vue";
import { useSelectionStore } from "../stores/selection";

const selection = useSelectionStore();

function onConfirm(): void {
  const p = selection.confirmPrompt;
  selection.confirmPrompt = null;
  p?.onConfirm();
}
function onCancel(): void {
  selection.confirmPrompt = null;
}
</script>

<template>
  <AppModal
    v-if="selection.confirmPrompt"
    :variant="selection.confirmPrompt.alert ? 'alert' : 'plain'"
    :alert-icon="selection.confirmPrompt.alert ?? 'warning'"
    :title="selection.confirmPrompt.title"
    class="app-confirm"
    @close="onCancel"
  >
    <p class="text">
      <template v-if="selection.confirmPrompt.bodySegments">
        <template v-for="(seg, i) in selection.confirmPrompt.bodySegments" :key="i">
          <span v-if="seg.accent" class="accent">{{ seg.text }}</span>
          <template v-else>{{ seg.text }}</template>
        </template>
      </template>
      <template v-else>{{ selection.confirmPrompt.body }}</template>
    </p>
    <template #actions>
      <AppButton
        variant="primary"
        :class="{ danger: selection.confirmPrompt.danger }"
        :label="selection.confirmPrompt.confirmLabel ?? '确认'"
        @click="onConfirm"
      />
      <AppButton variant="panel" label="取消" @click="onCancel" />
    </template>
  </AppModal>
</template>

<style scoped>
/* 正文：正文档常态色；限宽 420（含壳 padding 24×2），超长换行 */
.text {
  margin: 0;
  max-width: 372px;
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-normal);
  word-break: break-all;
}
/* 按钮 88×38（速查表按钮组定宽口径，2026-08-15 探针实证「取消」两字钮塌到 56 宽）；
   min-width 不压长标签（删除文件夹等超 88 按内容撑开） */
.app-confirm :deep(.actions) .app-btn {
  min-width: 88px;
}
/* danger（不可逆操作二次确认）：主按钮错误色 */
.app-confirm :deep(.app-btn.danger) {
  background: var(--error);
}
/* 正文高亮段（3.4 移动确认，29 稿：发起名 / 目标父级名品牌色-常态） */
.text .accent {
  color: var(--brand);
}
</style>
