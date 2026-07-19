<script setup lang="ts">
// 素材卡片（02_通用组件/06_卡片 03）：图片容器（内边距 4 / 圆角 12，悬停 2px C4-40 / 选中 2px 品牌色-1）
// + 图片（圆角 8 / 1px C4-15）+ 格式角标（左上 4px / C1-80）+ 放大镜按钮（悬停显示，右下 4px）
// + 标题 + 尺寸标签。
import { computed } from "vue";
import AppIcon from "./AppIcon.vue";
import { CARD_CAPTION_H, type Box } from "../lib/justified";
import type { FakeAsset } from "../stores/content";

const props = withDefaults(defineProps<{ asset: FakeAsset; box: Box; selected?: boolean }>(), {
  selected: false,
});
const imgH = computed(() => Math.max(20, props.box.height - CARD_CAPTION_H));
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
      <div class="img" :style="{ background: asset.color }" />
      <span class="badge">{{ asset.format.toUpperCase() }}</span>
      <button class="zoom" title="查看大图" @click.stop>
        <AppIcon name="magnifier" />
      </button>
    </div>
    <div class="title">{{ asset.name }}</div>
    <div class="meta">{{ asset.w }} x {{ asset.h }}</div>
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
  box-shadow: inset 0 0 0 2px var(--c4-40);
}
.asset-card.selected .thumb {
  box-shadow: inset 0 0 0 2px var(--brand-1);
}
.img {
  width: 100%;
  height: 100%;
  border-radius: var(--radius-8);
  box-shadow: inset 0 0 0 1px var(--c4-15);
}
/* 格式角标：图片左上角，距上左 4px（= 容器内边距 4 + 4），内边距 2 4 / 圆角 6 / C1-80 */
.badge {
  position: absolute;
  top: 8px;
  left: 8px;
  padding: var(--space-2) var(--space-4);
  border-radius: var(--radius-6);
  background: var(--c1-80);
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--c4-70);
  pointer-events: none;
  z-index: 2;
}
/* 放大镜按钮：图片右下角，距右下 4px，悬停卡片时显示，悬停自身底色 C1-80 */
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
  color: var(--c4);
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
  background: var(--c1-80);
}
.title {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--c4);
  text-align: center;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.meta {
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--c4-40);
  text-align: center;
  white-space: nowrap;
}
</style>
