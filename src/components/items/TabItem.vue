<script setup lang="ts">
// 标签页 item——docs/UI 重构/Design Token/组件/item/标签页 item/ + 布局/中间面板-标签栏
// 选中：通用/一级底 + 圆角 8 8 0 0 + 与中间面板无缝衔接（底部两侧反弧肩膀，radial-gradient 反圆角）；
// 未选中 hover：四周 4px 间距的 6px 圆角矩形罩（亮色/半透明-3，2026-08-28 用户定）；关闭钮常态仅选中显示。
import AppIcon from "../AppIcon.vue";

withDefaults(
  defineProps<{
    label: string;
    selected?: boolean;
    closable?: boolean; // 锁定页等业务态可不显示关闭钮
    forceState?: "" | "hover" | "selected"; // 预览页强制态
  }>(),
  { selected: false, closable: true, forceState: "" },
);

const emit = defineEmits<{ click: []; close: [] }>();
</script>

<template>
  <div
    class="tab"
    :class="[{ selected }, forceState ? `force-${forceState}` : '']"
    @click="emit('click')"
  >
    <span class="label">{{ label }}</span>
    <button v-if="closable" class="x" @click.stop="emit('close')">
      <AppIcon name="close-small" :size="14" />
    </button>
  </div>
</template>

<style scoped>
.tab {
  position: relative;
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
  height: 38px;
  padding: 0 var(--n-space-16);
  min-width: 0;
  border-radius: var(--n-radius-8) var(--n-radius-8) 0 0;
  color: var(--n-l3);
  cursor: pointer;
}
.label {
  position: relative;
  z-index: 1;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
  flex: 1;
}
.x {
  position: relative;
  z-index: 1;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 14px;
  height: 14px;
  flex: none;
  padding: 0;
  color: var(--n-l4);
  opacity: 0;
  transition:
    color var(--n-dur-100) var(--n-ease-in-out),
    opacity var(--n-dur-100) var(--n-ease-in-out);
}
.x:hover {
  color: var(--n-l2);
}
/* 未选中 hover：四周 4px 间距的 6px 圆角矩形罩（亮色/半透明-3） */
.tab:not(.selected):not(.force-selected)::after {
  content: "";
  position: absolute;
  inset: var(--n-space-4);
  border-radius: var(--n-radius-6);
  background: transparent;
  transition: background var(--n-dur-100) var(--n-ease-in-out);
  pointer-events: none;
}
.tab:not(.selected):not(.force-selected):hover::after,
.tab.force-hover::after {
  background: var(--n-l-a3);
}
.tab:hover .x,
.tab.force-hover .x {
  opacity: 1;
}
/* 选中：通用/一级底与中间面板同色无缝；底部两侧反弧肩膀（8px 反圆角方块，圆心朝外上角） */
.tab.selected,
.tab.force-selected {
  background: var(--n-g1);
  color: var(--n-l1);
}
.tab.selected::before,
.tab.selected::after,
.tab.force-selected::before,
.tab.force-selected::after {
  content: "";
  position: absolute;
  bottom: 0;
  width: 8px;
  height: 8px;
  pointer-events: none;
}
.tab.selected::before,
.tab.force-selected::before {
  left: -8px;
  background: radial-gradient(circle at 0 0, transparent 7.5px, var(--n-g1) 8px);
}
.tab.selected::after,
.tab.force-selected::after {
  right: -8px;
  background: radial-gradient(circle at 100% 0, transparent 7.5px, var(--n-g1) 8px);
}
.tab.selected .x,
.tab.force-selected .x {
  opacity: 1;
}
</style>
