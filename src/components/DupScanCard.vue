<script setup lang="ts">
// 扫描重复文件卡片（35_扫描重复文件稿 item 内"素材-卡片"，327×240.94，静态简化版 AssetCard）：
// 缩略图区 183.94（C3 底；无缩略图 = 36 稿同款浅色块 #C6C7CC + 扩展名文字）+ 简介区 57
//（名称单行高亮色 + 宽×高 标签色，无尺寸回退扩展名文案，同 AssetCard meta 口径）；
// 左上格式徽标（formatBadgeColor，静态图片 null 不出）+ 右下原始文件徽章
//（35 稿注明：只扫描原始文件，徽章恒为原始文件徽章）。无悬停播放 / 选择 / 重命名交互。
// 2026-10-16 链接扫描复用扩展（PRD 03_查找文件-04，同名第二处不新建组件）：
// badge="link" 换链接徽章；nameOverride / metaOverride 覆盖名称与简介行（链接卡 = 链接显示名 +
// 所在文件夹）；selectable 开整卡点击 + 右上复选框（复选 = 删除标记），selected 压暗 30%（§5 档）。
import { computed } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import AppIcon from "./AppIcon.vue";
import Checkbox from "./buttons/Checkbox.vue";
import { formatBadgeColor } from "../lib/formatBadge";
import type { DupAsset } from "../lib/dupScanOps";

const props = withDefaults(
  defineProps<{
    asset: DupAsset;
    badge?: "origin" | "link";
    nameOverride?: string;
    metaOverride?: string;
    selectable?: boolean;
    selected?: boolean;
  }>(),
  { badge: "origin", nameOverride: undefined, metaOverride: undefined },
);
const emit = defineEmits<{ toggle: [] }>();

const thumbUrl = computed(() =>
  props.asset.thumbPath ? convertFileSrc(props.asset.thumbPath) : null,
);
// 角标底色：静态图片 null = 不出徽标（图片内容即格式说明，同卡片口径）
const badgeColor = computed(() => formatBadgeColor(props.asset.ext));
const displayName = computed(() => props.nameOverride ?? props.asset.name);
// 简介档：宽×高；fla 等无尺寸资源回退扩展名（AssetCard 同口径）；链接卡 = metaOverride 所在文件夹
const metaText = computed(
  () =>
    props.metaOverride ??
    (props.asset.w != null && props.asset.h != null
      ? `${props.asset.w} x ${props.asset.h}`
      : props.asset.ext.toUpperCase()),
);

function onClick(): void {
  if (props.selectable) emit("toggle");
}
</script>

<template>
  <div class="scan-card" :class="{ selectable, selected }" @click="onClick">
    <div class="thumb">
      <img
        v-if="thumbUrl"
        :class="{ icon: asset.thumbKind === 'formatIcon' }"
        :src="thumbUrl"
        :alt="displayName"
        decoding="async"
        draggable="false"
      />
      <span v-else class="ext">{{ asset.ext.toUpperCase() }}</span>
    </div>
    <div class="info">
      <span class="name" :title="displayName">{{ displayName }}</span>
      <span class="meta">{{ metaText }}</span>
    </div>
    <span v-if="badgeColor" class="fmt-badge" :style="{ background: badgeColor }">{{ asset.ext.toUpperCase() }}</span>
    <!-- 可选卡复选框（链接卡 = 删除标记）：右上 6/6 与格式徽标同位镜像；点击不冒泡（卡 click 同义防双触发） -->
    <Checkbox
      v-if="selectable"
      class="card-check"
      :model-value="selected"
      @update:model-value="emit('toggle')"
      @click.stop
    />
    <AppIcon :name="badge === 'link' ? 'linked-file-badge' : 'original-file-badge'" :size="15" class="origin-badge" />
  </div>
</template>

<style scoped>
/* 卡片（35 稿）：327×240.94，圆角 6，边框走 ::after 伪元素（规范第 4 条，同素材卡片机制） */
.scan-card {
  position: relative;
  display: flex;
  flex-direction: column;
  width: 327px;
  height: 240.94px;
  border-radius: var(--n-radius-6);
  flex: none;
}
.scan-card::after {
  content: "";
  position: absolute;
  inset: 0;
  border: 1px solid var(--n-l-a3);
  border-radius: var(--n-radius-6);
  pointer-events: none;
}
/* 缩略图区（35 稿）：183.94 高 / C3 底 / 上圆角 6 */
.thumb {
  width: 100%;
  height: 183.94px;
  border-radius: var(--n-radius-6) var(--n-radius-6) 0 0;
  background: var(--n-g3);
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
  flex: none;
}
.thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  border-radius: var(--n-radius-6) var(--n-radius-6) 0 0; /* 双保险：容器 overflow 裁剪之外显式圆角 */
}
/* 格式图标：方形 contain 不裁切（2026-07-22 用户定） */
.thumb img.icon {
  object-fit: contain;
}
/* 无缩略图占位（36 稿同款）：浅色块 #C6C7CC + 扩展名文字 */
.thumb .ext {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #c6c7cc;
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  font-weight: var(--n-fw-bold);
  color: var(--n-g3);
}
/* 简介区（35 稿）：57 高 / C3 底 + 上边框 / padding 8 8 8 12 / 间距 4 */
.info {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-4);
  height: 57px;
  padding: var(--n-space-8) var(--n-space-8) var(--n-space-8) var(--n-space-12);
  background: var(--n-g3);
  box-shadow: inset 0 1px 0 0 var(--n-l-a3);
  border-radius: 0 0 var(--n-radius-6) var(--n-radius-6);
  flex: none;
}
.name {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l1);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.meta {
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  color: var(--n-l3);
}
/* 格式徽标（35 稿）：左上各 6，徽标档 10px，padding 2 4，圆角 4 */
.fmt-badge {
  position: absolute;
  left: 6px;
  top: 6px;
  padding: var(--n-space-2) var(--n-space-4);
  border-radius: var(--n-radius-4);
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  font-weight: var(--n-fw-bold);
  color: var(--n-l2);
  z-index: 1;
}
/* 原始文件徽章（35 稿）：右下各 8，15×15 品牌色图标；链接卡同位换 linked-file-badge */
.origin-badge {
  position: absolute;
  right: 8px;
  bottom: 8px;
  z-index: 1;
}
/* 可选卡（链接扫描）：整卡可点；选中 = 删除标记压暗 30%（§5 档，跳过/取消勾选同口径） */
.scan-card.selectable {
  cursor: pointer;
  transition: opacity 0.15s ease;
}
.scan-card.selected {
  opacity: 0.3;
}
/* 卡上复选框：右上各 6（与格式徽标左上同位镜像） */
.card-check {
  position: absolute;
  right: 6px;
  top: 6px;
  z-index: 1;
}
</style>
