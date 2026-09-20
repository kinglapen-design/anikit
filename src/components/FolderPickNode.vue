<script setup lang="ts">
// 文件夹选择弹窗的递归节点（6.3）：TreeItem + 子层缩进（容器 padding-left 承担，item 无层级 padding），
// 点击选中 / 箭头展开折叠
import TreeItem from "./TreeItem.vue";
import type { FolderNode } from "../stores/content";

defineProps<{
  node: FolderNode;
  depth: number; // 层级（内容缩进：TreeItem padding-left；底色块满宽不缩进）
  picked: string | null;
  collapsed: Set<string>;
}>();
const emit = defineEmits<{ pick: [id: string]; toggle: [id: string] }>();
</script>

<template>
  <div class="pick-node">
    <TreeItem
      :icon="node.icon ?? (!collapsed.has(node.id) ? 'folder-open' : 'folder-closed')"
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
  gap: var(--n-space-4);
}
/* 子层容器不缩进：内容缩进由 TreeItem padding-left 承担（底色满宽，与主树同口径） */
.kids {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-4);
}
</style>
