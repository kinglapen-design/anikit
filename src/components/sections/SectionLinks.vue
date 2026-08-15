<script setup lang="ts">
// 区块 · 链接区块（3.2 一轮，只读；figma 数据 item §2 链接区块/副本，2026-08-15 走查贴稿）：
// 两行卡（C4 / padding 8 8 8 12 / 圆角 6 / gap 8 / 1px 暗-悬停，hover C5）——
// 行1 = 文件夹图标 18 + 路径（区块名 14 常态 / 根父级 标签色 / 所在目录 极淡，`/` 分隔，
// 分隔符色随前段）+ ×（hover 从右往左滑入平时不占位，DetailNotes .del 同款；点击业务 6.2，
// disabled 占位）；行2 = 文件显示名 12/17 标签色。
// 档案形态 / 无 Placement = 空内容（标题照常，2026-08-15 定案：无内容区块不隐藏）。
import { computed, ref } from "vue";
import SectionHeader from "../SectionHeader.vue";
import AppIcon from "../AppIcon.vue";
import IconButton from "../IconButton.vue";
import type { DetailTarget, PlacementLink } from "../../lib/detailSections";

const props = defineProps<{ target: DetailTarget; title: string }>();
const open = ref(true);

const items = computed<PlacementLink[]>(() =>
  props.target.kind === "file" ? props.target.detail.placements : [],
);

// 路径段色档（§2：根父级 = 标签色，所在目录 = 极淡；多于两段时中间段同标签色）
function segClass(p: PlacementLink, j: number): string {
  return j === p.path.length - 1 ? "faint" : "label";
}
</script>

<template>
  <section class="section-links detail-block">
    <SectionHeader :title="title" grip v-model:open="open" />
    <!-- 空数据不渲染容器（2026-08-15 走查：父级 detail-block gap 会让空 .body 也撑出 8px，
         展开/折叠必须零高度变化） -->
    <div v-if="open && items.length > 0" class="body">
      <div v-for="(p, i) in items" :key="`${p.folderId ?? 'root'}|${p.name}|${i}`" class="link-item">
        <div class="row1">
          <AppIcon name="folder-closed" :size="18" class="deco" />
          <span class="seg block">{{ p.blockName }}</span>
          <template v-for="(seg, j) in p.path" :key="j">
            <span class="sep" :class="j === 0 ? 'normal' : 'label'">/</span>
            <span class="seg" :class="segClass(p, j)">{{ seg }}</span>
          </template>
          <span class="spacer" />
          <IconButton name="clear" small disabled title="移除链接" class="del" />
        </div>
        <!-- 行2（稿：名称 + 链接状态徽章，justify-between / padding 0 4 0 0 / gap 10）：
             原始文件徽章 15px 品牌色，origin=true 才出；链接 Placement 不出 -->
        <div class="row2">
          <span class="file-name">{{ p.name }}</span>
          <AppIcon v-if="p.origin" name="original-file-badge" :size="15" class="origin-badge" />
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.section-links {
  display: flex;
  flex-direction: column;
}
/* item 容器：间距 8 */
.body {
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
}
/* 两行卡（§2）：C4 / padding 8 8 8 12 / gap 8 / 圆角 6 / 1px 暗-悬停；hover C5 */
.link-item {
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
  padding: var(--space-8) var(--space-8) var(--space-8) var(--space-12);
  border-radius: var(--radius-6);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--line-hover);
}
.link-item:hover {
  background: var(--c5);
}
/* 行1：图标 18 + 路径三段 + ×（高 20，间距 4） */
.row1 {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  height: 20px;
}
.deco {
  color: var(--text-normal);
  flex: none;
}
.seg,
.sep {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  white-space: nowrap;
}
.seg.block,
.sep.normal {
  color: var(--text-normal);
}
.seg.label,
.sep.label {
  color: var(--text-label);
}
.seg.faint {
  color: var(--text-faint);
}
.seg {
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
}
.seg.block {
  flex: none;
}
.spacer {
  flex: 1;
  min-width: var(--space-4);
}
/* 行2（稿：名称 + 链接状态徽章）：justify-between / padding 0 4 0 0 / gap 10；
   文件显示名 12/17 标签色单行省略；原始文件徽章 15px（品牌色，SVG 自带） */
.row2 {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding-right: var(--space-4);
}
.file-name {
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--text-label);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
}
.origin-badge {
  flex: none;
}
/* ×：常态不占位隐藏（宽度 0），悬停 item 展开 24px 露出（DetailNotes .del 同款机制）；
   选择器带 .link-item 提权，覆盖 IconButton.small 的固定 width 24 */
.link-item .del {
  width: 0;
  padding-left: 0;
  padding-right: 0;
  overflow: hidden;
  opacity: 0;
  color: var(--text-label);
  flex: none;
  transition:
    width 0.12s ease,
    opacity 0.12s ease,
    color 0.12s ease;
}
.link-item:hover .del {
  width: 24px;
  opacity: 1;
}
</style>
