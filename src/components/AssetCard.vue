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
import { formatDuration } from "../lib/formatDuration";
import { useUiStore } from "../stores/ui";
import type { PlacementView } from "../stores/content";

const props = withDefaults(defineProps<{ asset: PlacementView; box: Box; selected?: boolean }>(), {
  selected: false,
});
// 查看大图按钮 → 进预览（4.3）；双击由 AssetGrid 绑在组件根元素上
defineEmits<{ preview: [] }>();
const ui = useUiStore();
const imgH = computed(() => Math.max(20, props.box.height - CARD_CAPTION_H));
const thumbUrl = computed(() => (props.asset.thumbPath ? convertFileSrc(props.asset.thumbPath) : null));
// 动图（5.1）：GIF / WEBP 动图；与序列帧区分（序列帧同样带 frameCount）
const isAnim = computed(() => props.asset.animated);
// 序列帧（2026-07-22 用户定）：格式角标显示"序列帧"，名称后缀显示为 .序列帧（仅显示层，数据不动）
const isSeq = computed(() => props.asset.frameCount != null && !isAnim.value);
// 角标文案：音频带 BPM 显示"格式 / BPM: 值"（Eagle 同格式，2026-07-26 用户截图定稿）
const badge = computed(() => {
  if (isSeq.value) return "序列帧";
  const ext = props.asset.ext.toUpperCase();
  return props.asset.bpm != null ? `${ext} / BPM: ${Math.round(props.asset.bpm)}` : ext;
});
// 角标徽章底色：格式实色（01_样式 token 01-6，7 组）
const badgeColor = computed(() => formatBadgeColor(props.asset.ext, isSeq.value, isAnim.value));
const displayName = computed(() =>
  isSeq.value ? `${props.asset.name.replace(/\.[^.]+$/, "")}.序列帧` : props.asset.name,
);
// 标签行：音视频改时长属性（mm:ss，5.1）；动图与其他按尺寸；fla 等无尺寸资源显示格式角标文案
const meta = computed(() => {
  if (props.asset.duration != null && !isAnim.value) return formatDuration(props.asset.duration);
  return props.asset.w != null && props.asset.h != null ? `${props.asset.w} x ${props.asset.h}` : badge.value;
});
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
        :class="{ icon: asset.thumbKind === 'formatIcon', contain: ui.layoutMode === 'grid' }"
        :src="thumbUrl"
        :alt="asset.name"
        decoding="async"
        draggable="false"
      />
      <div v-else class="img" />
      <span v-if="badgeColor && ui.showExtBadge" class="badge" :style="{ background: badgeColor }">{{ badge }}</span>
      <div class="actions">
        <button class="act" v-tip="'发送至 Animate'" @click.stop>
          <AppIcon name="send-to" />
        </button>
        <button class="act" v-tip="'查看大图'" @click.stop="$emit('preview')">
          <AppIcon name="magnifier" />
        </button>
      </div>
    </div>
    <div class="texts">
      <span class="title">{{ displayName }}</span>
      <span v-show="ui.briefMode !== 'none'" class="meta">{{ meta }}</span>
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
  display: flex; /* 格式图标居中缩到 50%（2026-07-25 用户定）；内容缩略图仍 width/height 100% */
  align-items: center;
  justify-content: center;
  border: 1px solid var(--border-2);
  border-radius: var(--radius-8);
  overflow: hidden;
  transition: border-color 0.12s ease;
}
.asset-card:hover .thumb {
  border: 2px solid var(--border-1);
}
.asset-card.selected .thumb {
  border: 2px solid var(--brand-1);
}
/* 网格模式固定格：contain 完整显示不裁切（cover 必裁，2026-07-24 用户定） */
.img {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: cover;
  /* 无缩略图占位：中间面板底色 C1 */
  background: var(--c1);
}
.img.contain {
  object-fit: contain;
}
/* shell 格式图标：最大边 = 容器 50%（左右各留 1/4，Eagle 口径——小一点，
   不与真正的内容缩略图抢视觉噪音，2026-07-25 用户定） */
.img.icon {
  width: 50%;
  height: 50%;
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
  /* 图标常态 60%，悬停按钮 80%（2026-07-23 用户定） */
  color: var(--font-icon-3);
  transition:
    background 0.12s ease,
    color 0.12s ease;
}
.act:hover {
  background: var(--c1-40);
  color: var(--font-icon-2);
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
