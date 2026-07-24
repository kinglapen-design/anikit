<script setup lang="ts">
// 序列帧弹窗（03_UI 布局/12_序列帧弹窗，4.1）：09 导入弹窗确认后、后端 detect_sequences
// 命中时追加弹出。样式直接复用 09 的 card/tip/radio-row/actions 规格（无层级/来源排——
// 来源已在 09 选过）。确认发打包/分散模式；取消 = 放弃整个导入。
import { ref } from "vue";
import AppCheckbox from "./AppCheckbox.vue";

const emit = defineEmits<{
  confirm: [mode: "pack" | "scatter"];
  cancel: [];
}>();

const mode = ref<"pack" | "scatter">("pack");
</script>

<template>
  <div class="seq-mask">
    <div class="seq-card">
      <h2 class="title">监测到序列帧</h2>
      <p class="tip">可将序列帧打包为动画预览图，菜单选项中依旧可查看单个文件</p>
      <div class="radio-row">
        <AppCheckbox
          tone="dialog"
          label="打包导入预览"
          :model-value="mode === 'pack'"
          @update:model-value="mode = 'pack'"
        />
        <AppCheckbox
          tone="dialog"
          label="分散图片导入"
          :model-value="mode === 'scatter'"
          @update:model-value="mode = 'scatter'"
        />
      </div>
      <div class="actions">
        <button class="btn primary" @click="emit('confirm', mode)">确认导入</button>
        <button class="btn secondary" @click="emit('cancel')">取消</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* 遮罩：与 ImportDialog 一致（C1） */
.seq-mask {
  position: absolute;
  inset: 0;
  z-index: 200;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--c1-80);
}
/* 总容器（08_二级窗口通用规则，同 09）：宽度随最宽子项、上限 368；
   C3 / 1px border-1（inset）/ 圆角 8 / --shadow-pop */
.seq-card {
  box-sizing: border-box;
  width: fit-content;
  max-width: 368px;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  padding: var(--space-32);
  gap: var(--space-16);
  background: var(--c3);
  box-shadow:
    var(--shadow-pop),
    inset 0 0 0 1px var(--border-1);
  border-radius: var(--radius-8);
}
.title {
  margin: 0;
  font-size: var(--text-title-size);
  line-height: var(--text-title-line);
  font-weight: var(--text-title-weight);
  color: var(--font-icon-1);
}
/* 提示文字：副文本 font-icon-3；超 368 上限换行（08 通用规则） */
.tip {
  margin: 0;
  max-width: 100%;
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--font-icon-3);
  word-break: break-all;
}
/* 单选框容器（同 09）：横向 / 间距 16 */
.radio-row {
  display: flex;
  flex-direction: row;
  gap: var(--space-16);
}
/* 按钮组（08 通用规则：右对齐；01_按钮#07 规格同 09） */
.actions {
  display: flex;
  flex-direction: row;
  justify-content: flex-end;
  gap: var(--space-16);
  width: 100%;
}
.btn {
  display: flex;
  justify-content: center;
  align-items: center;
  padding: var(--space-8) var(--space-16);
  gap: var(--space-6);
  height: 38px;
  border: none;
  border-radius: var(--radius-6);
  cursor: pointer;
  font-family: var(--font-family);
  font-size: var(--text-title-size);
  line-height: var(--text-title-line);
  font-weight: var(--text-title-weight);
  color: var(--font-icon-1);
}
.btn.primary {
  background: var(--brand-1);
}
.btn.primary:hover {
  background: var(--brand-2);
}
.btn.secondary {
  background: var(--c5);
}
.btn.secondary:hover {
  background: var(--border-1);
}
</style>
