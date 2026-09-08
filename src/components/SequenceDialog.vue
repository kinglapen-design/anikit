<script setup lang="ts">
// 序列帧弹窗（2026-09-08 换代 = 布局/模态窗口/导入弹窗 稿「导入提示-监测到序列帧」；
// 旧 figma 27 §3 AppModal plain 口径退役）：导入弹窗确认后、后端 detect_sequences 命中时追加弹出。
// 壳 = modals/BaseModal（312 宽稿值）；单选 = buttons/Radio；按钮 = ActionButton lg 恒 128×38（次左主右）。
// 稿同时画了来源组（复制/剪切）——与旧 figma 27 §3 同款复制残留（2026-08-12 用户已定：
// 来源已在导入弹窗选过，本窗只问打包/分散；且混导场景层级与打包两问无法并一组），不重复落地。
// 确认发打包/分散模式；取消 = 放弃整个导入。
import { ref } from "vue";
import BaseModal from "./modals/BaseModal.vue";
import Radio from "./buttons/Radio.vue";
import ActionButton from "./buttons/ActionButton.vue";

const emit = defineEmits<{
  confirm: [mode: "pack" | "scatter"];
  cancel: [];
}>();

const mode = ref<"pack" | "scatter">("pack");
</script>

<template>
  <BaseModal title="检测到序列帧" :width="312" class="sequence-dialog">
    <div class="option-group">
      <p class="tip">是否将序列帧打包为动态缩略图</p>
      <div class="radio-row">
        <Radio v-model="mode" value="pack" label="打包导入" />
        <Radio v-model="mode" value="scatter" label="分散导入" />
      </div>
    </div>
    <template #actions>
      <ActionButton label="取消" variant="cancel" size="lg" @click="emit('cancel')" />
      <ActionButton label="确认" variant="primary" size="lg" @click="emit('confirm', mode)" />
    </template>
  </BaseModal>
</template>

<style scoped>
/* 选项组：提示 + 单选行，组内 gap 12（稿） */
.option-group {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
}
/* 提示行：正文档 13/18 亮色/四级（稿） */
.tip {
  margin: 0;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l4);
}
/* 单选行：横向 gap 24（稿） */
.radio-row {
  display: flex;
  flex-direction: row;
  gap: var(--n-space-24);
}
</style>
