<script setup lang="ts">
// 区块 · 归属档案（3.2 一轮，只读）：资产 Placement 所在 folder 链上 kind = archive 者反查逐行（名称）；
// 无归属 / 档案形态 = 空内容（标题照常，2026-08-15 定案：无内容区块不隐藏）。
import { computed, ref } from "vue";
import SectionHeader from "../SectionHeader.vue";
import AppIcon from "../AppIcon.vue";
import type { ArchiveRef, DetailTarget } from "../../lib/detailSections";

const props = defineProps<{ target: DetailTarget; title: string }>();
const open = ref(true);

const items = computed<ArchiveRef[]>(() =>
  props.target.kind === "file" ? props.target.detail.archives : [],
);
</script>

<template>
  <section class="section-archive detail-block">
    <SectionHeader :title="title" grip v-model:open="open" />
    <div v-if="open" class="body">
      <div v-for="a in items" :key="a.folderId" class="archive-item">
        <AppIcon name="archive" class="deco" />
        <span class="label">{{ a.name }}</span>
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
/* item：同链接区块口径（C4 底 / hover C5 / 圆角 8 / inset 1px border-1） */
.archive-item {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  padding: var(--space-4) var(--space-8);
  border-radius: var(--radius-8);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--border-1);
  color: var(--text-highlight);
}
.archive-item:hover {
  background: var(--c5);
}
.label {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.deco {
  color: var(--text-normal);
  flex: none;
}
</style>
