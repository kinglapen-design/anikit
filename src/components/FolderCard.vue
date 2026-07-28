<script setup lang="ts">
// 文件夹卡片（02_通用组件/06_卡片，2026-07-23 用户定稿；4.4 接 02 有缩略图态）：
// 总容器间距 4；图标容器 padding 0 / 圆角 8 / 1px border-2（hover 2px font-icon-20 /
// 选中 2px 品牌色——与素材卡片一致）；文字组与素材卡片一致（间距 2 / 高 60 / 标题两行截断）。
// 有缩略图态按 06_卡片 02 分层：图层4 背面 + 子项首个静态图片（照片探出，contain 不裁切，
// 原文件夹浅色纸底保留托住透明通道）+ 有缩略图层1 正面下沿。
import { computed } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { getIcon } from "../lib/icons";
import { dragView } from "../lib/dragOperation";
import type { FolderNode } from "../stores/content";

const props = withDefaults(
  defineProps<{ folder: FolderNode; thumbPath?: string | null; selected?: boolean }>(),
  { thumbPath: null, selected: false },
);
const emit = defineEmits<{ select: [] }>();
const folderSvg = getIcon("folder-card");
const backSvg = getIcon("folder-card-layer4");
const frontThumbSvg = getIcon("folder-card-thumb-layer1");
const thumbUrl = computed(() => (props.thumbPath ? convertFileSrc(props.thumbPath) : ""));

// 拖拽落点视觉（6.1 C4b，2026-07-28 定案）：并入 = 覆盖缩略图区域的方形覆盖
//（--brand-1-12 + inset 1px --brand-1）；同级排序 = 卡片左/右侧间隙竖块（仅文件夹拖拽）
const dropInto = computed(() => dragView.folderCardId === props.folder.id);
const gapSide = computed(() =>
  dragView.folderGapId === props.folder.id ? (dragView.folderGapAfter ? "right" : "left") : null,
);

// 副文本规则：有子文件夹显示"X 个文件 / Y 个子文件夹"，没有则仅"X 个文件"（可为 0）
const meta = computed(() => {
  const files = `${props.folder.count.toLocaleString()} 个文件`;
  return props.folder.children.length > 0
    ? `${files} / ${props.folder.children.length} 个子文件夹`
    : files;
});
</script>

<template>
  <div
    class="folder-card"
    :class="{ selected }"
    :data-folder-id="folder.id"
    :data-drag-name="folder.name"
    @click="emit('select')"
  >
    <div class="icon-wrap" :class="{ thumb: !!thumbUrl }">
      <template v-if="thumbUrl">
        <span class="layer back" v-html="backSvg" />
        <span class="paper-back" aria-hidden="true" />
        <img class="photo" :src="thumbUrl" alt="" draggable="false" />
        <span class="layer front" v-html="frontThumbSvg" />
      </template>
      <template v-else>
        <span class="layer plain" v-html="folderSvg" />
      </template>
      <!-- 拖拽并入覆盖（C4b）：盖住整个缩略图区域的方形覆盖 -->
      <span v-if="dropInto" class="drop-cover" aria-hidden="true" />
    </div>
    <div class="texts">
      <span class="title">{{ folder.name }}</span>
      <span class="meta">{{ meta }}</span>
    </div>
    <!-- 同级排序间隙竖块（C4 混排护栏：文件夹只与文件夹排序；宽 16 = 卡片间距档） -->
    <span v-if="gapSide" class="drop-gap" :class="gapSide" aria-hidden="true" />
  </div>
</template>

<style scoped>
/* 总容器：间距 4；宽度由 FolderStrip 弹性控制（基准 189） */
.folder-card {
  position: relative; /* 拖拽间隙竖块挂点（C4） */
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  cursor: default;
}
/* 拖拽并入覆盖（C4b，2026-07-28 定案）：--brand-1-12 填充 + inset 1px --brand-1，
   盖住整个缩略图区域（含分层 SVG，须在 ::after 边框层之上） */
