<script setup lang="ts">
// 确认弹窗（警告与错误）——docs/UI 重构/Design Token/组件/模态窗口/弹窗.md：
// 窗口 520 宽、padding 24、列 gap 24，通用/三级底 + 1px 通用/五级真 border
//（弹窗口径：真 border 盒模型内缩，禁 inset box-shadow 画框）+ 大阴影 + 圆角 8；
// 遮罩 = 黑色-50%（--n-mask，2026-09-01 用户定），点击不关闭（关闭只走取消/确认）。
// 标题 18/26 Bold 亮色/一级；正文 13/18 亮色/三级（默认 slot）；
// 按钮行右对齐 gap 8（次左主右，2026-08-31 口径）：按钮 120×38 圆角 8 按钮文字-大——
// 取消 = 通用/五级底 + 立体感-通用 + 亮色/二级；
// 确认 = kind 色底 + 立体感-主要按钮 + 亮色/一级
//（warning 状态/警告 / danger 状态/禁止 / brand 品牌/常态——稿：不危险的提示换品牌色）。
// 弹出动画 = 缩放+淡入 150ms（就近套旧 AppModal modal-pop 先例）。按钮 hover 态稿未给，待补稿。
const props = withDefaults(
  defineProps<{
    title: string;
    kind?: "warning" | "danger" | "brand";
    confirmLabel?: string;
    cancelLabel?: string;
  }>(),
  { kind: "warning", confirmLabel: "确认", cancelLabel: "取消" },
);
const emit = defineEmits<{ confirm: []; cancel: [] }>();
</script>

<template>
  <div class="kit-mask">
    <div class="kit-modal" role="dialog" :aria-label="props.title">
      <h2 class="title">{{ title }}</h2>
      <div class="body"><slot /></div>
      <div class="actions">
        <button class="btn cancel" @click="emit('cancel')">{{ cancelLabel }}</button>
        <button class="btn confirm" :class="[kind]" @click="emit('confirm')">{{ confirmLabel }}</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.kit-mask {
  position: fixed;
  inset: 0;
  z-index: 1800;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--n-mask);
}
.kit-modal {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-24);
  width: 520px;
  padding: var(--n-space-24);
  box-sizing: border-box;
  border: 1px solid var(--n-g5);
  border-radius: var(--n-radius-8);
  background: var(--n-g3);
  box-shadow: var(--n-shadow-lg);
  animation: kit-modal-pop var(--n-dur-150) var(--n-ease-in-out);
}
@keyframes kit-modal-pop {
  from {
    opacity: 0;
    transform: scale(0.96);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}
.title {
  font-size: var(--n-fs-title);
  line-height: var(--n-lh-title);
  font-weight: var(--n-fw-bold);
  color: var(--n-l1);
}
.body {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l3);
}
.actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--n-space-8);
}
.btn {
  width: 120px;
  height: 38px;
  padding: var(--n-space-8) 0;
  border-radius: var(--n-radius-8);
  font-size: var(--n-fs-btn-lg);
  line-height: var(--n-lh-btn-lg);
  font-weight: var(--n-fw-bold);
  flex: none;
}
.btn.cancel {
  background: var(--n-g5);
  box-shadow: var(--n-effect-raised);
  color: var(--n-l2);
}
.btn.confirm {
  color: var(--n-l1);
  box-shadow: var(--n-effect-raised-primary);
}
.btn.confirm.warning {
  background: var(--n-warning);
}
.btn.confirm.danger {
  background: var(--n-danger);
}
.btn.confirm.brand {
  background: var(--n-brand);
}
</style>
