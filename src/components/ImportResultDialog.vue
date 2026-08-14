<script setup lang="ts">
// 导入失败弹窗（2026-08-14 用户定，替掉旧手写壳）：AppModal alert 错误模板
// （48 error 徽章 + 侵入式提示音，figma 09），描述 = 失败文件名依次竖排、间距 8；
// 主按钮"重试"（按原导入参数重投失败项，content.retryFailedImport）/ 次按钮"退出"（主左次右）。
// 旧"文件名 —— 原因"双段废除（用户定：描述只写文件名）；path 缺失项回退显示原因防空行。
import { ref } from "vue";
import AppModal from "./AppModal.vue";
import AppButton from "./AppButton.vue";
import ScrollThumb from "./ScrollThumb.vue";
import { useContentStore, type FailedItem } from "../stores/content";

defineProps<{ failed: FailedItem[] }>();
const emit = defineEmits<{ close: [] }>();
const content = useContentStore();

function fileName(p: string): string {
  return p.split(/[\\/]/).pop() ?? p;
}

function retry(): void {
  content.retryFailedImport();
  emit("close");
}

// 悬浮滚轮容器锚点（ScrollThumb：滚动容器须为包裹层直接子元素）
const listEl = ref<HTMLElement>();
</script>

<template>
  <AppModal variant="alert" alert-icon="error" title="导入未完成" @close="emit('close')">
    <div class="scroll-wrap">
      <div ref="listEl" class="fail-list kit-scroll">
        <p v-for="(f, i) in failed" :key="i" class="fail-item">{{ f.path ? fileName(f.path) : f.reason }}</p>
      </div>
      <ScrollThumb :body="listEl" />
    </div>
    <template #actions>
      <AppButton variant="primary" label="重试" @click="retry" />
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
