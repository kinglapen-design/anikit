<script setup lang="ts">
// 文件夹卡片（token 16，无缩略图态）：内联 SVG（结构色 var() 化，必须内联才生效）
// + 标题 + 标签（子文件夹 / 文件数量）。181×150 设计框，横排固定尺寸。
import { getIcon } from "../lib/icons";
import type { FakeFolder } from "../lib/fakeData";

defineProps<{ folder: FakeFolder }>();
const folderSvg = getIcon("folder-card");
</script>

<template>
  <div class="folder-card">
    <div class="icon-wrap" v-html="folderSvg" />
    <div class="fname">{{ folder.name }}</div>
    <div class="fmeta">{{ folder.children.length }} 个子文件夹 / {{ folder.count.toLocaleString() }} 个文件</div>
  </div>
</template>

<style scoped>
/* 内容容器：子项间距 2 */
.folder-card {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  width: 193px;
  flex: none;
  cursor: default;
}
/* 图标容器：圆角 12；悬停 2px C4-40 / 选中 2px 品牌色-1（常态透明占位） */
.icon-wrap {
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
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--c4);
  text-align: center;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.fmeta {
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--c4-40);
  text-align: center;
  white-space: nowrap;
}
</style>
