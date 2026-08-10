<script setup lang="ts">
// 通用确认弹窗（6.3）：轻量一次性确认——警示前置（文件管理器打开）/ 可撤销操作说明
//（删除文件夹）/ 不可逆操作二次确认（彻底删除，danger 主按钮错误色）。
// 由 selection.confirmPrompt 驱动，宿主 App.vue；样式照 RecycleConfirmDialog（del-* 口径）。
import IconButton from "./IconButton.vue";
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
  <div v-if="selection.confirmPrompt" class="del-mask">
    <div class="del-card">
      <header class="del-titlebar">
        <h2 class="del-title">{{ selection.confirmPrompt.title }}</h2>
        <IconButton name="window-close" title="关闭" @click="onCancel" />
      </header>
      <div class="del-body">
        <p class="del-text">{{ selection.confirmPrompt.body }}</p>
      </div>
      <footer class="del-footer">
        <button class="btn secondary" @click="onCancel">取消</button>
        <button class="btn primary" :class="{ danger: selection.confirmPrompt.danger }" @click="onConfirm">
          {{ selection.confirmPrompt.confirmLabel ?? "确认" }}
        </button>
      </footer>
    </div>
  </div>
</template>

<style scoped>
/* 遮罩 / 卡片 / 标题栏 / 按钮：del-* 口径（同 RecycleConfirmDialog） */
.del-mask {
  position: fixed;
  inset: 0;
  z-index: 2200;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--c1-80);
}
.del-card {
  display: flex;
  flex-direction: column;
  width: 420px;
  border-radius: var(--radius-8);
  background: var(--c3);
  box-shadow:
    inset 0 0 0 1px var(--border-1),
    var(--shadow-pop);
}
.del-titlebar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 44px;
  padding: 0 var(--space-8) 0 var(--space-16);
  flex: none;
}
.del-title {
  font-size: var(--text-title-size);
  line-height: var(--text-title-line);
  font-weight: var(--text-title-weight);
  color: var(--font-icon-1);
}
.del-body {
  padding: var(--space-8) var(--space-16);
}
.del-text {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--font-icon-2);
}
.del-footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-8);
  padding: 0 var(--space-16) var(--space-16);
}
/* 按钮（01_按钮#07：主要 / 次要） */
.btn {
  height: 38px;
  padding: var(--space-8) var(--space-16);
  border-radius: var(--radius-6);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
}
.btn.primary {
  background: var(--brand-1);
  color: var(--font-icon-1);
}
.btn.primary:hover {
  background: var(--brand-2);
}
.btn.primary.danger {
  background: var(--error);
}
.btn.secondary {
  background: var(--c5);
  color: var(--font-icon-1);
}
.btn.secondary:hover {
  background: var(--border-1);
}
</style>
