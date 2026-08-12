<script setup lang="ts">
// 导入弹窗（figma 27_导入 §1 导入文件 / §2 导入文件夹）：AppModal plain 壳（320 宽 / padding 24 / gap 24），
// 来源组（复制/剪切）在上、层级组（保留/剔除层级，仅含文件夹时显示）在下，双组时来源组带下分隔线。
// 默认按稿选中态：复制 + 保留层级；提示文案"剪切导入有风险，谨慎选择"为稿定静态文案（含风险明示）。
// 确认只发事件：导入走后台任务 + Channel 流式（异步不阻塞 UI），弹窗立即关闭。
import { computed, ref } from "vue";
import AppModal from "./AppModal.vue";
import AppRadio from "./AppRadio.vue";
import AppButton from "./AppButton.vue";
import type { ImportDraft } from "../stores/content";

const props = defineProps<{ draft: ImportDraft }>();
const emit = defineEmits<{
  confirm: [opts: { structure: "flatten" | "keep"; source: "copy" | "move" }];
  cancel: [];
}>();

const structure = ref<"flatten" | "keep">("keep");
const source = ref<"copy" | "move">("copy");

const title = computed(
  () => ({ files: "导入文件", folder: "导入文件夹", mixed: "导入文件 / 文件夹" })[props.draft.kind],
);
const showStructure = computed(() => props.draft.kind !== "files");
</script>

<template>
  <AppModal variant="plain" :title="title" :mask-close="false" class="import-dialog" @close="emit('cancel')">
    <!-- 来源组（figma 27：标签 12 标签档 + 单选行 gap 24；双组时带下分隔线） -->
    <div class="option-group" :class="{ divided: showStructure }">
      <p class="tip">剪切导入有风险，谨慎选择</p>
      <div class="radio-row">
        <AppRadio v-model="source" value="copy" label="复制" />
        <AppRadio v-model="source" value="move" label="剪切" />
      </div>
    </div>
    <!-- 层级组（仅含文件夹时显示） -->
    <div v-if="showStructure" class="option-group">
      <p class="tip">是否保留层级导入</p>
      <div class="radio-row">
        <AppRadio v-model="structure" value="keep" label="保留层级" />
        <AppRadio v-model="structure" value="flatten" label="剔除层级" />
      </div>
    </div>
    <template #actions>
      <AppButton variant="primary" label="确认" @click="emit('confirm', { structure, source })" />
      <AppButton variant="panel" label="取消" @click="emit('cancel')" />
    </template>
  </AppModal>
</template>

<style scoped>
/* 内容定宽 272（figma 27：弹窗 320 = 272 + padding 24×2），撑出弹窗定宽 */
.import-dialog :deep(.content) {
  width: 272px;
}
/* 选项组：标签 + 单选行，组内 gap 12 */
.option-group {
  display: flex;
  flex-direction: column;
  gap: var(--space-12);
}
/* 双组时来源组带下分隔线（figma 27 §2：padding-bottom 12 + 1px 暗-悬停） */
.option-group.divided {
  padding-bottom: var(--space-12);
  border-bottom: 1px solid var(--line-hover);
}
/* 提示标签：12 标签档 / 文字-标签色 */
.tip {
  margin: 0;
  font-size: var(--text-label-size);
  line-height: var(--text-label-line);
  color: var(--text-label);
}
/* 单选行：横向 gap 24 */
.radio-row {
  display: flex;
  flex-direction: row;
  gap: var(--space-24);
}
/* 按钮定宽 88（figma 27：确认/取消均 88×38，38 高由 AppButton large 保证） */
.import-dialog :deep(.actions) .app-btn {
  width: 88px;
}
</style>
