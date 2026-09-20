<script setup lang="ts">
// 区块管理行（30 稿 item，2026-08-14 区块子系统）：高 52 / padding 12 0 / gap 8 / 底线 通用/五级——
// 排序箭头（buttons/IconButton bare sm，图标 = 箭头-小 chevron-small：下移在前、上移旋转 180；
// 可用态亮色/一级（2026-08-14 用户定沿），禁用 = 新件自带 30% 透明度——旧「--line-active 原样呈现」
// 覆盖随旧件退役）+「区块名称：」标签 + 输入框（inputs/TextInput sm——旧 AppInput small 与
// 稿定 C1 底覆盖一并退役，底/描边走新件稿口径）+「默认图标：」标签 + 下拉 58×28 + 红色圆减号删除
//（删除钮 = clear X bare sm，2026-09-20 用户定「红色删除改成 X」）。
// 「目录」行（isDefault，2026-08-14 用户定）：仅删除 opacity 0 占位不可点（目录是各兜底落点）；
// 名称 / 图标 / 排序全部放开——锚定常量 id "dir"，改名无影响。
// 草稿式：行编辑只改草稿对象（TextInput v-model / setIcon 直改 row 属性，提交由弹窗统一组装）。
// 图标下拉 = 05 稿 §3 网格面板（2026-08-18，3.3：openPanelMenu 整体面板菜单，FolderIconGrid 共用）。
import AppIcon from "./AppIcon.vue";
import TextInput from "./inputs/TextInput.vue";
import IconButton from "./buttons/IconButton.vue";
import { openPanelMenu } from "../lib/contextMenus";
import type { BlockDraft } from "../lib/blockOps";

const props = defineProps<{
  row: BlockDraft;
  isFirst: boolean; // 首行：上箭头禁用
  isLast: boolean; // 末行：下箭头禁用
  isDefault: boolean; // 「目录」行：删除与上箭头占位隐藏
}>();
const emit = defineEmits<{ moveUp: []; moveDown: []; remove: [] }>();

// 默认图标下拉（58×28）：点击弹 05 稿 §3 图标网格（2026-08-18，3.3 换网格面板落地——
// 整体面板菜单 openPanelMenu，20 格，选中即写草稿并关菜单）
function openIconMenu(e: MouseEvent): void {
  const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
  openPanelMenu(
    { clientX: rect.left, clientY: rect.bottom + 4 },
    { kind: "iconPicker", current: props.row.icon, onPick: (name) => (props.row.icon = name) },
  );
}
</script>

<template>
  <div class="bm-row" :class="{ default: isDefault }">
    <!-- 排序箭头（30 稿：图标 = 箭头-小 chevron-small；下移在前、上移在后。
         chevron-small 源图形朝上（实机渲染核对）——上移不旋转、下移旋转 180） -->
    <IconButton variant="bare" size="sm" name="chevron-small" class="sort-arrow arrow-down" :disabled="isLast" title="下移" @click="emit('moveDown')" />
    <IconButton variant="bare" size="sm" name="chevron-small" class="sort-arrow arrow-up" :disabled="isFirst" title="上移" @click="emit('moveUp')" />
    <span class="row-label">区块名称:</span>
    <TextInput v-model="row.name" size="sm" class="name-input" placeholder="区块名称" />
    <span class="row-label">默认图标:</span>
    <button class="icon-pick" @click="openIconMenu">
      <AppIcon :name="row.icon ?? 'folder-closed'" :size="18" />
      <AppIcon name="tree-expand-arrow" :size="12" class="pick-chev" />
    </button>
    <IconButton name="clear" variant="bare" size="sm" title="删除区块" :class="{ ghost: isDefault }" @click="emit('remove')" />
  </div>
</template>

<style scoped>
/* 行（30 稿 item）：52 高 / padding 12 0 / gap 8 / 底线 通用/五级 */
.bm-row {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
  height: 52px;
  padding: var(--n-space-12) 0;
  border-bottom: 1px solid var(--n-g5);
  flex: none;
}
/* 下箭头：chevron-small 源图形朝上，旋转 180 朝下 */
.arrow-down :deep(svg) {
  transform: rotate(180deg);
}
/* 排序箭头（2026-08-14 用户定）：可用态 = 亮色/一级（提权压组件自带 .ic-btn.bare）；
   禁用 = 新件自带 30% 透明度（控件族通用口径，2026-09-20 用户定） */
.bm-row .ic-btn.sort-arrow {
  color: var(--n-l1);
}
/* 「目录」行占位隐藏（稿定 opacity 0 不可点，布局不动） */
.ghost {
  opacity: 0;
  pointer-events: none;
}
/* 行内标签（30 稿）：正文 13/18 亮色/三级 */
.row-label {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l3);
  white-space: nowrap;
  flex: none;
}
/* 名称输入：弹性占满（稿 flex-grow 1） */
.name-input {
  flex: 1;
  min-width: 0;
}
/* 图标下拉（30 稿）：58×28 / 通用/四级底 / 1px 通用/五级 / 圆角 6 / padding 4 4 4 8，图标 18 + 箭头 12 亮色/三级 */
.icon-pick {
  display: inline-flex;
  align-items: center;
  gap: var(--n-space-8);
  width: 58px;
  height: 28px;
  padding: var(--n-space-4) var(--n-space-4) var(--n-space-4) var(--n-space-8);
  border-radius: var(--n-radius-6);
  background: var(--n-g4);
  box-shadow: inset 0 0 0 1px var(--n-g5);
  color: var(--n-l2);
  flex: none;
  transition: background var(--n-dur-150) var(--n-ease-in-out);
}
.icon-pick:hover {
  background: var(--n-g5);
}
.pick-chev {
  color: var(--n-l3);
  flex: none;
}
</style>
