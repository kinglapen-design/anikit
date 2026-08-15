<script setup lang="ts">
// 区块 · 自定义数据（3.2 一轮，pinned 样式：无副标题、置顶观感；排序/显隐在 22 稿编辑面板）。
// 一轮只读：无任何输入/编辑交互。色板业务 6.1 未做——无颜色数据，不渲染色板行。
// 文件 = 缩略图（thumbKind === "formatIcon" contain；无图 = 浅色块 #C6C7CC + 扩展名，速查表 §6）
//   + 格式徽标（静态图不出，formatBadgeColor 口径）+ 名称 + 链接逐行 + 属性 chips + 描述；
// 档案 = 封面占位（头像三轮）+ 徽标芯片 + 名称 + 属性 chips + 描述列表（标题 + 内容）。
import { computed } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import AppTag from "../AppTag.vue";
import { displayName, formatBadgeColor, isSequence } from "../../lib/formatBadge";
import type { DetailTarget } from "../../lib/detailSections";
import { useUiStore } from "../../stores/ui";

const props = defineProps<{ target: DetailTarget; title?: string }>();
const ui = useUiStore();

const file = computed(() => (props.target.kind === "file" ? props.target : null));
const archive = computed(() => (props.target.kind === "archive" ? props.target : null));

const thumbUrl = computed(() => {
  const p = file.value?.placement.thumbPath;
  return p ? convertFileSrc(p) : null;
});
// 格式徽标底色：静态图 / 无扩展名映射返回 null = 不出徽标
const badgeColor = computed(() => {
  if (!file.value) return null;
  const p = file.value.placement;
  return formatBadgeColor(p.ext, isSequence(p), p.animated);
});
const name = computed(() => {
  if (file.value) return displayName(file.value.placement, ui.showExt);
  return archive.value?.node.name ?? "";
});
</script>

<template>
  <section class="section-custom detail-block pinned">
    <template v-if="file">
      <div class="thumb">
        <img
          v-if="thumbUrl"
          :class="{ icon: file.placement.thumbKind === 'formatIcon' }"
          :src="thumbUrl"
          alt=""
          draggable="false"
        />
        <span v-else class="ext">{{ file.placement.ext.toUpperCase() }}</span>
        <span v-if="badgeColor" class="fmt-badge" :style="{ background: badgeColor }">
          {{ file.placement.ext.toUpperCase() }}
        </span>
      </div>
      <span class="name" v-tip="name">{{ name }}</span>
      <div v-for="l in file.detail.links" :key="l" class="field">
        <span class="field-text">{{ l }}</span>
      </div>
      <div v-if="file.detail.tags.length > 0" class="chips">
        <AppTag v-for="t in file.detail.tags" :key="t" :label="t" readonly />
      </div>
      <div v-if="file.detail.note" class="field desc">
        <span class="field-text">{{ file.detail.note }}</span>
      </div>
    </template>
    <template v-else-if="archive">
      <!-- 封面占位：头像业务三轮，一轮浅色块占位（同无图口径，无扩展名文字） -->
      <div class="thumb">
        <span class="ext" />
        <span v-if="archive.data.badge" class="fmt-badge" :style="{ background: archive.data.badge.color }">
          {{ archive.data.badge.text }}
        </span>
      </div>
      <span class="name" v-tip="archive.node.name">{{ archive.node.name }}</span>
      <div v-if="archive.data.props.length > 0" class="chips">
        <AppTag v-for="t in archive.data.props" :key="t" :label="t" readonly />
      </div>
      <div v-for="d in archive.data.descriptions" :key="d.title" class="desc-item">
        <span class="desc-title">{{ d.title }}</span>
        <span class="desc-content">{{ d.content }}</span>
      </div>
    </template>
  </section>
</template>

<style scoped>
.section-custom {
  display: flex;
  flex-direction: column;
}
/* 缩略图：高 120 / 圆角 6 / 1px border-1（同旧颜色信息区块口径） */
.thumb {
  position: relative;
  width: 100%;
  height: 120px;
  border-radius: var(--radius-6);
  box-shadow: inset 0 0 0 1px var(--border-1);
  overflow: hidden;
  flex: none;
}
.thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}
/* 格式图标缩略图：方形 contain 不裁切（4.2 口径） */
.thumb img.icon {
  object-fit: contain;
}
/* 无图占位（速查表 §6）：浅色块 #C6C7CC + 扩展名文字 */
.thumb .ext {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #c6c7cc;
  font-size: var(--text-label-size);
  line-height: var(--text-label-line);
  font-weight: var(--text-title-weight);
  color: var(--c3);
}
/* 格式徽标 / 档案徽标芯片：左上 4px / 内边距 0 4 / 圆角 4 / 徽标档 10/14 700 常态色 */
.fmt-badge {
  position: absolute;
  top: 4px;
  left: 4px;
  max-width: calc(100% - 8px);
  padding: 0 var(--space-4);
  border-radius: var(--radius-4);
  font-size: var(--text-badge-size);
  line-height: var(--text-badge-line);
  font-weight: var(--text-badge-weight);
  color: var(--text-normal);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  pointer-events: none;
}
/* 名称：正文 14/20 高亮，单行省略 */
.name {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-highlight);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
/* 链接 / 描述行（03_输入框 01 形态只读）：内边距 6 8 / 圆角 8 / C4 / 1px border-1 */
.field {
  display: flex;
  align-items: center;
  min-height: 32px;
  box-sizing: border-box;
  padding: var(--space-6) var(--space-8);
  border-radius: var(--radius-8);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--border-1);
}
.field-text {
  min-width: 0;
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-highlight);
  overflow-wrap: anywhere;
}
/* 属性 chips：流式换行，间距 8 */
.chips {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-8);
}
/* 档案描述列表：标题（标签档）+ 内容（常态） */
.desc-item {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  padding: var(--space-6) var(--space-8);
  border-radius: var(--radius-8);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--border-1);
}
.desc-title {
  font-size: var(--text-label-size);
  line-height: var(--text-label-line);
  color: var(--text-label);
}
.desc-content {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-normal);
  overflow-wrap: anywhere;
  white-space: pre-wrap;
}
</style>
