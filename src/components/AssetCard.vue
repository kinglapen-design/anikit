<script setup lang="ts">
// 素材卡片（02_通用组件/06_卡片 03，2026-07-23 用户定：回退图片+居中文字方案）：
// 总容器 padding 0 / 间距 4（无卡片框）；图片圆角 8 / 1px border-2（hover 2px 40% / 选中 2px 品牌色）；
// 格式角标：图片左上各 4px（实底格式色徽章）；按钮组：图片右下各 4px（图标按钮-小 24×24，
// 悬停卡片显示，hover 底色 C5——图片干扰大不裸奔 16 图标）；
// 文字组：间距 2 / 居中 / 标题最多两行截断。
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
      <span v-if="badgeColor" class="badge" :style="{ background: badgeColor }">{{ badge }}</span>
      <div class="actions">
        <button class="act" v-tip="'发送至 Animate'" @click.stop>
          <AppIcon name="send-to" />
        </button>
        <button class="act" v-tip="'查看大图'" @click.stop>
          <AppIcon name="magnifier" />
        </button>
      </div>
    </div>
    <div class="texts">
      <span class="title">{{ displayName }}</span>
      <span class="meta">{{ meta }}</span>
    </div>
  </div>
</template>

<style scoped>
/* 总容器：padding 0 / 间距 4（无卡片框） */
.asset-card {
  position: absolute;
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  cursor: default;
}
/* 图片：圆角 8 / 1px border-2（真 border，inset shadow 会被图片盖住）；
   hover 2px 40% / 选中 2px 品牌色（粗细变化由 cover 图片吸收，不动布局） */
.thumb {
  position: relative;
  flex: none;
  border: 1px solid var(--border-2);
  border-radius: var(--radius-8);
  overflow: hidden;
  transition: border-color 0.12s ease;
}
.asset-card:hover .thumb {
  border: 2px solid var(--font-icon-20);
}
.asset-card.selected .thumb {
  border: 2px solid var(--brand-1);
}
.img {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: cover;
  /* 无缩略图占位：中间面板底色 C1 */
  background: var(--c1);
}
/* 系统格式图标：方形图标 contain 完整显示不裁切（4.2 用户定） */
.img.icon {
  object-fit: contain;
}
/* 格式角标：图片左上各 4px；深色实底（降噪）/ 内边距 0 4 / 圆角 4 / 角标文字 10/16 粗体 60% */
.badge {
  position: absolute;
  top: 4px;
  left: 4px;
  padding: 0 var(--space-4);
  border-radius: var(--radius-4);
  font-size: var(--text-badge-size);
  line-height: var(--text-badge-line);
  font-weight: var(--text-badge-weight);
  color: var(--font-icon-3);
  pointer-events: none;
}
/* 按钮组：图片右下各 4px / 间距 4；图标按钮-小（24×24 圆角 4），
   底色 C1-20% 压图（深色透明不抢眼），悬停卡片显示，hover 底色 C1-40% */
.actions {
  position: absolute;
  right: 4px;
  bottom: 4px;
  display: flex;
  gap: var(--space-4);
  opacity: 0;
  transition: opacity 0.12s ease;
}
.asset-card:hover .actions {
  opacity: 1;
}
.act {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  padding: var(--space-4);
  border-radius: var(--radius-4);
  background: var(--c1-20);
  color: var(--font-icon-1);
  transition: background 0.12s ease;
}
.act:hover {
  background: var(--c1-40);
}
/* 文字组：间距 2 / 居中 / 高 60 顶部对齐（内容一行时与图片间距恒为总容器 gap 4）；
   标题最多两行截断 */
.texts {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  height: 60px;
  flex: none;
}
.title {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--font-icon-1);
  text-align: center;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  word-break: break-all;
}
.meta {
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--font-icon-3);
  text-align: center;
  white-space: nowrap;
}
</style>
