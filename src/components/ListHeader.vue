<script setup lang="ts">
// 列表式列头（02_item 10 详情容器对应的表头行）：名称 / 尺寸 / 格式 / 文件大小 / 修改日期，
// 固定列宽与列表行对齐（名称列随缩略图宽度弹性）；悬停列出现排序箭头（阶段 4 接排序）。
import AppIcon from "./AppIcon.vue";

const cols = ["尺寸", "格式", "文件大小", "修改日期"] as const;
</script>

<template>
  <div class="list-header">
    <div class="main">
      <span class="col-title">名称</span>
    </div>
    <div class="details">
      <span class="col size">
        尺寸
        <AppIcon name="sort-arrow" :size="16" class="sort-arrow" />
      </span>
      <span class="col format">
        格式
        <AppIcon name="sort-arrow" :size="16" class="sort-arrow" />
      </span>
      <span class="col filesize">
        文件大小
        <AppIcon name="sort-arrow" :size="16" class="sort-arrow" />
      </span>
      <span class="col date">
        修改日期
        <AppIcon name="sort-arrow" :size="16" class="sort-arrow" />
      </span>
    </div>
  </div>
</template>

<style scoped>
/* 列头行：固定于列表区顶部（不随滚动），高 28，背景 C2；标题文字正文（14/20），
   右 padding 16（与行 details 右缘对齐：行 box 右缘 -8 + 行 padding-right 8） */
.list-header {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  z-index: 5;
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 28px;
  padding: 0 16px 0 8px;
  background: var(--c2);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--c4-40);
}
.main {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
}
.details {
  display: flex;
  align-items: center;
  gap: var(--space-16);
  flex: none;
}
.col {
  display: inline-flex;
  align-items: center;
  gap: var(--space-4);
  white-space: nowrap;
  cursor: pointer;
}
.size {
  width: 90px;
}
.format {
  width: 50px;
}
.filesize {
  width: 70px;
}
.date {
  width: 130px;
}
/* 悬停列显示排序箭头（16×16，hover 提亮 C4） */
.sort-arrow {
  opacity: 0;
  transition: opacity 0.12s ease;
}
.col:hover .sort-arrow {
  opacity: 1;
}
.col:hover .sort-arrow :deep(svg [stroke-opacity]) {
  stroke-opacity: 1;
}
</style>
