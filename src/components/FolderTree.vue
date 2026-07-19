<script setup lang="ts">
// 文件夹树（递归）：展开 / 折叠状态每层自管，选中态写入 content store（阶段 1 仅样式）。
// 缩进线由 TreeItem 在展开占位空间内自画（guide）：叶子行竖线 + 横线，有子项的行跳过，
// 最后一个子项 L 形收笔（02_左侧边栏 04）。
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
  <div v-for="(node, index) in nodes" :key="node.id">
    <TreeItem
      :icon="openSet.has(node.id) ? 'folder-open' : 'folder-closed'"
      :label="node.name"
      :count="node.count"
      :depth="depth"
      :expandable="node.children.length > 0"
      :open="openSet.has(node.id)"
      :selected="content.sidebarSelected === `folder:${node.id}`"
      :last="index === nodes.length - 1"
      guide
      @toggle="toggle(node.id)"
      @select="content.sidebarSelected = `folder:${node.id}`"
    />
    <div v-if="openSet.has(node.id) && node.children.length > 0">
      <FolderTree :nodes="node.children" :depth="depth + 1" />
    </div>
  </div>
</template>