.drop-cover {
  position: absolute;
  inset: 0;
  z-index: 2;
  border-radius: var(--radius-8);
  background: var(--brand-1-12);
  box-shadow: inset 0 0 0 1px var(--brand-1);
  pointer-events: none;
}
/* 同级排序间隙竖块（C4）：宽 = 卡片间距 16 / 高随卡片整体，圆角竖向矩形色块 */
.drop-gap {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 16px;
  border-radius: var(--radius-4);
  background: var(--brand-1-12);
  box-shadow: inset 0 0 0 1px var(--brand-1);
  pointer-events: none;
}
.drop-gap.left {
  left: -16px;
}
.drop-gap.right {
  right: -16px;
}
/* 图标容器：padding 0 / 圆角 8（= SVG 外形圆角，裁切弧与 SVG 弧重合，常态不截断）；
   边框由 ::after 绘制（不占布局，圆角同为 8——hover/选中框外缘与 SVG 弧重合，
   框盖在图形上 2px，与素材卡片边框盖图片同一机制） */
.icon-wrap {
  position: relative;
  width: 100%;
  border-radius: var(--radius-8);
  overflow: hidden;
  line-height: 0;
}
.icon-wrap::after {
  content: "";
  position: absolute;
  inset: 0;
  border: 2px solid transparent;
  border-radius: var(--radius-8);
  pointer-events: none;
  transition: border-color 0.12s ease;
}
.plain {
  display: block;
  line-height: 0;
}
/* 有缩略图态：181×150 分层定位——背面整卡；装饰纸张保留前后两层：后层纸（图层3，16,25 / 149×109）
   只露上沿，缩略图放前层纸（图层2，8,29 / 165×113）；有缩略图层1 正面下沿（高 60，盖住照片下段；
   06_卡片 02 图层1 top 59.49% ≈ 60/150） */
.icon-wrap.thumb {
  aspect-ratio: 181 / 150;
}
.icon-wrap.thumb .layer {
  position: absolute;
  inset: 0;
  line-height: 0;
  color: var(--border-1); /* 分层 SVG 线条为 currentColor 15%，对齐文件夹结构色 */
}
.icon-wrap.thumb .layer :deep(svg) {
  width: 100%;
  height: 100%;
  display: block;
}
/* 分层 SVG 线条原为 currentColor 15%（图标批处理口径），文件夹卡片须与无缩略图态一致：
   图层描边强制回 border-1 实线（2026-07-25 用户定） */
.icon-wrap.thumb .layer :deep(path[stroke]) {
  stroke: var(--border-1);
  stroke-opacity: 1;
}
.icon-wrap.thumb .front {
  top: auto;
  height: 40%;
}
/* 后层纸（图层3）：缩略图不放在这层，只作装饰纸从后层露出 */
.paper-back {
  position: absolute;
  left: 8.84%; /* 16/181 */
  top: 16.67%; /* 25/150 */
  width: 82.32%; /* 149/181 */
  height: 72.67%; /* 109/150 */
  background: var(--font-icon-3);
  border-radius: var(--radius-4);
  box-shadow: inset 0 0 0 1px var(--font-icon-20);
}
/* 缩略图 = 前层纸（图层2）：contain 不裁切，文件纸底色托透明通道 */
.photo {
  position: absolute;
  left: 4.42%; /* 8/181 */
  top: 19.33%; /* 29/150 */
  width: 91.16%; /* 165/181 */
  height: 75.33%; /* 113/150 */
  object-fit: contain; /* 与网格缩略图同口径：自适应缩放不裁切（2026-07-25 用户定） */
  background: var(--folder-card-file); /* 保留前层文件纸底色：透明通道不能漏成黑底 */
  border-radius: var(--radius-4);
  box-shadow: inset 0 0 0 1px var(--font-icon-3);
  user-select: none;
}
/* hover 2px border-1 / 选中 2px 品牌色 */
.folder-card:hover .icon-wrap::after {
  border-color: var(--border-1);
}
.folder-card.selected .icon-wrap::after {
  border-color: var(--brand-1);
}
.icon-wrap :deep(svg) {
  width: 100%;
  height: auto;
  display: block;
}
/* 文字组：间距 2 / 居中 / 高度按内容自适应（单行 40，两行 60）——文件夹图标比例固定，
   无需像素材卡片那样为布局预留定高（2026-07-23 用户定：区域紧凑）；标题最多两行截断 */
.texts {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
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
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
