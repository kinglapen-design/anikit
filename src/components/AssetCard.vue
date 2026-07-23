<script setup lang="ts">
// 素材卡片（02_通用组件/06_卡片 03）：图片容器（内边距 4 / 圆角 12，悬停 2px font-icon-3 / 选中 2px 品牌色-1）
// + 图片（圆角 8 / 1px border-2）+ 格式角标（左上 4px / C1）+ 放大镜按钮（悬停显示，右下 4px）
// + 标题 + 尺寸标签。
import { computed } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import AppIcon from "./AppIcon.vue";
import { CARD_CAPTION_H, type Box } from "../lib/layouts";
import { formatBadgeColor } from "../lib/formatBadge";
import type { PlacementView } from "../stores/content";

const props = withDefaults(defineProps<{ asset: PlacementView; box: Box; selected?: boolean }>(), {
  selected: false,
});
const imgH = computed(() => Math.max(20, props.box.height - CARD_CAPTION_H));
const thumbUrl = computed(() => (props.asset.thumbPath ? convertFileSrc(props.asset.thumbPath) : null));
// 序列帧（2026-07-22 用户定）：格式角标显示"序列帧"，名称后缀显示为 .序列帧（仅显示层，数据不动）
const isSeq = computed(() => props.asset.frameCount != null);
const badge = computed(() => (isSeq.value ? "序列帧" : props.asset.ext.toUpperCase()));
// 角标文字色：品牌格式各自品牌色 / 图片与序列帧统一色 / 其他 font-icon-2（01_样式 token 01-5）
const badgeColor = computed(() => formatBadgeColor(props.asset.ext, isSeq.value));
const displayName = computed(() =>
  isSeq.value ? `${props.asset.name.replace(/\.[^.]+$/, "")}.序列帧` : props.asset.name,
);
// 尺寸标签：fla 等无尺寸资源显示格式（阶段 5 插件补图后即有尺寸）
const meta = computed(() =>
  props.asset.w != null && props.asset.h != null ? `${props.asset.w} x ${props.asset.h}` : badge.value,
);
</script>

<template>
  <div
    class="asset-card"
    :class="{ selected }"
    :style="{
      left: `${box.left}px`,
      top: `${box.top}px`,
      width: `${box.width}px`,
      height: `${box.height}px`,
    }"
  >
    <div class="thumb" :style="{ height: `${imgH}px` }">
      <img
        v-if="thumbUrl"
        class="img"
        :class="{ icon: asset.thumbKind === 'formatIcon' }"
        :src="thumbUrl"
        :alt="asset.name"
        decoding="async"
        draggable="false"
      />
      <div v-else class="img" />
      <span class="badge" :style="{ color: badgeColor }">{{ badge }}</span>
      <button class="zoom" v-tip="'查看大图'" @click.stop>
        <AppIcon name="magnifier" />
      </button>
    </div>
    <div class="title">{{ displayName }}</div>
    <div class="meta">{{ meta }}</div>
  </div>
</template>

<style scoped>
.asset-card {
  position: absolute;
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  cursor: default;
}
/* 图片容器：内边距 4 / 圆角 12；悬停 / 选中 2px 内阴影描边（不占空间不抖动） */
.thumb {
  position: relative;
  padding: var(--space-4);
  border-radius: var(--radius-12);
  transition: box-shadow 0.12s ease;
}
.asset-card:hover .thumb {
  box-shadow: inset 0 0 0 2px var(--border-1);
}
.asset-card.selected .thumb {
  box-shadow: inset 0 0 0 2px var(--brand-1);
}
.img {
  display: block;
  width: 100%;
  height: 100%;
  border-radius: var(--radius-8);
  /* 卡片线框 border-2（2026-07-23 线框定案：静态线统一一档，废弃弱透明线） */
  box-shadow: inset 0 0 0 1px var(--border-2);
  object-fit: cover;
  /* 无缩略图占位：中间面板底色 C1（阶段3修改——不要 font-icon-4 灰块） */
  background: var(--c1);
}
/* 系统格式图标：方形图标 contain 完整显示不裁切（4.2 用户定） */
.img.icon {
  object-fit: contain;
}
/* 格式角标：图片左上角，距上左 4px（= 容器内边距 4 + 4），内边距 2 4 / 圆角 6 / C1 */
.badge {
  position: absolute;
  top: 8px;
  left: 8px;
  padding: var(--space-2) var(--space-4);
  border-radius: var(--radius-6);
  background: var(--c1);
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--font-icon-2);
  pointer-events: none;
  z-index: 2;
}
/* 放大镜按钮：图片右下角，距右下 4px，悬停卡片时显示，悬停自身底色 C1 */
.zoom {
  position: absolute;
  right: 8px;
  bottom: 8px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  padding: var(--space-4);
  border-radius: var(--radius-6);
  color: var(--font-icon-1);
  opacity: 0;
  transition:
    opacity 0.12s ease,
    background 0.12s ease;
  z-index: 1;
}
.asset-card:hover .zoom {
  opacity: 1;
}
.zoom:hover {
  background: var(--c1);
}
.title {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--font-icon-1);
  text-align: center;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.meta {
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--font-icon-3);
  text-align: center;
  white-space: nowrap;
}
</style>
