<script setup lang="ts">
// 文件夹卡片（02_通用组件/06_卡片 01 无缩略图态）：189 宽，图标容器 189×158（内边距 4 / 圆角 12）
// + 图标组 181×150（内联 SVG，结构色 var() 化）+ 标题 + 标签（文件数 / 子文件夹数）。
import { getIcon } from "../lib/icons";
import type { FakeFolder } from "../lib/fakeData";

defineProps<{ folder: FakeFolder }>();
const folderSvg = getIcon("folder-card");
</script>

<template>
  <div class="folder-card">
    <div class="icon-wrap" v-html="folderSvg" />
    <div class="fname">{{ folder.name }}</div>
    <div class="fmeta">{{ folder.count.toLocaleString() }} 个文件 / {{ folder.children.length }} 个子文件夹</div>
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
/* 图标容器：内边距 4 / 圆角 12；悬停 2px C4-40 / 选中 2px 品牌色-1（常态透明占位） */
.icon-wrap {
  width: 100%;
  padding: var(--space-4);
  border-radius: var(--radius-12);
  border: var(--border-thick) solid transparent;
  transition: border-color 0.12s ease;
  line-height: 0;
}
.folder-card:hover .icon-wrap {
  border-color: var(--c4-40);
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
