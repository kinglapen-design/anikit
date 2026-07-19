<script setup lang="ts">
// 文件夹树（递归）：展开 / 折叠状态每层自管，选中态写入 content store（阶段 1 仅样式）。
// 缩进线（03_UI 布局/02_左侧边栏 04）：竖线从父项展开图标中心向下、止于最后子项中心；
// 叶子 item 画 4px 横线，有子项的跳过绘制。
import { ref } from "vue";
import TreeItem from "./TreeItem.vue";
import { useContentStore } from "../stores/content";
import type { FakeFolder } from "../lib/fakeData";

withDefaults(defineProps<{ nodes: FakeFolder[]; depth?: number }>(), { depth: 0 });
const content = useContentStore();

const openSet = ref(new Set<number>());
function toggle(id: number): void {
  const next = new Set(openSet.value);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  openSet.value = next;
}
</script>

<template>
  <div v-for="node in nodes" :key="node.id">
    <TreeItem
      :icon="openSet.has(node.id) ? 'folder-open' : 'folder-closed'"
      :label="node.name"
      :count="node.count"
      :depth="depth"
      :expandable="node.children.length > 0"
      :open="openSet.has(node.id)"
      :selected="content.sidebarSelected === `folder:${node.id}`"
      @toggle="toggle(node.id)"
      @select="content.sidebarSelected = `folder:${node.id}`"
    />
    <div
      v-if="openSet.has(node.id) && node.children.length > 0"
      class="ft-children"
      :style="{ '--guide-x': `${depth * 14 + 5}px` }"
    >
      <FolderTree :nodes="node.children" :depth="depth + 1" />
    </div>
  </div>
</template>

<style scoped>
.ft-children {
  position: relative;
}
/* 竖线：1px，从父项展开图标中心（每层 14px 缩进的中心线）向下，止于最后子项中心 */
.ft-children::before {
  content: "";
  position: absolute;
  left: var(--guide-x);
  top: 0;
  bottom: 14px;
  width: 1px;
  background: var(--c4-7);
}
/* 横线：叶子 item 从竖线向右 4px；有子项的 item 跳过绘制（--guide-x 按最近祖先层取值） */
.ft-children :deep(.tree-item.leaf)::after {
  content: "";
  position: absolute;
  left: var(--guide-x);
  top: 50%;
  width: 4px;
  height: 1px;
  background: var(--c4-7);
}
</style>
