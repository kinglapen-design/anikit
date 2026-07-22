<script setup lang="ts">
// 文件夹卡片（02_通用组件/06_卡片 01 无缩略图态）：189 宽，图标容器 189×158（内边距 4 / 圆角 12）
// + 图标组 181×150（内联 SVG，结构色 var() 化）+ 标题 + 标签（文件数 / 子文件夹数）。
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
    <div class="fname">{{ folder.name }}</div>
    <div class="fmeta">{{ meta }}</div>
  </div>
</template>

<style scoped>
/* 总容器：间距 2 / 圆角 8；宽度由 FolderStrip 弹性控制（基准 189） */
.folder-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-2);
  border-radius: var(--radius-8);
  cursor: default;
}
/* 图标容器：内边距 4 / 圆角 12；悬停 2px / 选中 2px 内阴影描边（不占空间不抖动） */
.icon-wrap {
  width: 100%;
  padding: var(--space-4);
  border-radius: var(--radius-12);
  transition: box-shadow 0.12s ease;
  line-height: 0;
}
.folder-card:hover .icon-wrap {
  box-shadow: inset 0 0 0 2px var(--c4-15);
}
.folder-card.selected .icon-wrap {
  box-shadow: inset 0 0 0 2px var(--brand-1);
}
.icon-wrap :deep(svg) {
  width: 100%;
  height: auto;
  display: block;
}
.fname {
  width: 100%;
  height: 20px;
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--c4);
  text-align: center;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.fmeta {
  width: 100%;
  height: 18px;
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--c4-40);
  text-align: center;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
