<script setup lang="ts">
// 移动面板递归节点（3.4；2026-09-08 换代 = 布局/模态窗口/移动 稿，旧 29 稿 TreeItem 口径退役）：
// item = items/TreeRowItem（26 高 / padding 4 6 / 圆角 6 / hover 半透明-3——与侧栏树同族）；
// 行级 hover 三分判定（上/下 1/3 = 移为目标 item 前/后兄弟，中 1/3 = 并入为子项）——
// 落点视觉 = TreeRowItem dropZone（插入线 2px 品牌/常态 / 并入 品牌/半透明填充 + 2px 描边，速查 §20）；
// forceOpen = 搜索模式强制展开（折叠态在面板本地，默认全展开）；
// 子层展开/收起动画 = lib/expandAnim 150ms（树节点展开范例，FolderTree .ft-children 同款）。
// 层级缩进 = TreeRowItem padding-left 6 + depth×20（稿未画层级沿用旧档）；不画缩进竖线（稿无）。
import { computed } from "vue";
import TreeRowItem from "./items/TreeRowItem.vue";
import { expandEnter, expandLeave } from "../lib/expandAnim";
import { toNewTint } from "../lib/customColors";
import { useContentStore, type FolderNode } from "../stores/content";

export type MoveZone = "before" | "into" | "after";

const props = defineProps<{
  node: FolderNode;
  depth: number;
  collapsed: Set<string>;
  forceOpen: boolean;
  hint: { id: string; zone: MoveZone } | null;
  inheritTint?: string;
  // —— 目的树选择器变体（2026-10-03 库 ↔ 工作空间互通 DestTreeDialog 复用；缺省全 = 移动面板原行为）——
  intoOnly?: boolean; // 无兄弟排序落点：行级 hover 恒为并入（不画上/下插入线）
  expandable?: (node: FolderNode) => boolean; // 展开箭头判定（缺省 = children 非空；ws 懒加载 = hasSubDir）
  countOf?: (node: FolderNode) => number; // 数量取值（缺省 = node.count + content.directCounts；ws 无库计数 = 0 不渲染）
}>();
const emit = defineEmits<{
  toggle: [id: string];
  hover: [id: string, zone: MoveZone];
  leave: [id: string];
  pick: [node: FolderNode];
}>();

const content = useContentStore();

const isOpen = computed(() => props.forceOpen || !props.collapsed.has(props.node.id));
// 自定义色（同 FolderTree nodeTint）：本节点色 ?? 祖先链最近色；展示层经 toNewTint 映射新九色名
const tint = computed(() => props.node.color ?? props.inheritTint);

// 三分判定：行包裹层即热区（子层递归在 .mv-kids 兄弟容器，不冒泡进本行）；
// intoOnly 变体（目的树选择器）无兄弟落点——恒为并入
function onRowMove(e: MouseEvent): void {
  if (props.intoOnly) {
    emit("hover", props.node.id, "into");
    return;
  }
  const row = e.currentTarget as HTMLElement;
  const r = row.getBoundingClientRect();
  const ratio = (e.clientY - r.top) / r.height;
  emit("hover", props.node.id, ratio < 1 / 3 ? "before" : ratio > 2 / 3 ? "after" : "into");
}
</script>

<template>
  <div class="mv-node">
    <div class="mv-row" @mousemove="onRowMove" @mouseleave="emit('leave', node.id)">
      <TreeRowItem
        :kind="node.kind === 'archive' ? 'archive' : 'folder'"
        :icon="node.kind === 'archive' ? '' : (node.icon ?? '')"
        :name="node.name"
        :count="countOf ? countOf(node) : node.count"
        :direct-count="countOf ? countOf(node) : (content.directCounts.get(node.id) ?? 0)"
        :expanded="isOpen"
        :expandable="expandable ? expandable(node) : node.children.length > 0"
        :tint="toNewTint(tint)"
        :depth="depth"
        :drop-zone="hint?.id === node.id ? hint.zone : null"
        @toggle="emit('toggle', node.id)"
        @click="emit('pick', node)"
      />
    </div>
    <Transition :css="false" @enter="expandEnter" @leave="expandLeave">
      <div v-if="isOpen && node.children.length > 0" class="mv-kids">
        <MoveNode
          v-for="c in node.children"
          :key="c.id"
          :node="c"
          :depth="depth + 1"
          :collapsed="collapsed"
          :force-open="forceOpen"
          :hint="hint"
          :inherit-tint="tint"
          :into-only="intoOnly"
          :expandable="expandable"
          :count-of="countOf"
          @toggle="emit('toggle', $event)"
          @hover="(id, zone) => emit('hover', id, zone)"
          @leave="emit('leave', $event)"
          @pick="emit('pick', $event)"
        />
      </div>
    </Transition>
  </div>
</template>

<style scoped>
/* 节点与展开子层间距 2（移动稿 树形容器 gap 2，与侧栏树同档） */
.mv-node,
.mv-kids {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-2);
}
</style>
