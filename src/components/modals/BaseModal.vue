<script setup lang="ts">
// 功能模态窗共享壳——docs/06_UI重构/Design Token/布局/模态窗口/（创建资源库 / 合并资源库 /
// 导入弹窗 / 移动 四稿，2026-09-06 入库；创建·合并 2026-09-08 落地，导入·移动同日换代落地）：
// 壳口径与 modals/ConfirmModal 同款——遮罩 黑色-50%（--n-mask，点击不关闭）、通用/三级底 +
// 1px 通用/五级真 border（弹窗真 border 口径）+ 大阴影 + 圆角 8 + padding 24 列 gap 24 +
// 弹出缩放淡入 150ms；标题 18/26 Bold 亮色/一级；宽度随稿（prop，border-box 含 1px 边）。
// framed 变体（移动稿）：壳 padding 0 / 列 gap 12；标题栏 54 高 = padding 12/12/12/24 + 底 1px
// 通用/五级通栏 + 标题 + 关闭钮（buttons/IconButton plain close-large——稿 30×30 圆角 6、
// 图标 18 盒 12 矢量与 close-large 实寸一致；图标为烘焙二级原值，hover 仅出半透明-3 底）；
// 内容各段（搜索 / 树形容器）自承 padding 0 24。
// 按钮：ActionButton lg **宽自适应内容 + 左右 48px**（2026-09-13 实机走查用户定稿值——
// 四稿原 128×38 定宽与 ConfirmModal 120×38 一并废除；次左主右）。按钮行右对齐 gap 8
//（actions 插槽）；合并稿「提示+按钮」混排行在 default 插槽内自排（.bm-modal 全域 :deep 罩，不限 actions）。
import IconButton from "../buttons/IconButton.vue";

withDefaults(defineProps<{ title: string; width: number; framed?: boolean }>(), { framed: false });
const emit = defineEmits<{ close: [] }>();
</script>

<template>
  <div class="bm-mask">
    <div class="bm-modal" :class="{ framed }" :style="{ width: `${width}px` }" role="dialog" :aria-label="title">
      <!-- framed（移动稿）：标题栏 54 高带底线与关闭 X；非 framed = 纯标题（创建/合并/导入稿） -->
      <div v-if="framed" class="bm-titlebar">
        <h2 class="bm-title">{{ title }}</h2>
        <IconButton name="close-large" title="关闭" @click="emit('close')" />
      </div>
      <h2 v-else class="bm-title">{{ title }}</h2>
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
/* framed 变体（移动稿）：壳 padding 0 / 列 gap 12，内容段自承 padding */
.bm-modal.framed {
  padding: 0;
  gap: var(--n-space-12);
}
/* framed 标题栏（移动稿）：54 高 / padding 12 12 12 24 / 底 1px 通用/五级通栏 */
.bm-titlebar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--n-space-12);
  box-sizing: border-box;
  height: 54px;
  padding: var(--n-space-12) var(--n-space-12) var(--n-space-12) var(--n-space-24);
  border-bottom: 1px solid var(--n-g5);
  flex: none;
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
/* 模态窗按钮：宽度自适应内容 + 左右 48px（2026-09-13 实机走查用户定稿值，废 128 定宽；
   ActionButton lg 宽 100% 随父——模态窗内随内容） */
.bm-modal :deep(.act-btn) {
  width: auto;
  padding: var(--n-space-8) 48px;
  flex: none;
}
</style>
