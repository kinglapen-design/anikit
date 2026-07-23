<script setup lang="ts">
// 素材卡片（02_通用组件/06_卡片 03，2026-07-23 用户新设计）：
// 总容器 padding 0 / gap 0 / 1px border-2 / 圆角 8（hover 2px border-1 / 选中 2px 品牌色-1）；
// 图片区（上两角圆角 8，overflow 裁切）+ 标题容器（padding 8 / 两端自动拉伸）：
//   文字容器（名称 正文 font-icon-1 + 标签 副文本 font-icon-3，纵向间距 4）
//   格式角标容器（间距 8）：实底格式色徽章（padding 0 6 / 圆角 4 / 角标文字 12/18 粗体）
//   + 按钮容器（间距 4，图标 16×16 / font-icon-3 / hover font-icon-1）。
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
// 角标徽章底色：格式实色（01_样式 token 01-6，7 组）
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
    </div>
    <div class="caption">
      <div class="texts">
        <span class="title">{{ displayName }}</span>
        <span class="meta">{{ meta }}</span>
      </div>
      <div class="badge-group">
        <span class="badge" :style="{ background: badgeColor }">{{ badge }}</span>
        <div class="actions">
          <button class="act" v-tip="'发送至 Animate'" @click.stop>
            <AppIcon name="send-to" />
          </button>
          <button class="act" v-tip="'查看大图'" @click.stop>
            <AppIcon name="magnifier" />
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* 总容器：padding 0 / gap 0 / 1px border-2 / 圆角 8（overflow 裁切图片与文字区角） */
.asset-card {
  position: absolute;
  display: flex;
  flex-direction: column;
  border-radius: var(--radius-8);
  box-shadow: inset 0 0 0 1px var(--border-2);
  overflow: hidden;
  cursor: default;
  transition: box-shadow 0.12s ease;
}
.asset-card:hover {
  box-shadow: inset 0 0 0 2px var(--border-1);
}
.asset-card.selected {
  box-shadow: inset 0 0 0 2px var(--brand-1);
}
/* 图片区：上两角随容器圆角（overflow 裁切），无缩略图占位 = 中间面板底色 C1 */
.thumb {
  flex: none;
  overflow: hidden;
}
.img {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: cover;
  background: var(--c1);
}
/* 系统格式图标：方形图标 contain 完整显示不裁切（4.2 用户定） */
.img.icon {
  object-fit: contain;
}
/* 标题容器：内边距 8 / 两端自动拉伸 / 垂直居中 */
.caption {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-8);
  padding: var(--space-8);
  flex: 1;
  min-height: 0;
}
/* 文字容器：纵向间距 4 */
.texts {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  min-width: 0;
}
.title {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--font-icon-1);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.meta {
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--font-icon-3);
  white-space: nowrap;
}
/* 格式角标容器：间距 8 */
.badge-group {
  display: flex;
  align-items: center;
  gap: var(--space-8);
  flex: none;
}
/* 格式角标徽章：实底格式色 / 内边距 0 6 / 圆角 4 / 角标文字 12/18 粗体 */
.badge {
  padding: 0 var(--space-6);
  border-radius: var(--radius-4);
  font-size: var(--text-badge-size);
  line-height: var(--text-badge-line);
  font-weight: var(--text-badge-weight);
  color: var(--font-icon-1);
  pointer-events: none;
}
/* 按钮容器：间距 4，图标 16×16 / font-icon-3 / hover font-icon-1 */
.actions {
  display: flex;
  gap: var(--space-4);
}
.act {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  color: var(--font-icon-3);
  transition: color 0.12s ease;
}
.act:hover {
  color: var(--font-icon-1);
}
</style>
