<script setup lang="ts">
// 列表行（02_通用组件/02_item 10）：缩略图（可随滑块缩放）+ 名称 + 尺寸 / 格式 / 文件大小 / 修改日期。
// C3 / C1 交替行底色；1px border-2（inset）/ 圆角 8 / 内边距 6 8。
// 文件大小 / 修改日期仍是假占位：阶段 4 排序功能接真实值（后端 PlacementView 待补 size / modifiedAt）。
import { computed } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { Box } from "../lib/layouts";
import type { PlacementView } from "../stores/content";
import { useUiStore } from "../stores/ui";

const props = withDefaults(
  defineProps<{ asset: PlacementView; box: Box; odd?: boolean; selected?: boolean }>(),
  { odd: false, selected: false },
);
const ui = useUiStore();

// 缩略图随滑块缩放（80×45 基准 × 行高 / 220），等高不等宽限定容器内不裁剪
const thumbW = computed(() => Math.max(30, Math.round((80 * ui.targetRowHeight) / 220)));
const thumbH = computed(() => Math.max(17, Math.round((45 * ui.targetRowHeight) / 220)));
const thumbUrl = computed(() => (props.asset.thumbPath ? convertFileSrc(props.asset.thumbPath) : null));
const size = computed(() =>
  props.asset.w != null && props.asset.h != null
    ? `${props.asset.w} x ${props.asset.h}`
    : props.asset.ext.toUpperCase(),
);
// 序列帧（2026-07-22 用户定）：格式列显示"序列帧"，名称后缀显示为 .序列帧（仅显示层，数据不动）；
// 动图（5.1）同带 frameCount，按 animated 区分——格式列显示扩展名大写
const isSeq = computed(() => props.asset.frameCount != null && !props.asset.animated);
const format = computed(() => (isSeq.value ? "序列帧" : props.asset.ext.toUpperCase()));
const displayName = computed(() =>
  isSeq.value ? `${props.asset.name.replace(/\.[^.]+$/, "")}.序列帧` : props.asset.name,
);
const emit = defineEmits<{ select: [] }>();
</script>

<template>
  <div
    class="list-row"
    :class="{ odd, selected }"
    :style="{ left: `${box.left}px`, top: `${box.top}px`, width: `${box.width}px`, height: `${box.height}px` }"
    @click="emit('select')"
  >
    <div class="main">
      <img
        v-if="thumbUrl"
        class="thumb"
        :class="{ icon: asset.thumbKind === 'formatIcon' }"
        :style="{ width: `${thumbW}px`, height: `${thumbH}px` }"
        :src="thumbUrl"
        :alt="asset.name"
        decoding="async"
        draggable="false"
      />
      <div v-else class="thumb" :style="{ width: `${thumbW}px`, height: `${thumbH}px` }" />
      <span class="name">{{ displayName }}</span>
    </div>
    <div class="details">
      <span class="col size">{{ size }}</span>
      <span class="col format">{{ format }}</span>
      <span class="col filesize">123 MB</span>
      <span class="col date">2026/07/12 14:26</span>
    </div>
  </div>
</template>

<style scoped>
.list-row {
  position: absolute;
  display: flex;
  align-items: center;
  justify-content: space-between;
  /* 上下 padding 8（2026-07-24 用户定，原 6） */
  padding: var(--space-8);
  border-radius: var(--radius-8);
  background: var(--c1);
  box-shadow: inset 0 0 0 1px var(--border-2);
  cursor: default;
}
/* C3 / C1 交替行底色 */
.list-row.odd {
  background: var(--c3);
}
/* hover / 选中与素材卡片一致：2px 内阴影描边不用 fill；悬停 border-1，选中品牌色-1 */
.list-row:hover {
  box-shadow: inset 0 0 0 2px var(--border-1);
}
.list-row.selected {
  box-shadow: inset 0 0 0 2px var(--brand-1);
}
/* 标题容器：缩略图 + 名称，间距 8 */
.main {
  display: flex;
  align-items: center;
  gap: var(--space-8);
  flex: 1;
  min-width: 0;
}
.thumb {
  flex: none;
  display: block;
  border-radius: var(--radius-4);
  box-shadow: inset 0 0 0 1px var(--border-2);
  /* contain 完整显示不裁切（行高固定 cover 必裁，2026-07-24 用户定；格式图标同规则） */
  object-fit: contain;
  /* 无缩略图占位：中间面板底色 C1（阶段3修改——不要 font-icon-4 灰块） */
  background: var(--c1);
}
.name {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--font-icon-1);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
/* 详情容器：尺寸 / 格式 / 文件大小 / 修改日期，正文 font-icon-3，固定列宽与列头对齐 */
.details {
  display: flex;
  align-items: center;
  gap: var(--space-16);
  flex: none;
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--font-icon-3);
}
.col {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.size {
  width: 90px;
}
.format {
  width: 50px;
}
.filesize {
  width: 70px;
}
.date {
  width: 130px;
}
</style>
