<script setup lang="ts">
// 区块 · 链接区块（3.2 一轮，只读）：该资产全部 Placement 反查逐行——
// 显示名（图标 + 名称）+ 位置副文本（区块名 / path 三段 ` / ` 连接）；
// item 样式 = 旧关联文件夹 item 口径（C4 底 / hover C5 / 圆角 8 / inset 1px border-1），无删除钮（只读）。
// 档案形态 / 无 Placement = 空内容（标题照常，2026-08-15 定案：无内容区块不隐藏）。
import { computed, ref } from "vue";
import SectionHeader from "../SectionHeader.vue";
import AppIcon from "../AppIcon.vue";
import type { DetailTarget, PlacementLink } from "../../lib/detailSections";

const props = defineProps<{ target: DetailTarget; title: string }>();
const open = ref(true);

const items = computed<PlacementLink[]>(() =>
  props.target.kind === "file" ? props.target.detail.placements : [],
);

function location(p: PlacementLink): string {
  return `${p.blockName} / ${p.path.join(" / ")}`;
}
</script>

<template>
  <section class="section-links detail-block">
    <SectionHeader :title="title" grip v-model:open="open" />
    <div v-if="open" class="body">
      <div v-for="(p, i) in items" :key="`${p.folderId ?? 'root'}|${p.name}|${i}`" class="link-item">
        <AppIcon name="folder-closed" class="deco" />
        <span class="label">{{ p.name }}</span>
        <span class="spacer" />
        <span class="sub" v-tip="location(p)">{{ location(p) }}</span>
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
/* item：一栏 / 内边距 4 4 4 8 / 圆角 8 / C4 / 1px border-1 / 间距 6（旧关联文件夹口径） */
.link-item {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  padding: var(--space-4) var(--space-8);
  border-radius: var(--radius-8);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--border-1);
  color: var(--text-highlight);
}
.link-item:hover {
  background: var(--c5);
}
.label {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.spacer {
  flex: 1;
  min-width: var(--space-8);
}
/* 位置副文本：区块名 / 路径三段，标签档，单行省略 */
.sub {
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--text-label);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 50%;
  flex: none;
}
/* 装饰图标：常态档（80%，同文件树 item 口径） */
.deco {
  color: var(--text-normal);
  flex: none;
}
</style>
