<script setup lang="ts">
// 区块管理行（30 稿 item，2026-08-14 区块子系统）：高 52 / padding 12 0 / gap 8 / 底线 暗-悬停——
// 排序箭头 20×20（IconButton small，图标 = 箭头-小 chevron-small：下移在前、上移旋转 180；
// 可用态高亮档 / 禁用态 状态-禁用色）+「区块名称：」标签 + 输入框 28 高（AppInput small，
// 稿定 C1 底覆盖）+「默认图标：」标签 + 下拉 58×28 + 红色圆减号删除。
// 「目录」行（isDefault，2026-08-14 用户定）：仅删除 opacity 0 占位不可点（目录是各兜底落点）；
// 名称 / 图标 / 排序全部放开——锚定常量 id "dir"，改名无影响。
// 草稿式：行编辑只改草稿对象（AppInput v-model / setIcon 直改 row 属性，提交由弹窗统一组装）。
// 图标下拉本轮从简 = openContextMenu 列表（每项带图标预览 + 当前勾；3.3 换 05 稿网格面板时
// 仅需替换 openIconMenu 一处）。
import AppIcon from "./AppIcon.vue";
import AppInput from "./AppInput.vue";
import IconButton from "./IconButton.vue";
import { openContextMenu, type CtxEntry } from "../lib/contextMenus";
import { CUSTOM_ICONS } from "../lib/customIcons";
import type { BlockDraft } from "../lib/blockOps";

const props = defineProps<{
  row: BlockDraft;
  isFirst: boolean; // 首行：上箭头禁用
  isLast: boolean; // 末行：下箭头禁用
  isDefault: boolean; // 「目录」行：删除与上箭头占位隐藏
}>();
const emit = defineEmits<{ moveUp: []; moveDown: []; remove: [] }>();

// 默认图标下拉（58×28）：点击弹列表——"默认"（无图标，folder-closed 预览）+ 19 个自定义图标
function openIconMenu(e: MouseEvent): void {
  const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
  const entries: CtxEntry[] = [
    { label: "默认", icon: "folder-closed", checked: props.row.icon === null, action: () => (props.row.icon = null) },
    ...CUSTOM_ICONS.map((ic) => ({
      label: ic.label,
      icon: ic.name,
      checked: props.row.icon === ic.name,
      action: () => (props.row.icon = ic.name),
    })),
  ];
  openContextMenu({ clientX: rect.left, clientY: rect.bottom + 4 }, entries);
}
</script>

<template>
  <div class="bm-row" :class="{ default: isDefault }">
    <!-- 排序箭头（30 稿：图标 = 箭头-小 chevron-small；下移在前、上移在后。
         chevron-small 源图形朝上（实机渲染核对）——上移不旋转、下移旋转 180） -->
    <IconButton small name="chevron-small" class="sort-arrow arrow-down" :disabled="isLast" title="下移" @click="emit('moveDown')" />
    <IconButton small name="chevron-small" class="sort-arrow arrow-up" :disabled="isFirst" title="上移" @click="emit('moveUp')" />
    <span class="row-label">区块名称：</span>
    <AppInput v-model="row.name" size="small" class="name-input" placeholder="区块名称" />
    <span class="row-label">默认图标：</span>
    <button class="icon-pick" @click="openIconMenu">
      <AppIcon :name="row.icon ?? 'folder-closed'" :size="18" />
      <AppIcon name="tree-expand-arrow" :size="12" class="pick-chev" />
    </button>
    <IconButton name="delete-red" delete-red title="删除区块" :class="{ ghost: isDefault }" @click="emit('remove')" />
  </div>
</template>

<style scoped>
/* 行（30 稿 item）：52 高 / padding 12 0 / gap 8 / 底线 暗-悬停 */
.bm-row {
  display: flex;
  align-items: center;
  gap: var(--space-8);
  height: 52px;
  box-sizing: border-box;
  padding: var(--space-12) 0;
  border-bottom: 1px solid var(--line-hover);
  flex: none;
}
/* 下箭头：chevron-small 源图形朝上，旋转 180 朝下 */
.arrow-down :deep(svg) {
  transform: rotate(180deg);
}
/* 排序箭头（2026-08-14 用户定）：可用态 = 常态亮一点（高亮档）；禁用态 = 状态/禁用（--line-active）
   原样呈现（取消 IconButton 自带的半透明，避免双重变淡）。
   选择器需压过 IconButton 自带 .icon-btn.small / .icon-btn:disabled（同(0,2,0)级会被组件内样式盖掉） */
.bm-row .icon-btn.sort-arrow {
  color: var(--text-highlight);
}
.bm-row .icon-btn.sort-arrow:disabled {
  color: var(--line-active);
  opacity: 1;
}
/* 「目录」行（2026-08-14 用户定）：仅删除钮 opacity 0 占位不可点（目录是各兜底落点，禁删）；
   名称 / 图标 / 排序与其他行无异（改名无影响——内部锚定常量 id "dir"，非名称） */

/* 「目录」行占位隐藏（稿定 opacity 0 不可点，布局不动） */
.ghost {
  opacity: 0;
  pointer-events: none;
}
/* 行内标签（30 稿）：正文 14 标签色 */
.row-label {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-label);
  white-space: nowrap;
  flex: none;
}
/* 名称输入：弹性占满（稿 flex-grow 1）；稿定 C1 底（AppInput small 默认 C2，弹窗内输入框 token = C1） */
.name-input {
  flex: 1;
  min-width: 0;
}
.bm-row .name-input {
  background: var(--c1);
}
/* 图标下拉（30 稿）：58×28 / C4 底 / 1px 暗-悬停 / 圆角 6 / padding 4 4 4 8，图标 18 + 箭头 12 标签色 */
.icon-pick {
  display: inline-flex;
  align-items: center;
  gap: var(--space-8);
  width: 58px;
  height: 28px;
  box-sizing: border-box;
  padding: var(--space-4) var(--space-4) var(--space-4) var(--space-8);
  border-radius: var(--radius-6);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--line-hover);
  color: var(--text-normal);
  flex: none;
  transition: background 0.15s ease;
}
.icon-pick:hover {
  background: var(--c5);
}
.pick-chev {
  color: var(--text-label);
  flex: none;
}
</style>
