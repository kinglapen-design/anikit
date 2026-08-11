<script setup lang="ts">
// 右键菜单 item（figma-css 04_右键菜单 item）：padding 6/8、gap 12、圆角 6、高 33；
// 一级项：图标 18（常态档）+ 标题（正文常态）+ 快捷键徽章组 或 二级菜单提示箭头
// （tree-expand-arrow 12 rotate(-90) 朝右，figma 04 第 3 节）；
// 快捷键按 "+" 拆分为多个徽章（多字母 padding 2/4、单字母 2/6，见 KbdBadge）；无快捷键时隐藏；
// 二级项（checkable）：对勾占位保证标题列对齐；hover 叠 暗-悬停；禁用 = 亮-选中档色不响应 hover。
import { computed } from "vue";
import AppIcon from "./AppIcon.vue";
import KbdBadge from "./KbdBadge.vue";

const props = withDefaults(
  defineProps<{
    icon?: string;
    iconClass?: string; // 图标附加类（如 rot-90）
    iconData?: string; // 内联图片图标（data URL，16 居中于 18 盒——1.3 新建文档组系统软件图标）
    label: string;
    shortcut?: string;
    arrow?: boolean;
    checkable?: boolean;
    checked?: boolean;
    disabled?: boolean; // 禁用态：不响应 hover / 点击
  }>(),
  { icon: undefined, iconClass: undefined, iconData: undefined, shortcut: undefined, arrow: false, checkable: false, checked: false, disabled: false },
);
const emit = defineEmits<{ select: [] }>();

// 快捷键拆分："Ctrl+Shift+N" → ["Ctrl", "Shift", "N"]；
// 显示口径（新稿 02_布局 03/04/05/07，2026-08-11 走查）：词键小写（ctrl/shift/alt/enter/delete），
// 单字母键保持大写，中文键名（"菜单"）原样
const keys = computed(() =>
  props.shortcut ? props.shortcut.split("+").filter(Boolean).map((k) => (/^[A-Za-z]{2,}$/.test(k) ? k.toLowerCase() : k)) : [],
);
</script>

<template>
  <button class="menu-item" :class="{ checked }" :disabled="disabled" @click="emit('select')">
    <span class="main">
      <span v-if="checkable" class="check-slot">
        <AppIcon v-if="checked" name="check" :size="12" />
      </span>
      <span v-else-if="iconData" class="deco data-icon"><img :src="iconData" alt="" /></span>
      <AppIcon v-else-if="icon" :name="icon" :size="18" class="deco" :class="iconClass" />
      <span class="label">{{ label }}</span>
    </span>
    <span v-if="keys.length" class="shortcut">
      <KbdBadge v-for="k in keys" :key="k" :label="k" />
    </span>
    <AppIcon v-if="arrow" name="tree-expand-arrow" :size="12" class="arrow" />
  </button>
</template>

<style scoped>
.menu-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-12);
  width: 100%;
  padding: var(--space-6) var(--space-8);
  border-radius: var(--radius-6);
  color: var(--text-normal);
  transition: background 0.15s ease;
}
/* hover 叠 暗-悬停 罩层 */
.menu-item:hover {
  background: linear-gradient(var(--line-hover), var(--line-hover));
}
/* 禁用态（token：禁用 = 亮-选中档）：不响应 hover / 点击 */
.menu-item:disabled {
  color: var(--line-active);
  cursor: default;
}
.menu-item:disabled:hover {
  background: none;
}
/* 基础 item：图标 + 标题，间距 6 */
.main {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  flex: 1;
  min-width: 0;
}
.deco {
  flex: none;
}
/* 内联图片图标（data URL）：16 居中于 18 盒 */
.deco.data-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
}
.deco.data-icon img {
  width: 16px;
  height: 16px;
  display: block;
}
/* 图标旋转 90°（如"上下分屏"与左右分屏同图标的语义区分） */
.deco.rot-90 :deep(svg) {
  transform: rotate(90deg);
}
.check-slot {
  width: 18px;
  height: 18px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--text-normal);
  flex: none;
}
.label {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.shortcut {
  display: inline-flex;
  align-items: center;
  gap: var(--space-6);
  flex: none;
}
/* 二级菜单提示：展开箭头旋转朝右（figma 04 第 3 节），标签档 */
.arrow {
  flex: none;
  color: var(--text-label);
  transform: rotate(-90deg);
}
</style>
