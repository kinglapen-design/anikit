<script setup lang="ts">
// 区块 · 归属档案（3.2 一轮，只读；figma 数据 item §3，2026-08-15 走查贴稿）：
// item 高 36 / padding 8 8 8 12 / C4 / 圆角 6 / 1px 暗-悬停 / hover C5；
// 行 = 档案图标 18 + 名称 14 常态 ｜ 右侧 文件显示名 12 标签 + 链接徽章 15（linked-file-badge，
// 该档案内 Placement 为链接 origin=false 时出）+ ×（hover 从右往左滑入平时不占位，
// DetailNotes .del 同款；点击业务 6.2，disabled 占位）。
// 无归属 / 档案形态 = 空内容（标题照常，2026-08-15 定案：无内容区块不隐藏）。
import { computed, ref } from "vue";
import SectionHeader from "../SectionHeader.vue";
import AppIcon from "../AppIcon.vue";
import IconButton from "../IconButton.vue";
import type { ArchiveRef, DetailTarget, PlacementLink } from "../../lib/detailSections";

const props = defineProps<{ target: DetailTarget; title: string }>();
const open = ref(true);

const items = computed<ArchiveRef[]>(() =>
  props.target.kind === "file" ? props.target.detail.archives : [],
);
const placements = computed<PlacementLink[]>(() =>
  props.target.kind === "file" ? props.target.detail.placements : [],
);

// 档案内该文件的 Placement：直接所在优先，否则按路径段含档案名兜底（链上中间级）
function placementOf(a: ArchiveRef): PlacementLink | null {
  return (
    placements.value.find((p) => p.folderId === a.folderId) ??
    placements.value.find((p) => p.path.includes(a.name)) ??
    placements.value[0] ??
    null
  );
}
</script>

<template>
  <section class="section-archive detail-block">
    <SectionHeader :title="title" grip v-model:open="open" />
    <div v-if="open" class="body">
      <div v-for="a in items" :key="a.folderId" class="archive-item">
        <AppIcon name="archive" :size="18" class="deco" />
        <span class="label">{{ a.name }}</span>
        <span class="spacer" />
        <span v-if="placementOf(a)" class="file-name">{{ placementOf(a)!.name }}</span>
        <AppIcon
          v-if="placementOf(a) && !placementOf(a)!.origin"
          name="linked-file-badge"
          :size="15"
          class="link-badge"
        />
        <IconButton name="clear" small disabled title="移除" class="del" />
      </div>
    </div>
  </section>
</template>

<style scoped>
.section-archive {
  display: flex;
  flex-direction: column;
}
.body {
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
}
/* item（§3）：高 36 / padding 8 8 8 12 / gap 2 / C4 / 圆角 6 / 1px 暗-悬停；hover C5 */
.archive-item {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  height: 36px;
  box-sizing: border-box;
  padding: var(--space-8) var(--space-8) var(--space-8) var(--space-12);
  border-radius: var(--radius-6);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--line-hover);
}
.archive-item:hover {
  background: var(--c5);
}
/* 档案图标 18 + 名称 14 常态（图标与名称间距 6） */
.deco {
  color: var(--text-normal);
  flex: none;
  margin-right: var(--space-4);
}
.label {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-normal);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.spacer {
  flex: 1;
  min-width: var(--space-4);
}
/* 右侧：文件显示名 12/17 标签 + 链接徽章 15（与名称间距 6） */
.file-name {
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--text-label);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 40%;
  flex: none;
  margin-right: var(--space-2);
}
.link-badge {
  color: var(--text-label);
  flex: none;
  margin-right: var(--space-2);
}
/* ×：常态不占位隐藏（宽度 0），悬停 item 展开 24px 露出（DetailNotes .del 同款机制）；
   选择器带 .archive-item 提权，覆盖 IconButton.small 的固定 width 24 */
.archive-item .del {
  width: 0;
  padding-left: 0;
  padding-right: 0;
  overflow: hidden;
  opacity: 0;
  color: var(--text-label);
  transition:
    width 0.12s ease,
    opacity 0.12s ease,
    color 0.12s ease;
}
.archive-item:hover .del {
  width: 24px;
  opacity: 1;
}
</style>
