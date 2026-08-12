<script setup lang="ts">
// 序列帧弹窗（figma 27_导入 §3）：09 导入弹窗确认后、后端 detect_sequences 命中时追加弹出。
// 来源（复制/剪切）已在导入弹窗选过，本窗只问打包/分散（2026-08-12 用户定：稿 §3 来源组为复制残留，
// 单组布局同 §1）。确认发打包/分散模式；取消 = 放弃整个导入。
import { ref } from "vue";
import AppModal from "./AppModal.vue";
import AppRadio from "./AppRadio.vue";
import AppButton from "./AppButton.vue";

const emit = defineEmits<{
  confirm: [mode: "pack" | "scatter"];
  cancel: [];
}>();

const mode = ref<"pack" | "scatter">("pack");
</script>

<template>
  <AppModal variant="plain" title="检测到序列帧" class="sequence-dialog" @close="emit('cancel')">
    <div class="option-group">
      <p class="tip">是否将序列帧打包为动态缩略图</p>
      <div class="radio-row">
        <AppRadio v-model="mode" value="pack" label="打包导入" />
        <AppRadio v-model="mode" value="scatter" label="分散导入" />
      </div>
    </div>
    <template #actions>
      <AppButton variant="primary" label="确认" @click="emit('confirm', mode)" />
      <AppButton variant="panel" label="取消" @click="emit('cancel')" />
    </template>
  </AppModal>
</template>

<style scoped>
/* 内容定宽 272（同 ImportDialog，figma 27：弹窗 320 = 272 + padding 24×2） */
.sequence-dialog :deep(.content) {
  width: 272px;
}
.option-group {
  display: flex;
  flex-direction: column;
  gap: var(--space-12);
}
.tip {
  margin: 0;
  font-size: var(--text-label-size);
  line-height: var(--text-label-line);
  color: var(--text-label);
}
.radio-row {
  display: flex;
  flex-direction: row;
  gap: var(--space-24);
}
.sequence-dialog :deep(.actions) .app-btn {
  width: 88px;
}
</style>
