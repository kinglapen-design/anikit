<script setup lang="ts">
// 导入弹窗（2026-09-08 换代 = 布局/模态窗口/导入弹窗 稿；旧 figma 27_导入 AppModal plain 口径退役）：
// 壳 = modals/BaseModal（312 宽稿值）；单选 = buttons/Radio；按钮 = ActionButton lg 恒 128×38（次左主右）。
// 来源组（复制/剪切，稿定提示「请选择复制一份导入或剪切导入」）在上、
// 层级组（保留/剔除层级，仅含文件夹时显示）在下；双组时层级组带顶分隔线
// （1px 亮色/半透明-3 + 上 padding 12）+ 来源组下 padding 12（稿值）。
// 默认按稿选中态：复制 + 保留层级。确认只发事件：导入走后台任务 + Channel 流式
// （异步不阻塞 UI），弹窗立即关闭。
import { computed, ref } from "vue";
import BaseModal from "./modals/BaseModal.vue";
import Radio from "./buttons/Radio.vue";
import ActionButton from "./buttons/ActionButton.vue";
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
  <BaseModal :title="title" :width="312" class="import-dialog">
    <!-- 来源组（稿：提示 正文 13/18 四级 + 单选行 gap 24；双组时下 padding 12 给分隔线让位） -->
    <div class="option-group" :class="{ divided: showStructure }">
      <p class="tip">请选择复制一份导入或剪切导入</p>
      <div class="radio-row">
        <Radio v-model="source" value="copy" label="复制" />
        <Radio v-model="source" value="move" label="剪切" />
      </div>
    </div>
    <!-- 层级组（仅含文件夹时显示；双组时顶分隔线 1px 亮色/半透明-3 + 上 padding 12——稿值） -->
    <div v-if="showStructure" class="option-group top-line">
      <p class="tip">是否保留层级导入</p>
      <div class="radio-row">
        <Radio v-model="structure" value="keep" label="保留层级" />
        <Radio v-model="structure" value="flatten" label="剔除层级" />
      </div>
    </div>
    <template #actions>
      <ActionButton label="取消" variant="cancel" size="lg" @click="emit('cancel')" />
      <ActionButton label="确认" variant="primary" size="lg" @click="emit('confirm', { structure, source })" />
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
/* 双组分隔（稿）：来源组下 padding 12；层级组上 padding 12 + 顶线 1px 亮色/半透明-3 */
.option-group.divided {
  padding-bottom: var(--n-space-12);
}
.option-group.top-line {
  padding-top: var(--n-space-12);
  border-top: 1px solid var(--n-l-a3);
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
