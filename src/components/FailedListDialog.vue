<script setup lang="ts">
// 失败清单弹窗（导入 / 导出共用，2026-08-14 用户定）：AppModal alert 错误模板
// （48 error 徽章 + 侵入式提示音，figma 09），描述 = 失败文件名依次竖排、间距 8；
// 主按钮"重试"（emit retry，由宿主分流 content.retryFailedImport / retryFailedExport）/
// 次按钮"退出"（主左次右）。path 缺失项回退显示原因防空行。
// 原 ImportResultDialog 手写壳（2026-08-14 废除，"文件名 —— 原因"双段同步废除）。
import { ref } from "vue";
import AppModal from "./AppModal.vue";
import AppButton from "./AppButton.vue";
import ScrollThumb from "./ScrollThumb.vue";

defineProps<{
  title: string; // "导入未完成" / "导出未完成"
  failed: { path: string; reason: string }[];
}>();
const emit = defineEmits<{ close: []; retry: [] }>();

function fileName(p: string): string {
  return p.split(/[\\/]/).pop() ?? p;
}

// 悬浮滚轮容器锚点（ScrollThumb：滚动容器须为包裹层直接子元素）
const listEl = ref<HTMLElement>();
</script>

<template>
  <AppModal variant="alert" alert-icon="error" :title="title" @close="emit('close')">
    <div class="scroll-wrap">
      <div ref="listEl" class="fail-list kit-scroll">
        <p v-for="(f, i) in failed" :key="i" class="fail-item">{{ f.path ? fileName(f.path) : f.reason }}</p>
      </div>
      <ScrollThumb :body="listEl" />
    </div>
    <template #actions>
      <AppButton variant="primary" label="重试" @click="emit('retry')" />
      <AppButton variant="panel" label="退出" @click="emit('close')" />
    </template>
  </AppModal>
</template>

<style scoped>
/* 滚动包裹层：relative 锚 ScrollThumb（悬浮滚轮悬停显隐，主窗口统一交互）；
   限宽 372（同 AppConfirm 正文口径），长文件名换行 */
.scroll-wrap {
  position: relative;
  max-width: 372px;
}
/* 失败清单：文件名竖排间距 8（用户定）；超高自滚（上限 = 旧弹窗 160 沿用） */
.fail-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
  max-height: 160px;
}
.fail-item {
  margin: 0;
  word-break: break-all;
}
</style>
