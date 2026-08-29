<script setup lang="ts">
// 列表卡片——docs/UI 重构/Design Token/组件/卡片/列表卡片.md：
// 行高 57 / padding 6/12/6/6 / gap 12 / 圆角 8；通用二级底 + 通用三级 1px 描边；
// hover = 底升通用三级 + 描边升通用五级（描边瞬时切换，踩坑 036 口径）；选中 2px 品牌描边（.kc-sel）。
// 列：缩略图 80×45（通用三级底 + 五级 1px 描边 + 圆角 6，图 contain——沿用旧 ListRowCard 口径）
//   / 名称+徽章（flex 1 gap 12：正文-加粗 一级 + 原始文件徽章 14）
//   / 尺寸 / 格式（FormatBadge）/ 文件大小 / 修改时间（日期 + 时间 gap 6）——各 123 宽，数字-大档 亮色/三级。
import AppIcon from "../AppIcon.vue";
import FormatBadge from "./FormatBadge.vue";
import "./cards.css";

withDefaults(
  defineProps<{
    name: string;
    thumbUrl?: string;
    badge?: string; // 格式角标（扩展名列）
    dims?: string; // 尺寸（1920 x 1080）
    fileSize?: string; // 文件大小（12 MB）
    modDate?: string; // 修改日期（2026/08/24）
    modTime?: string; // 修改时间（16:07）
    originBadge?: boolean; // 名称后原始文件-徽章
    selected?: boolean;
    forceState?: "" | "hover" | "selected";
  }>(),
  {
    thumbUrl: "",
    badge: "",
    dims: "",
    fileSize: "",
    modDate: "",
    modTime: "",
    originBadge: false,
    selected: false,
    forceState: "",
  },
);
</script>

<template>
  <div class="kc-sel lr-card" :class="[{ selected }, forceState ? `force-${forceState}` : '']">
    <span class="lr-thumb">
      <img v-if="thumbUrl" :src="thumbUrl" class="lr-thumb-img" alt="" />
    </span>
    <span class="lr-name-cell">
      <span class="lr-name">{{ name }}</span>
      <AppIcon v-if="originBadge" name="original-file-badge" :size="14" />
    </span>
    <span class="lr-col lr-dims">{{ dims }}</span>
    <span class="lr-col">
      <FormatBadge v-if="badge" :label="badge" />
    </span>
    <span class="lr-col lr-size">{{ fileSize }}</span>
    <span class="lr-col lr-mtime">
      <span>{{ modDate }}</span>
      <span>{{ modTime }}</span>
    </span>
  </div>
</template>

<style scoped>
.lr-card {
  box-sizing: border-box;
  position: relative;
  width: 100%;
  display: flex;
  flex-direction: row;
  align-items: center;
  padding: var(--n-space-6) var(--n-space-12) var(--n-space-6) var(--n-space-6);
  gap: var(--n-space-12);
  background: var(--n-g2);
  border: 1px solid var(--n-g3);
  border-radius: var(--n-radius-8);
  transition: background var(--n-dur-150) var(--n-ease-in-out);
}
.lr-card:hover,
.lr-card.force-hover {
  background: var(--n-g3);
  border-color: var(--n-g5);
}

/* 缩略图容器 80×45：通用三级底 + 五级 1px 描边 + 圆角 6，图 contain */
.lr-thumb {
  flex: none;
  width: 80px;
  height: 45px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--n-g3);
  border: 1px solid var(--n-g5);
  border-radius: var(--n-radius-6);
  overflow: hidden;
}
.lr-thumb-img {
  width: 100%;
  height: 100%;
  object-fit: contain;
}

/* 名称列：flex 1 gap 12，正文-加粗 一级截断 + 徽章 14 */
.lr-name-cell {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: var(--n-space-12);
}
.lr-name {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  font-weight: var(--n-fw-bold);
  color: var(--n-l1);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* 数据列：123 宽，数字-大档 亮色/三级 */
.lr-col {
  flex: none;
  width: 123px;
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: var(--n-space-6);
  font-family: var(--n-font-mono);
  font-size: var(--n-fs-num-lg);
  line-height: var(--n-lh-num-lg);
  font-weight: var(--n-fw-medium);
  color: var(--n-l3);
  white-space: nowrap;
  overflow: hidden;
}
</style>
