<script setup lang="ts">
// 资源库菜单 item（08 稿 §1 资源库 item）：移动抓手（拖动排序起点，拖拽由 ContextMenu 协调）+
// 品牌徽章（20×20 品牌底圆角 4 + 亮-选中边框 + library-duotone-small）+ 库名（标题档 16 常态色）
// + 当前库对勾 + 右侧删除×；当前库整行叠亮-选中罩层。点击行 = 切换；× = 删除（确认弹窗在外层）。
import AppIcon from "./AppIcon.vue";

withDefaults(defineProps<{ label: string; current?: boolean }>(), { current: false });
const emit = defineEmits<{ select: []; remove: []; gripDown: [e: PointerEvent] }>();
</script>

<template>
  <div class="lib-item" :class="{ current }" @click="emit('select')">
    <span class="main">
      <span class="grip" @pointerdown.stop="emit('gripDown', $event)">
        <AppIcon name="drag-handle" :size="12" />
      </span>
      <span class="badge"><AppIcon name="library-duotone-small" :size="15" /></span>
      <span class="label">{{ label }}</span>
      <AppIcon v-if="current" name="check" :size="15" class="check" />
    </span>
    <button class="remove" title="删除资源库" @click.stop="emit('remove')">
      <AppIcon name="window-close" :size="12" />
    </button>
  </div>
</template>

<style scoped>
/* 行（08 稿）：33 高 / padding 6 8 / 圆角 6；当前库叠亮-选中罩层 */
.lib-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-12);
  width: 100%;
  padding: var(--space-6) var(--space-8);
  border-radius: var(--radius-6);
  color: var(--text-normal);
  cursor: pointer;
  transition:
    background 0.15s ease,
    transform 0.15s ease;
  user-select: none;
}
.lib-item:hover {
  background: linear-gradient(var(--line-hover), var(--line-hover));
}
.lib-item.current {
  background: linear-gradient(var(--line-active), var(--line-active));
}
/* 左组（稿：gap 8） */
.main {
  display: flex;
  align-items: center;
  gap: var(--space-8);
  flex: 1;
  min-width: 0;
}
/* 移动抓手：12 盒标签色，抓取光标 */
.grip {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 12px;
  height: 12px;
  color: var(--text-label);
  cursor: grab;
  flex: none;
  touch-action: none; /* 指针捕获拖拽，禁浏览器滚动手势 */
}
/* 品牌徽章（稿：20×20 品牌底 / 亮-选中边框 / 圆角 4 / 双色小图标高亮） */
.badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  border-radius: var(--radius-4);
  background: var(--brand);
  box-shadow: inset 0 0 0 1px var(--line-active);
  color: var(--text-highlight);
  flex: none;
}
/* 库名：标题档 16/22 700 常态色（08 稿，与其他菜单项的正文档不同） */
.label {
  font-size: var(--text-title-size);
  line-height: var(--text-title-line);
  font-weight: var(--text-title-weight);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.check {
  color: var(--text-normal);
  flex: none;
}
/* 删除×：20×20 圆角 6 标签色，hover 叠暗-悬停 */
.remove {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  border-radius: var(--radius-6);
  color: var(--text-label);
  flex: none;
}
.remove:hover {
  background: linear-gradient(var(--line-hover), var(--line-hover));
  color: var(--text-normal);
}
</style>
