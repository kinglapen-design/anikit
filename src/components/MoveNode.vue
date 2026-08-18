<script setup lang="ts">
// 移动面板递归节点（3.4，29 稿）：TreeItem 复用（icon/variant/tint/count 同 FolderTree 口径，
// tint 祖先链传递）；行级 hover 三分判定（上/下 1/3 = 移为目标 item 前/后兄弟，中 1/3 = 并入为子项）
// —— 落点视觉 = TreeItem dropZoneOverride（插入线 / 并入填充与主窗口拖拽同款）；
// forceOpen = 搜索模式强制展开（折叠态在面板本地，默认全展开）。
import { computed } from "vue";
import TreeItem from "./TreeItem.vue";
import type { FolderNode } from "../stores/content";

export type MoveZone = "before" | "into" | "after";

const props = defineProps<{
  node: FolderNode;
  depth: number;
  parentId: string | null;
  collapsed: Set<string>;
  forceOpen: boolean;
  hint: { id: string; zone: MoveZone } | null;
  inheritTint?: string;
}>();
const emit = defineEmits<{
  toggle: [id: string];
  hover: [id: string, zone: MoveZone];
  leave: [id: string];
  pick: [node: FolderNode];
}>();

const isOpen = computed(() => props.forceOpen || !props.collapsed.has(props.node.id));
// 自定义色（同 FolderTree nodeTint）：本节点色 ?? 祖先链最近色
const tint = computed(() => props.node.color ?? props.inheritTint);

// 三分判定：行包裹层即热区（子层递归在 .mv-kids 兄弟容器，不冒泡进本行）
function onRowMove(e: MouseEvent): void {
  const row = e.currentTarget as HTMLElement;
  const r = row.getBoundingClientRect();
  const ratio = (e.clientY - r.top) / r.height;
  emit("hover", props.node.id, ratio < 1 / 3 ? "before" : ratio > 2 / 3 ? "after" : "into");
}
</script>

<template>
  <div class="mv-node">
    <div class="mv-row" @mousemove="onRowMove" @mouseleave="emit('leave', node.id)">
      <TreeItem
        :icon="node.kind === 'archive' ? 'archive' : node.icon ?? (isOpen ? 'folder-open' : 'folder-closed')"
        :label="node.name"
        :count="node.count"
        :depth="depth"
        :variant="node.kind === 'archive' ? 'archive' : parentId === null ? 'root' : 'child'"
        :tint="tint"
        :expandable="node.children.length > 0"
        :open="isOpen"
        :drop-zone-override="hint?.id === node.id ? hint.zone : null"
        @toggle="emit('toggle', node.id)"
        @select="emit('pick', node)"
      />
    </div>
    <div v-if="isOpen && node.children.length > 0" class="mv-kids">
      <MoveNode
        v-for="c in node.children"
        :key="c.id"
        :node="c"
        :depth="depth + 1"
        :parent-id="node.id"
        :collapsed="collapsed"
        :force-open="forceOpen"
        :hint="hint"
        :inherit-tint="tint"
        @toggle="emit('toggle', $event)"
        @hover="(id, zone) => emit('hover', id, zone)"
        @leave="emit('leave', $event)"
        @pick="emit('pick', $event)"
      />
    </div>
  </div>
</template>

<style scoped>
.mv-node,
.mv-kids {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}
</style>
