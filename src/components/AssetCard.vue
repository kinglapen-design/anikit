<script setup lang="ts">
// 素材卡片（token 17）：图片容器（悬停 2px C4-40 / 选中 2px 品牌色-1）
// + 色块图（圆角 8 / 1px C4-15）+ 格式角标（左上 4px / C1-80）+ 标题 + 尺寸标签。
import { computed } from "vue";
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
/* 图片容器：内边距 6、圆角 12；边框常态透明占位，悬停 / 选中换色不抖动 */
.thumb {
  position: relative;
  padding: var(--space-6);
  border-radius: var(--radius-12);
  border: var(--border-thick) solid transparent;
  transition: border-color 0.12s ease;
}
.asset-card:hover .thumb {
  border-color: var(--c4-40);
}
.asset-card.selected .thumb {
  border-color: var(--brand-1);
}
.img {
  width: 100%;
  height: 100%;
  border-radius: var(--radius-8);
  border: 1px solid var(--c4-15);
}
/* 格式角标（token 18）：图片左上角，距上左 4px */
.badge {
  position: absolute;
  top: 10px;
  left: 10px;
  padding: 3px var(--space-6);
  border-radius: var(--radius-6);
  background: var(--c1-80);
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--c4-70);
  pointer-events: none;
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
