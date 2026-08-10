<script setup lang="ts">
// 列表式列头（figma-css 列表显示稿，2026-08-10 对齐）：高 46 / padding 12 / 标题档 16/22 700
// 标签色；名称列 flex 1，右侧列组间距 24、列宽 82 / 64 / 64 / 119 与列表行对齐；
// 每列均可作排序依据（图标 chevron-small 12，hover 列显示；排序业务阶段 4 接）。
import AppIcon from "./AppIcon.vue";
</script>

<template>
  <div class="list-header">
    <!-- 每列均可作排序依据（2026-08-10 用户定：名称 / 尺寸 / 文件大小 / 修改日期升降序；
         图标统一 chevron-small 12，hover 列显示；排序业务阶段 4 接） -->
    <div class="main">
      <span class="col col-title">
        名称
        <AppIcon name="chevron-small" :size="12" class="sort-arrow" />
      </span>
    </div>
    <div class="details">
      <span class="col size">
        尺寸
        <AppIcon name="chevron-small" :size="12" class="sort-arrow" />
      </span>
      <span class="col format">
        格式
        <AppIcon name="chevron-small" :size="12" class="sort-arrow" />
      </span>
      <span class="col filesize">
        文件大小
        <AppIcon name="chevron-small" :size="12" class="sort-arrow" />
      </span>
      <span class="col date">
        修改日期
        <AppIcon name="chevron-small" :size="12" class="sort-arrow" />
      </span>
    </div>
  </div>
</template>

<style scoped>
/* 列头行（figma 列表显示稿）：固定于列表区顶部（不随滚动），高 46 / 背景 C1 /
   标题档 16/22 700 标签色；padding 左 12 对齐行左缘，右 24 = 行 padding 12 + 网格
   PAD.right 12（滚动条区）——列头 details 右缘与行 details 右缘严格对齐（列头满宽、行 box 右缘内缩） */
.list-header {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  z-index: 5;
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 46px;
  box-sizing: border-box;
  padding: var(--space-12) var(--space-24) var(--space-12) var(--space-12);
  background: var(--c1);
  font-size: var(--text-title-size);
  line-height: var(--text-title-line);
  font-weight: var(--text-title-weight);
  color: var(--text-label);
}
.main {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
}
/* 右侧列组（figma）：间距 24 / 列宽 82 / 64 / 64 / 119 与列表行对齐 */
.details {
  display: flex;
  align-items: center;
  gap: var(--space-24);
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
  width: 82px;
}
.format {
  width: 64px;
}
.filesize {
  width: 64px;
}
.date {
  width: 119px;
}
/* 悬停列显示排序箭头（16×16，hover 提亮 C4） */
.sort-arrow {
  opacity: 0;
  transition: opacity 0.12s ease;
}
.col:hover .sort-arrow {
  opacity: 1;
}
</style>
