<script setup lang="ts">
// 右键菜单 item（7.1 二十四轮 2026-09-01 按新稿 组件/上下文菜单 重写，旧 figma-css 04 稿废除）：
// 高 26 / padding 4 8 / 圆角 6；标题 = 通用/正文 13/18 亮色/二级；hover 底 = 亮色/半透明-3；
// 禁用 = 亮色/禁用档不响应 hover/点击。
// 间距（2026-09-01 二十五轮用户定）：名称→36px→右侧件（flex gap 36 = 最小间距；
// 容器宽由最长 item 内容自适应，其余项 .main flex:1 拉伸把右件顶到右缘对齐）。
// 右侧件（space-between，互斥）：
// - 快捷键纯文本（通用/数字 Fira Mono 500 12/16 亮色/三级，原串显示 "Ctrl+Enter"——
//   旧 KbdBadge 徽章组形态与"词键转小写"逻辑随新稿废除）；
// - 对勾（checkable，18 盒 check 12）：常态不显示 / 行 hover 显 亮色/三级 / 选中恒显 亮色/一级
//   （恒渲染 opacity 切换——v-if 移除会改 flex 基线致行高抖动，踩坑 026）；
// - 展开箭头（arrow，expand 14 盒 rotate(-90deg) 朝右，亮色/三级）；
// - 色块行（swatches，「更改颜色」稿）：16 圆块 gap 4 + 1px 亮色/半透明-1 描边（ColorSwatchItem
//   口径），hover 单块放大 125% 提层（2026-08-29 用户定），当前色 1.5px 亮色/一级外环（不占布局）；
//   色块点击 @pick（不关菜单，连续试色场景），整行点击无动作。
// trailingIcon = 标题右侧小图标 12（3.5 分组菜单「上移/下移」口径保留，亮色/三级，禁用随父色）。
import AppIcon from "./AppIcon.vue";

export interface SwatchColor {
  tint: string | null;
  hex: string;
  label: string;
}

const props = withDefaults(
  defineProps<{
    label: string;
    trailingIcon?: string;
    trailingIconClass?: string; // 尾图标附加类（rot-180 = 源图形朝上转朝下）
    shortcut?: string;
    arrow?: boolean;
    checkable?: boolean;
    checked?: boolean;
    disabled?: boolean; // 禁用态：不响应 hover / 点击
    swatches?: { colors: SwatchColor[]; current: string | null }; // 内嵌色块行（与 arrow/shortcut 互斥）
    forceState?: "" | "hover"; // 预览页强制态：静态验收 hover（实际交互仅悬停触发）
  }>(),
  {
    trailingIcon: undefined,
    trailingIconClass: undefined,
    shortcut: undefined,
    arrow: false,
    checkable: false,
    checked: false,
    disabled: false,
    swatches: undefined,
    forceState: "",
  },
);
const emit = defineEmits<{ select: []; pick: [tint: string | null] }>();

function onRowClick(): void {
  if (props.swatches) return; // 色块行整行点击无动作（仅色块可点）
  emit("select");
}
</script>

<template>
  <button class="menu-item" :class="[{ checked }, forceState ? `force-${forceState}` : '']" :disabled="disabled" @click="onRowClick">
    <span class="main">
      <span class="label">{{ label }}</span>
      <AppIcon v-if="trailingIcon" :name="trailingIcon" :size="12" class="trail" :class="trailingIconClass" />
    </span>
    <span v-if="swatches" class="swatches">
      <button
        v-for="s in swatches.colors"
        :key="s.tint ?? 'default'"
        class="sw"
        :class="{ cur: s.tint === swatches.current }"
        :style="{ background: s.hex }"
        :aria-label="s.label"
        @click.stop="emit('pick', s.tint)"
      />
    </span>
    <span v-else-if="shortcut" class="shortcut">{{ shortcut }}</span>
    <span v-else-if="checkable" class="check-slot">
      <AppIcon name="check" :size="12" />
    </span>
    <AppIcon v-else-if="arrow" name="expand" :size="14" class="arrow" />
  </button>
</template>

<style scoped>
.menu-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--n-space-36); /* 名称→右件最小间距（二十五轮用户定）；无右件时单子元素 gap 不生效 */
  width: 100%;
  height: 26px;
  padding: var(--n-space-4) var(--n-space-8);
  box-sizing: border-box;
  border-radius: var(--n-radius-6);
  color: var(--n-l2);
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  transition: background var(--n-dur-150) var(--n-ease-in-out);
}
/* hover 底 = 亮色/半透明-3 */
.menu-item:hover,
.menu-item.force-hover {
  background: var(--n-l-a3);
}
/* 禁用态（亮色/禁用档）：不响应 hover / 点击 */
.menu-item:disabled {
  color: var(--n-l-disabled);
  cursor: default;
}
.menu-item:disabled:hover {
  background: none;
}
.main {
  display: flex;
  align-items: center;
  gap: var(--n-space-6);
  flex: 1;
  min-width: 0;
}
.label {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
/* 标题右侧小图标（12，三级色；禁用态随父色） */
.trail {
  flex: none;
  color: var(--n-l3);
}
.menu-item:disabled .trail {
  color: inherit;
}
.trail.rot-180 :deep(svg) {
  transform: rotate(180deg);
}
/* 快捷键：纯文本 通用/数字（Fira Mono 500 12/16）三级色 */
.shortcut {
  font-family: var(--n-font-mono);
  font-weight: var(--n-fw-medium);
  font-size: var(--n-fs-num);
  line-height: var(--n-lh-num);
  color: var(--n-l3);
  flex: none;
}
/* 对勾 18 盒（check 12）：常态隐 / hover 显三级 / 选中恒显一级 */
.check-slot {
  width: 18px;
  height: 18px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: none;
  color: var(--n-l3);
  opacity: 0;
  transition: opacity var(--n-dur-150) var(--n-ease-in-out);
}
.menu-item:hover .check-slot,
.menu-item.force-hover .check-slot {
  opacity: 1;
}
.menu-item.checked .check-slot {
  color: var(--n-l1);
  opacity: 1;
}
/* 二级菜单提示箭头：expand 14 盒旋转朝右，三级色 */
.arrow {
  flex: none;
  color: var(--n-l3);
  transform: rotate(-90deg);
}
/* 色块行（「更改颜色」稿）：16 圆块 gap 4、1px 半透明-1 描边；hover 单块 125% 提层；当前色 1.5px 一级外环 */
.swatches {
  display: inline-flex;
  align-items: center;
  gap: var(--n-space-4);
  flex: none;
}
.sw {
  box-sizing: border-box;
  width: 16px;
  height: 16px;
  flex: none;
  border-radius: 50%;
  border: 1px solid var(--n-l-a1);
  padding: 0;
  cursor: pointer;
  transition: transform var(--n-dur-150) var(--n-ease-in-out);
}
/* 放大块提层：避免被 DOM 后继块盖住边缘（ColorSwatchItem 同口径） */
.sw:hover {
  position: relative;
  z-index: 1;
  transform: scale(1.25);
}
.sw.cur {
  box-shadow: 0 0 0 1.5px var(--n-l1);
}
</style>
