<script setup lang="ts">
// 通用确认弹窗（6.3）——2026-09-08 换代：渲染层 AppModal（plain / alert）退役，换 modals/ConfirmModal
//（组件/模态窗口/弹窗 稿：520 宽 / 标题 18/26 Bold 一级 / 正文 13/18 三级 / 按钮 120×38 次左主右）。
// 字段映射：alert warning → kind warning（状态/警告钮）；alert error / danger → kind danger（状态/禁止钮）；
// 无 alert 非危险 → kind brand（品牌/常态钮——稿：不危险的提示换品牌色）。
// alert 提示音保留（figma 09 侵入式语义，lib/sounds playAlert）；bodySegments 高亮段保留 = 品牌/常态
// --n-brand（新稿正文未画高亮——功能必需就近定档，旧 29 稿品牌色高亮口径沿，色号随新品牌）。
// 由 selection.confirmPrompt 驱动，宿主 App.vue。
import { watch } from "vue";
import ConfirmModal from "./modals/ConfirmModal.vue";
import { playAlert } from "../lib/sounds";
import { useSelectionStore, type ConfirmPrompt } from "../stores/selection";

const selection = useSelectionStore();

// alert 提示音：弹窗出现时播一次（旧 AppModal alert onMounted 口径）
watch(
  () => selection.confirmPrompt,
  (p) => {
    if (p?.alert) playAlert();
  },
);

function kindOf(p: ConfirmPrompt): "warning" | "danger" | "brand" {
  if (p.alert === "error" || p.danger) return "danger";
  if (p.alert === "warning") return "warning";
  return "brand";
}

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
  <ConfirmModal
    v-if="selection.confirmPrompt"
    :title="selection.confirmPrompt.title"
    :kind="kindOf(selection.confirmPrompt)"
    :confirm-label="selection.confirmPrompt.confirmLabel ?? '确认'"
    @confirm="onConfirm"
    @cancel="onCancel"
  >
    <template v-if="selection.confirmPrompt.bodySegments">
      <template v-for="(seg, i) in selection.confirmPrompt.bodySegments" :key="i">
        <span v-if="seg.accent" class="accent">{{ seg.text }}</span>
        <template v-else>{{ seg.text }}</template>
      </template>
    </template>
    <template v-else>{{ selection.confirmPrompt.body }}</template>
  </ConfirmModal>
</template>

<style scoped>
/* 正文高亮段（3.4 移动确认：发起名 / 目标父级名品牌色——新稿未画高亮，就近定档 --n-brand） */
.accent {
  color: var(--n-brand);
}
</style>
