<script setup lang="ts">
// 功能模态窗共享壳——docs/06_UI重构/Design Token/布局/模态窗口/（创建资源库 / 合并资源库 /
// 导入弹窗 / 移动 四稿，2026-09-06 入库；2026-09-08 落地前两件）：
// 壳口径与 modals/ConfirmModal 同款——遮罩 黑色-50%（--n-mask，点击不关闭）、通用/三级底 +
// 1px 通用/五级真 border（弹窗真 border 口径）+ 大阴影 + 圆角 8 + padding 24 列 gap 24 +
// 弹出缩放淡入 150ms；标题 18/26 Bold 亮色/一级；宽度随稿（prop）。
// 按钮：ActionButton lg 恒宽 128（四稿钮值 128×38——区别于 ConfirmModal 警告稿 120×38，
// 各按各稿；次左主右）。按钮行右对齐 gap 8（actions 插槽）；合并稿「提示+按钮」混排行在
// default 插槽内自排（.bm-modal 全域 :deep 罩 128 宽，不限 actions）。
withDefaults(defineProps<{ title: string; width: number }>(), {});
</script>

<template>
  <div class="bm-mask">
    <div class="bm-modal" :style="{ width: `${width}px` }" role="dialog" :aria-label="title">
      <h2 class="bm-title">{{ title }}</h2>
      <slot />
      <div v-if="$slots.actions" class="bm-actions"><slot name="actions" /></div>
    </div>
  </div>
</template>

<style scoped>
.bm-mask {
  position: fixed;
  inset: 0;
  z-index: 1800;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--n-mask);
}
.bm-modal {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-24);
  padding: var(--n-space-24);
  box-sizing: border-box;
  border: 1px solid var(--n-g5);
  border-radius: var(--n-radius-8);
  background: var(--n-g3);
  box-shadow: var(--n-shadow-lg);
  animation: bm-pop var(--n-dur-150) var(--n-ease-in-out);
}
@keyframes bm-pop {
  from {
    opacity: 0;
    transform: scale(0.96);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}
.bm-title {
  font-size: var(--n-fs-title);
  line-height: var(--n-lh-title);
  font-weight: var(--n-fw-bold);
  color: var(--n-l1);
}
.bm-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--n-space-8);
}
/* 四稿按钮定宽 128×38（ActionButton lg 宽 100% 随父——模态窗内恒 128） */
.bm-modal :deep(.act-btn) {
  width: 128px;
  flex: none;
}
</style>
