<script setup lang="ts">
// 标签按钮——docs/06_UI重构/Design Token/组件/按钮/ 标签按钮-大-立体 / 小-立体 / 立体-半透明 三稿合一：
// solid md = 通用/四级，高 28 padding 6/12，次要文字-加粗 亮色/二级（hover 通用/六级）；
// solid sm = 通用/五级，高 24 padding 4/8，次要文字 亮色/二级（hover 通用/六级；
//            选中 = 品牌/常态底 + 文字升一级 + 立体感-主要按钮）；
// ghost sm = 亮色/半透明-3，高 24 padding 4/8，次要文字-加粗 亮色/二级（hover 半透明-2；
//            选中 = 半透明-1 底 + 文字升一级）；三者圆角 6 均带立体感-通用。
// removable（2026-09-20 用户定「扩充点击和删除」，烧 AppTag）：hover 时删除钮右至左滑入
// （宽 0→12 + 左距 6，150ms——沿旧 AppTag 既定交互）；removable 时根元素换 span（button 不可嵌 button）。
import AppIcon from "../AppIcon.vue";

withDefaults(
  defineProps<{
    label: string;
    variant?: "solid" | "ghost";
    size?: "md" | "sm";
    selected?: boolean;
    removable?: boolean;
    forceState?: "" | "hover";
  }>(),
  { variant: "solid", size: "sm", selected: false, removable: false, forceState: "" },
);
const emit = defineEmits<{ click: [e: MouseEvent]; remove: [] }>();

function onRemove(e: MouseEvent): void {
  e.stopPropagation();
  emit("remove");
}
</script>

<template>
  <span
    v-if="removable"
    class="tag-btn removable"
    :class="[variant, size, { selected }, forceState ? `force-${forceState}` : '']"
    role="button"
    @click="emit('click', $event)"
  >
    <span class="tag-lbl">{{ label }}</span>
    <button class="tag-remove" v-tip="'删除'" @click="onRemove">
      <AppIcon name="clear" :size="12" />
    </button>
  </span>
  <button
    v-else
    class="tag-btn"
    :class="[variant, size, { selected }, forceState ? `force-${forceState}` : '']"
    @click="emit('click', $event)"
  >
    {{ label }}
  </button>
</template>

<style scoped>
.tag-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: none;
  border-radius: var(--n-radius-6);
  box-shadow: var(--n-effect-raised);
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  color: var(--n-l2);
  transition:
    background var(--n-dur-150) var(--n-ease-in-out),
    color var(--n-dur-150) var(--n-ease-in-out);
}
/* 尺寸档 */
.tag-btn.md {
  height: 28px;
  padding: var(--n-space-6) var(--n-space-12);
  font-weight: var(--n-fw-bold);
}
.tag-btn.sm {
  height: 24px;
  padding: var(--n-space-4) var(--n-space-8);
}
/* solid：大 = 通用/四级加粗，小 = 通用/五级常规 */
.tag-btn.solid.md {
  background: var(--n-g4);
}
.tag-btn.solid.sm {
  background: var(--n-g5);
}
.tag-btn.solid:hover,
.tag-btn.solid.force-hover {
  background: var(--n-g6);
}
.tag-btn.solid.selected,
.tag-btn.solid.selected:hover {
  background: var(--n-brand);
  box-shadow: var(--n-effect-raised-primary);
  color: var(--n-l1);
}
/* ghost：半透明-3 加粗（筛选栏） */
.tag-btn.ghost {
  background: var(--n-l-a3);
  font-weight: var(--n-fw-bold);
}
.tag-btn.ghost:hover,
.tag-btn.ghost.force-hover {
  background: var(--n-l-a2);
}
.tag-btn.ghost.selected,
.tag-btn.ghost.selected:hover {
  background: var(--n-l-a1);
  color: var(--n-l1);
}
/* removable：hover 删除钮右至左滑入（宽 0→12 + 左距 6，150ms——沿旧 AppTag 既定交互） */
.tag-btn.removable {
  cursor: pointer;
}
.tag-remove {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 0;
  margin-left: 0;
  overflow: hidden;
  color: var(--n-l3);
  flex: none;
  transition:
    width var(--n-dur-150) var(--n-ease-in-out),
    margin-left var(--n-dur-150) var(--n-ease-in-out);
}
.tag-remove:hover {
  color: var(--n-l1);
}
.tag-btn.removable:hover .tag-remove,
.tag-btn.removable.force-hover .tag-remove {
  width: 12px;
  margin-left: var(--n-space-6);
}
</style>
