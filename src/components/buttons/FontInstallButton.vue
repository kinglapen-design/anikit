<script setup lang="ts">
// 字体安装按钮——docs/UI 重构/Design Token/组件/按钮/字体安装按钮：
// 18px 圆钮（通用/四级底 + 通用/六级 1px 内描边），位于字体卡片 / 预览标题右上角；
// hover = 描边升 亮色/四级（2026-08-29 用户定，稿原无 hover 态）；
// 选中 = 品牌/常态底 + 亮色/半透明-3 描边（用户改定，稿 prose 原写半透明-1）；过渡 150ms（--n-dur-150）。
withDefaults(defineProps<{ selected?: boolean; forceState?: "" | "hover" }>(), {
  selected: false,
  forceState: "",
});
defineEmits<{ click: [e: MouseEvent] }>();
</script>

<template>
  <button
    class="fi-btn"
    :class="[{ selected }, forceState ? `force-${forceState}` : '']"
    aria-label="安装字体"
    @click="$emit('click', $event)"
  />
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
.fi-btn:hover,
.fi-btn.force-hover {
  box-shadow: inset 0 0 0 1px var(--n-l4);
}
.fi-btn.selected,
.fi-btn.selected:hover,
.fi-btn.selected.force-hover {
  background: var(--n-brand);
  box-shadow: inset 0 0 0 1px var(--n-l-a3);
}
</style>
