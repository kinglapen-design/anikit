<script setup lang="ts">
// 操作按钮——docs/06_UI重构/Design Token/组件/按钮/ 主按钮-大/小-立体、文字按钮-大/小-立体、
// 文字按钮、图标+文字-立体 五稿合一：
// variant：primary = 品牌/常态底 + 立体感-主要按钮，文字亮色/一级（hover 品牌/提亮）；
//           solid = 通用/四级底 + 立体感-通用，文字亮色/二级（hover 通用/六级）；
//           ghost = 无填充菜单钮，文字亮色/三级（hover 亮色/半透明-3 底 + 文字升二级）；
//           cancel = 通用/五级底 + 立体感-通用，文字亮色/二级（2026-09-06 插件窗轮补——模态稿
//                    取消/关闭钮：ConfirmModal 取消、Animate 插件窗 关闭，按钮族原 solid 是四级底）；
//           danger = 状态/禁止底 + 立体感-主要按钮，文字亮色/一级（模态稿危险确认钮：
//                    ConfirmModal danger、Animate 插件窗 卸载）；
//           （cancel / danger 的 hover 态稿均未画——不做不自创，速查 §15/§24 口径）
// size：lg = 高 38 + 按钮文字-大 / md = 高 34 + 正文-加粗 / sm = 高 28 + 按钮文字（稿仅 ghost）；
// lg/md 为填充式按钮（宽 100%——figma padding-x 0 + flex-grow 的拉伸语境，宽度由父容器定），
// sm 自适应内容（padding 4/8）；圆角 lg/md = 8、sm = 6；
// icon = 前置图标（图标+文字-立体稿：icon 容器 18 实寸 / 亮色三级 / gap 6，hover 不变色）；
// hover 过渡 = 150ms 缓入缓出（--n-dur-150）。
import AppIcon from "../AppIcon.vue";

withDefaults(
  defineProps<{
    label: string;
    variant?: "primary" | "solid" | "ghost" | "cancel" | "danger";
    size?: "lg" | "md" | "sm";
    icon?: string;
    forceState?: "" | "hover";
    disabled?: boolean; // 禁用态 = 30% 透明度 + 不响应（2026-09-20 用户定，控件族通用口径，速查 §39②）
  }>(),
  { variant: "solid", size: "md", icon: undefined, forceState: "", disabled: false },
);
defineEmits<{ click: [e: MouseEvent] }>();
</script>

<template>
  <button
    class="act-btn"
    :class="[variant, size, forceState ? `force-${forceState}` : '']"
    :disabled="disabled"
    @click="$emit('click', $event)"
  >
    <AppIcon v-if="icon" :name="icon" :size="18" />
    <span class="lbl">{{ label }}</span>
  </button>
</template>

<style scoped>
.act-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--n-space-6);
  flex: none;
  border-radius: var(--n-radius-8);
  transition:
    background var(--n-dur-150) var(--n-ease-in-out),
    color var(--n-dur-150) var(--n-ease-in-out);
}
/* 禁用态 = 30% 透明度 + 不响应（控件族通用口径，速查 §39②） */
.act-btn:disabled {
  cursor: default;
  pointer-events: none;
  opacity: 0.3;
}
/* 尺寸档 */
.act-btn.lg,
.act-btn.md {
  width: 100%;
  padding: var(--n-space-8) 0;
}
.act-btn.lg {
  height: 38px;
  font-size: var(--n-fs-btn-lg);
  line-height: var(--n-lh-btn-lg);
  font-weight: var(--n-fw-bold);
}
.act-btn.md {
  height: 34px;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  font-weight: var(--n-fw-bold);
}
.act-btn.sm {
  height: 28px;
  padding: var(--n-space-4) var(--n-space-8);
  border-radius: var(--n-radius-6);
  font-size: var(--n-fs-btn);
  line-height: var(--n-lh-btn);
}
/* 变体 */
.act-btn.primary {
  background: var(--n-brand);
  box-shadow: var(--n-effect-raised-primary);
  color: var(--n-l1);
}
.act-btn.primary:hover,
.act-btn.primary.force-hover {
  background: var(--n-brand-bright);
}
.act-btn.solid {
  background: var(--n-g4);
  box-shadow: var(--n-effect-raised);
  color: var(--n-l2);
}
.act-btn.solid:hover,
.act-btn.solid.force-hover {
  background: var(--n-g6);
}
.act-btn.ghost {
  color: var(--n-l3);
}
.act-btn.ghost:hover,
.act-btn.ghost.force-hover {
  background: var(--n-l-a3);
  color: var(--n-l2);
}
/* cancel：模态稿取消/关闭钮（通用/五级底 + 立体感-通用 + 文字二级）；hover 稿未画不做 */
.act-btn.cancel {
  background: var(--n-g5);
  box-shadow: var(--n-effect-raised);
  color: var(--n-l2);
}
/* danger：模态稿危险确认钮（状态/禁止底 + 立体感-主要按钮 + 文字一级）；hover 稿未画不做 */
.act-btn.danger {
  background: var(--n-danger);
  box-shadow: var(--n-effect-raised-primary);
  color: var(--n-l1);
}
/* 前置图标恒 亮色/三级（图标+文字-立体稿，hover 不变色） */
.act-btn :deep(.app-icon) {
  color: var(--n-l3);
}
</style>
