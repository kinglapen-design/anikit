<script setup lang="ts">
// 文件夹选择弹窗的递归节点（6.3）：TreeItem + 子层缩进，点击选中 / 箭头展开折叠
import TreeItem from "./TreeItem.vue";
import type { FolderNode } from "../stores/content";

const props = defineProps<{
  node: FolderNode;
  depth: number;
  picked: string | null;
  collapsed: Set<string>;
}>();
const emit = defineEmits<{ pick: [id: string]; toggle: [id: string] }>();
</script>

<template>
  <div class="pick-node">
    <TreeItem
      :icon="!collapsed.has(node.id) ? 'folder-open' : 'folder-closed'"
      :label="node.name"
      :count="node.count"
      :depth="depth"
      :expandable="node.children.length > 0"
      :open="!collapsed.has(node.id)"
      :selected="picked === node.id"
      @toggle="emit('toggle', node.id)"
      @select="emit('pick', node.id)"
    />
    <div v-if="!collapsed.has(node.id) && node.children.length > 0" class="kids">
      <FolderPickNode
        v-for="c in node.children"
        :key="c.id"
        :node="c"
        :depth="depth + 1"
        :picked="picked"
        :collapsed="collapsed"
        @pick="emit('pick', $event)"
        @toggle="emit('toggle', $event)"
      />
    </div>
  </div>
</template>

<style scoped>
.pick-node {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.kids {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  padding-left: 20px;
}
</style>
