<script setup lang="ts">
// 文件夹卡片（02_通用组件/06_卡片 01 无缩略图态，2026-07-23 用户定稿）：
// 总容器间距 4；图标容器 padding 0 / 圆角 8 / 1px border-2（hover 2px font-icon-20 /
// 选中 2px 品牌色——与素材卡片一致）；文字组与素材卡片一致（间距 2 / 高 60 / 标题两行截断）。
import { computed } from "vue";
import { getIcon } from "../lib/icons";
import type { FolderNode } from "../stores/content";

const props = withDefaults(defineProps<{ folder: FolderNode; selected?: boolean }>(), {
  selected: false,
});
const emit = defineEmits<{ select: [] }>();
const folderSvg = getIcon("folder-card");

// 副文本规则：有子文件夹显示"X 个文件 / Y 个子文件夹"，没有则仅"X 个文件"（可为 0）
const meta = computed(() => {
  const files = `${props.folder.count.toLocaleString()} 个文件`;
  return props.folder.children.length > 0
    ? `${files} / ${props.folder.children.length} 个子文件夹`
    : files;
});
</script>

<template>
  <div class="folder-card" :class="{ selected }" @click="emit('select')">
    <div class="icon-wrap" v-html="folderSvg" />
    <div class="texts">
      <span class="title">{{ folder.name }}</span>
      <span class="meta">{{ meta }}</span>
    </div>
  </div>
</template>

<style scoped>
/* 总容器：间距 4；宽度由 FolderStrip 弹性控制（基准 189） */
.folder-card {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  cursor: default;
}
/* 图标容器：padding 0 / 圆角 10（= SVG 圆角 8 + 边框 2，内缘 8 与 SVG 贴合）；
   边框由 ::after 绘制（不占布局——SVG 常态即与副标题左对齐，hover/选中仅上色） */
.icon-wrap {
  position: relative;
  width: 100%;
  border-radius: 10px;
  overflow: hidden;
  line-height: 0;
}
.icon-wrap::after {
  content: "";
  position: absolute;
  inset: 0;
  border: 2px solid transparent;
  border-radius: 10px;
  pointer-events: none;
  transition: border-color 0.12s ease;
}
/* hover 2px border-1 / 选中 2px 品牌色 */
.folder-card:hover .icon-wrap::after {
  border-color: var(--border-1);
}
.folder-card.selected .icon-wrap::after {
  border-color: var(--brand-1);
}
.icon-wrap :deep(svg) {
  width: 100%;
  height: auto;
  display: block;
}
/* 文字组：与素材卡片一致——间距 2 / 居中 / 高 60 顶部对齐 / 标题两行截断 */
.texts {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  height: 60px;
  flex: none;
}
.title {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--font-icon-1);
  text-align: center;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  word-break: break-all;
}
.meta {
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--font-icon-3);
  text-align: center;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
