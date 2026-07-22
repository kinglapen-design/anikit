<script setup lang="ts">
// 导入弹窗（03_UI 布局/09_导入弹窗）：三态标题（导入文件 / 导入文件夹 / 导入文件 / 文件夹）
// + 层级单选组（仅含文件夹时显示）+ 来源单选组 + 按钮组；默认剔除层级 + 复制。
// 选中"剪切原始文件"时提示文字变警示（PRD 05-01：弹窗文案必须明示"源文件将被移走"）。
// 确认只发事件：导入走后台任务 + Channel 流式（异步不阻塞 UI），弹窗立即关闭。
import { computed, ref } from "vue";
import AppCheckbox from "./AppCheckbox.vue";
import type { ImportDraft } from "../stores/content";

const props = defineProps<{ draft: ImportDraft }>();
const emit = defineEmits<{
  confirm: [opts: { structure: "flatten" | "keep"; source: "copy" | "move" }];
  cancel: [];
}>();

const structure = ref<"flatten" | "keep">("flatten");
const source = ref<"copy" | "move">("copy");

const title = computed(
  () => ({ files: "导入文件", folder: "导入文件夹", mixed: "导入文件 / 文件夹" })[props.draft.kind],
);
const sourceTip = computed(() =>
  source.value === "move"
    ? "源文件将被移走，导入后原位置不再保留"
    : "选择复制一份文件导入，或者剪切原始文件导入",
);
</script>

<template>
  <div class="import-mask">
    <!-- 选项态（09：368 宽 / padding 16 / gap 16；files 态去层级组高度自收缩） -->
    <div class="import-card">
      <h2 class="title">{{ title }}</h2>
      <template v-if="draft.kind !== 'files'">
        <p class="tip">剔除原始层级导入所有文件，或者保留层级导入</p>
        <div class="radio-row">
          <AppCheckbox
            tone="dialog"
            label="剔除原始层级"
            :model-value="structure === 'flatten'"
            @update:model-value="structure = 'flatten'"
          />
          <AppCheckbox
            tone="dialog"
            label="保留原始层级"
            :model-value="structure === 'keep'"
            @update:model-value="structure = 'keep'"
          />
        </div>
      </template>
      <p class="tip" :class="{ warn: source === 'move' }">{{ sourceTip }}</p>
      <div class="radio-row">
        <AppCheckbox
          tone="dialog"
          label="复制一份文件"
          :model-value="source === 'copy'"
          @update:model-value="source = 'copy'"
        />
        <AppCheckbox
          tone="dialog"
          label="剪切原始文件"
          :model-value="source === 'move'"
          @update:model-value="source = 'move'"
        />
      </div>
      <div class="actions">
        <button class="btn primary" @click="emit('confirm', { structure, source })">确认导入</button>
        <button class="btn secondary" @click="emit('cancel')">取消</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* 遮罩：与 LibrarySetupModal 一致（C1-80） */
.import-mask {
  position: absolute;
  inset: 0;
  z-index: 200;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--c1-80);
}
/* 总容器（09）：C5 / 1px C4-15（inset）/ 圆角 8 / --shadow-pop */
.import-card {
  box-sizing: border-box;
  width: 368px;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  padding: var(--space-16);
  gap: var(--space-16);
  background: var(--c5);
  box-shadow:
    var(--shadow-pop),
    inset 0 0 0 1px var(--c4-15);
  border-radius: var(--radius-8);
}
.title {
  margin: 0;
  font-size: var(--text-title-size);
  line-height: var(--text-title-line);
  font-weight: var(--text-title-weight);
  color: var(--c4);
}
/* 提示文字：副文本 C4-40；剪切选中时警示（--warning） */
.tip {
  margin: 0;
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--c4-40);
}
.tip.warn {
  color: var(--warning);
}
/* 单选框容器（09）：横向 / 间距 16 */
.radio-row {
  display: flex;
  flex-direction: row;
  gap: var(--space-16);
}
/* 按钮组（09）：确认导入（主要）/ 取消（非主要），各 160×34，间距 16 */
.actions {
  display: flex;
  flex-direction: row;
  gap: var(--space-16);
  width: 100%;
}
.btn {
  display: flex;
  justify-content: center;
  align-items: center;
  padding: 8px 16px;
  gap: var(--space-6);
  height: 38px;
  border: none;
  border-radius: var(--radius-6);
  cursor: pointer;
  font-family: var(--font-family);
  font-size: var(--text-title-size);
  line-height: var(--text-title-line);
  font-weight: var(--text-title-weight);
  color: var(--c4);
}
.btn.primary {
  background: var(--brand-1);
}
.btn.primary:hover {
  background: var(--brand-2);
}
.btn.secondary {
  background: var(--c4-7);
}
.btn.secondary:hover {
  background: var(--c4-15);
}
.btn:disabled {
  cursor: default;
  opacity: 0.6;
}
</style>
