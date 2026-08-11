<script setup lang="ts">
// 图标按钮（figma-css 01_按钮 1.1 小 / 1.3 大 / 1.2 红色删除）：
// 大 30×30 padding6 radius6 图标18（默认）/ 小 20×20 padding4 radius4 图标12（small）；
// hover 叠 暗-悬停 罩层；active（持久选中）叠 亮-选中 罩层；
// filled = 底色装饰（侧边栏 C5 底 + 暗-悬停边框，01_按钮 1.6）；
// danger = 窗口控制关闭（hover 错误色）；deleteRed = 红色删除（20px 圆红底，hover 旋转 180°、离开逆时针恢复）。
import { computed } from "vue";
import AppIcon from "./AppIcon.vue";
import type { TipValue } from "../lib/tip";

const props = withDefaults(
  defineProps<{
    name: string;
    small?: boolean;
    filled?: boolean;
    danger?: boolean;
    deleteRed?: boolean;
    active?: boolean;
    disabled?: boolean;
    title?: string | TipValue; // 对象形态 = 标题 + 快捷键副文本（如 { text: "新增", keys: "Ctrl+N" }）
  }>(),
  { small: false, filled: false, danger: false, deleteRed: false, active: false, disabled: false, title: undefined },
);
defineEmits<{ click: [e: MouseEvent] }>();

const iconName = computed(() => (props.deleteRed ? "delete-red" : props.name));
// delete-red.svg = 20×20 完整按钮图形（圆底+边框+横杠），铺满按钮；其余大 18 / 小 12
const iconSize = computed(() => (props.deleteRed ? 20 : props.small ? 12 : 18));
</script>

<template>
  <button
    class="icon-btn"
    :class="{ small, filled, danger, 'delete-red': deleteRed, active }"
    :disabled="disabled"
    v-tip="title"
    @click="$emit('click', $event)"
  >
    <AppIcon :name="iconName" :size="iconSize" />
  </button>
</template>

<style scoped>
.icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  padding: var(--space-6);
  border-radius: var(--radius-6);
  color: var(--text-normal);
  flex: none;
  transition: background 0.15s ease;
}
/* 小（20×20，圆角 4，图标 12，标签档色） */
.icon-btn.small {
  width: 20px;
  height: 20px;
  padding: var(--space-4);
  border-radius: var(--radius-4);
  color: var(--text-label);
}
/* hover 叠 暗-悬停 罩层（gradient 叠加法，保留底色） */
.icon-btn:hover {
  background: linear-gradient(var(--line-hover), var(--line-hover));
}
/* 持久选中叠 亮-选中 罩层 */
.icon-btn.active,
.icon-btn.active:hover {
  background: linear-gradient(var(--line-active), var(--line-active));
}
/* 底色装饰（01_按钮 1.6 侧边栏）：C5 底 + 暗-悬停边框，hover 再叠一层暗-悬停 */
.icon-btn.filled {
  background: var(--c5);
  box-shadow: inset 0 0 0 1px var(--line-hover);
}
.icon-btn.filled:hover {
  background: linear-gradient(var(--line-hover), var(--line-hover)), var(--c5);
}
/* 窗口控制关闭：hover 错误色 */
.icon-btn.danger:hover {
  background: var(--error);
}
/* 红色删除（01_按钮 1.2）：20px 圆红底恒显（SVG 自带），hover 图标顺时针旋转 180° + 叠暗-悬停罩层，离开逆时针恢复 */
.icon-btn.delete-red {
  width: 20px;
  height: 20px;
  padding: 0;
  border-radius: var(--radius-circle);
  background: transparent;
  position: relative;
}
.icon-btn.delete-red::after {
  content: "";
  position: absolute;
  inset: 0;
  border-radius: var(--radius-circle);
  background: transparent;
  transition: background 0.15s ease;
}
.icon-btn.delete-red:hover::after {
  background: var(--line-hover);
}
.icon-btn.delete-red :deep(svg) {
  transition: transform 0.25s ease;
}
.icon-btn.delete-red:hover :deep(svg) {
  transform: rotate(180deg);
}
.icon-btn:disabled {
  cursor: default;
  background: none;
  box-shadow: none;
  opacity: 0.5;
}
</style>
