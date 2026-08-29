<script setup lang="ts">
// 字体安装按钮——docs/UI 重构/Design Token/组件/按钮/字体安装按钮：
// 18px 圆钮（通用/四级底 + 通用/六级 1px 内描边），位于字体卡片 / 预览标题右上角；
// 选中 = 品牌/常态底 + 亮色/半透明-1 描边。稿无 hover 态，不做。
withDefaults(defineProps<{ selected?: boolean }>(), { selected: false });
defineEmits<{ click: [e: MouseEvent] }>();
</script>

<template>
  <button class="fi-btn" :class="{ selected }" aria-label="安装字体" @click="$emit('click', $event)" />
</template>

<style scoped>
.fi-btn {
  width: 18px;
  height: 18px;
  padding: 0;
  flex: none;
  border-radius: 50%;
  background: var(--n-g4);
  box-shadow: inset 0 0 0 1px var(--n-g6);
  transition:
    background var(--n-dur-150) var(--n-ease-in-out),
    box-shadow var(--n-dur-150) var(--n-ease-in-out);
}
.fi-btn.selected {
  background: var(--n-brand);
  box-shadow: inset 0 0 0 1px var(--n-l-a1);
}
</style>
